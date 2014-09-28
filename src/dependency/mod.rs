use semver::{Version, VersionReq};

pub use self::composer::ComposerDependency;
pub use self::puppet::PuppetDependency;

mod composer;
mod puppet;

pub trait Dependency : Clone + Send {
    fn to_check(dependency_file_contents: &str) -> Vec<Self>;
    fn name(&self) -> &String;
    fn version_req(&self) -> Option<&VersionReq>;
    fn registry_version(&self) -> Option<Version>;
    fn clone_dep(&self) -> Box<Dependency + Send> {
        box self.clone() as Box<Dependency + Send>
    }
    fn to_dependency_box(self) -> Box<Dependency> {
        (box self) as Box<Dependency>
    }
}
