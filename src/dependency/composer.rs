use std::cmp::max;
use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use futures::TryFutureExt;
use reqwest::Client;
use semver::{Version, VersionReq};
use serde_json::Value;
use tracing::{debug, warn};

use super::Dependency;

#[derive(Debug, Clone)]
pub struct ComposerDependency {
    name: String,
    version_req: VersionReq,
}

impl ComposerDependency {
    fn packagist_version_from_json(&self, json: &Value) -> Option<Version> {
        json["package"]["versions"]
            .as_object()
            .and_then(|versions_map| {
                versions_map
                    .keys()
                    .filter_map(|version_string| {
                        Version::parse(version_string.trim_start_matches('v'))
                            .ok()
                            .and_then(|v| if v.pre.is_empty() {
                                Some(v)
                            } else {
                                None
                            })
                    })
                    .fold(None, |a, b| match (a, b) {
                        (None, b) => Some(b),
                        (Some(a), b) => Some(max(a, b)),
                    })
            })
    }

    fn packagist_url(&self) -> String {
        format!("https://packagist.org/packages/{}.json", self.name)
    }
}

#[async_trait]
impl Dependency for ComposerDependency {
    fn to_check(composer_json_contents: &str, _path: &Path) -> Result<Vec<ComposerDependency>> {
        let composer_json = serde_json::from_str::<Value>(composer_json_contents)?;

        let requires = composer_json["require"].as_object();
        let require_devs = composer_json["require-dev"].as_object();

        Ok(requires
            .into_iter()
            .chain(require_devs.into_iter())
            .flat_map(|map| {
                map.iter().map(|(k, v)| match v {
                    Value::String(version) => Some((k.clone(), version.clone())),
                    _ => None,
                })
            })
            .filter_map(|opt| match opt {
                Some((name, version)) => match VersionReq::parse(version.trim_start_matches('v')) {
                    Ok(vr) => Some(ComposerDependency {
                        name: name,
                        version_req: vr,
                    }),
                    Err(err) => {
                        println!("{} ignored (could not parse {}: {:?})", name, version, err);
                        None
                    }
                },
                _ => None,
            })
            .filter(|c| c.name != "php" && !c.name.starts_with("ext-"))
            .collect())
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
            .get(self.packagist_url())
            .send()
            .and_then(|resp| resp.json::<Value>())
            .await
            .map_or_else(
                |e| {
                    warn!("Could not fetch {}: {e}", self.name);
                    None
                },
                |decoded| self.packagist_version_from_json(&decoded),
            )
    }
}
