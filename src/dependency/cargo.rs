use std::path::Path;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use cargo::core::EitherManifest;
use cargo::util::toml::read_manifest;
use cargo::util::OptVersionReq;
use futures::TryFutureExt;
use reqwest::Client;
use semver::{Version, VersionReq};

use cargo::core::source::SourceId;
use cargo::util::config::Config;
use serde::Deserialize;
use tracing::{debug, warn};

use super::Dependency;

#[derive(Clone, Debug)]
pub struct CargoDependency {
    name: String,
    version_req: VersionReq,
}

#[derive(Debug, Deserialize)]
struct CratesIoCrate {
    max_stable_version: String,
}

#[derive(Debug, Deserialize)]
struct CratesIoResponse {
    #[serde(rename = "crate")]
    crate_: CratesIoCrate,
}

fn get_config_source_id() -> Result<(Config, SourceId)> {
    let config = Config::default()?;
    let source_id = SourceId::crates_io(&config)?;

    Ok((config, source_id))
}

impl CargoDependency {
    fn cargo_url(&self) -> String {
        format!("https://crates.io/api/v1/crates/{}", self.name)
    }
}

#[async_trait]
impl Dependency for CargoDependency {
    fn to_check(_cargo_toml_contents: &str, path: &Path) -> Result<Vec<CargoDependency>> {
        let (config, source_id) = get_config_source_id()?;

        Ok(
            match read_manifest(&path.canonicalize()?, source_id, &config) {
                Ok((EitherManifest::Real(manifest), _)) => manifest
                    .dependencies()
                    .iter()
                    .map(|dep| CargoDependency {
                        name: dep.package_name().to_string(),
                        version_req: match dep.version_req() {
                            OptVersionReq::Any => VersionReq::STAR,
                            OptVersionReq::Locked(_, vr) | OptVersionReq::Req(vr) => vr.clone(),
                        },
                    })
                    .collect(),
                _ => vec![],
            },
        )
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
            .get(self.cargo_url())
            .header("User-Agent", "dependency-checker")
            .send()
            .and_then(|resp| resp.json::<CratesIoResponse>())
            .await
            .map_err(|e| anyhow!(e))
            .and_then(|decoded| {
                Version::parse(&decoded.crate_.max_stable_version).map_err(|e| anyhow!(e))
            })
            .map_or_else(
                |e| {
                    warn!("Could not fetch {}: {e}", self.name);
                    None
                },
                |v| Some(v),
            )
    }
}
