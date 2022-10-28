use std::cmp::max;
use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use futures::TryFutureExt;
use reqwest::Client;
use semver::{Version, VersionReq};
use serde::Deserialize;
use serde_json::Value;
use tracing::{debug, info, warn};

use super::Dependency;

#[derive(Deserialize, Debug)]
struct PackageJson {
    dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    dev_dependencies: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct NpmDependency {
    name: String,
    version_req: VersionReq,
}

impl NpmDependency {
    fn npm_version_from_json(&self, json: &serde_json::Value) -> Option<Version> {
        json["versions"].as_object().and_then(|versions_map| {
            versions_map
                .keys()
                .map(|version_string| Version::parse(version_string.trim_start_matches('v')).ok())
                .fold(None, |a, b| match (a, b) {
                    (None, b @ _) => b,
                    (a @ Some(_), None) => a,
                    (Some(a), Some(b)) => Some(max(a, b)),
                })
        })
    }

    fn npm_url(&self) -> String {
        format!("https://registry.npmjs.org/{}", self.name)
    }
}

#[async_trait]
impl Dependency for NpmDependency {
    fn to_check(package_json_contents: &str, _path: &Path) -> Result<Vec<NpmDependency>> {
        let package_json: PackageJson = serde_json::from_str(package_json_contents)?;

        let requires = package_json.dependencies.unwrap_or(HashMap::new());
        let require_devs = package_json.dev_dependencies.unwrap_or(HashMap::new());

        Ok(requires
            .into_iter()
            .chain(require_devs.into_iter())
            .filter_map(|(name, version)| {
                match VersionReq::parse(version.trim_start_matches('v')) {
                    Ok(vr) => Some(NpmDependency {
                        name: name,
                        version_req: vr,
                    }),
                    Err(err) => {
                        info!("{name} ignored (could not parse {version}: {err:?})");
                        None
                    }
                }
            })
            .collect::<Vec<_>>())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn version_req(&self) -> &VersionReq {
        &self.version_req
    }

    async fn registry_version(&self) -> Option<Version> {
        debug!("{} start", self.name);
        Client::new()
            .get(self.npm_url())
            .send()
            .and_then(|resp| resp.json::<Value>())
            .await
            .map_or_else(
                |e| {
                    warn!("Could not fetch {}: {e}", self.name);
                    None
                },
                |decoded| self.npm_version_from_json(&decoded),
            )
    }
}
