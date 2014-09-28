use std::collections::TreeMap;

use semver::{Version, VersionReq};
use serialize::json;

use super::Dependency;

#[deriving(Decodable, Show)]
#[allow(non_snake_case)]
struct BowerJson {
    dependencies: Option<TreeMap<String, String>>,
    devDependencies: Option<TreeMap<String, String>>
}

#[deriving(Show, Clone)]
pub struct BowerDependency {
    name: String,
    version_req: VersionReq,
}

impl BowerDependency {
}

impl Dependency for BowerDependency {
    fn to_check(bower_json_contents: &str) -> Vec<BowerDependency> {
        let bower_json = match json::decode::<BowerJson>(bower_json_contents) {
            Ok(json) => json,
            Err(err) => fail!("Failed to parse bower.json: {}", err)
        };

        let requires = bower_json.dependencies.clone().unwrap_or(TreeMap::new());
        let require_devs = bower_json.devDependencies.clone().unwrap_or(TreeMap::new());

        requires.iter().chain(require_devs.iter()).filter_map(|(name, version)|
            match VersionReq::parse(version.as_slice().trim_left_chars('v')) {
                Ok(vr) => Some(BowerDependency { name: name.clone(), version_req: vr }),
                Err(err) => {
                    println!("{} ignored (could not parse {}: {})", name, version, err);
                    None
                }
            }
        ).collect::<Vec<BowerDependency>>()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn version_req(&self) -> Option<&VersionReq> {
        Some(&self.version_req)
    }

    fn registry_version(&self) -> Option<Version> {
        None // this needs git handling
    }
}
