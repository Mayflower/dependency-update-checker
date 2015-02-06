use puppetfile::Module as PuppetModule;
use puppetfile::Puppetfile;
use semver::{Version, VersionReq};

use super::Dependency;

#[derive(Clone, Debug)]
pub struct PuppetDependency {
    module: PuppetModule,
    forge_url: String,
}
impl Dependency for PuppetDependency {
    fn to_check(puppetfile_contents: &str) -> Vec<PuppetDependency> {
        match Puppetfile::parse(puppetfile_contents) {
            Ok(puppetfile) => {
                let forge_url = puppetfile.forge.clone();
                puppetfile.modules.into_iter()
                    .filter(|module| module.version().is_some())
                    .map(|module|
                        PuppetDependency {
                            module: module,
                            forge_url: forge_url.clone(),
                        }
                    ).collect()
            },
            Err(err) => {
                println!("Couldn't parse Puppetfile: {}", err);
                vec![]
            }
        }
    }

    fn name(&self) -> &String {
        &self.module.name
    }

    fn version_req(&self) -> Option<&VersionReq> {
        self.module.version()
    }

    fn registry_version(&self) -> Option<Version> {
        match self.module.forge_version(&self.forge_url) {
            Ok(version) => Some(version),
            Err(err) => {
                println!("{} ignored ({:?})", self.name(), err);
                None
            }
        }
    }
}
