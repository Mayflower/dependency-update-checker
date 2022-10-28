use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use semver::{Version, VersionReq};

//pub use self::bower::BowerDependency;
pub use self::cargo::CargoDependency;
pub use self::composer::ComposerDependency;
pub use self::npm::NpmDependency;
//pub use self::puppet::PuppetDependency;

//mod bower;
mod cargo;
mod composer;
mod npm;
//mod puppet;

#[async_trait]
pub trait Dependency : Clone + Send + 'static {
    fn to_check(dependency_file_contents: &str, path: &Path) -> Result<Vec<Self>>;
    fn name(&self) -> &str;
    fn version_req(&self) -> &VersionReq;
    async fn registry_version(&self) -> Option<Version>;
}
