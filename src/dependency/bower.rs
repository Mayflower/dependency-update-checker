use std::collections::HashMap;

use semver::{Version, VersionReq};
use rustc_serialize::json;

use super::Dependency;

#[derive(RustcDecodable, Show)]
#[allow(non_snake_case)]
struct BowerJson {
    dependencies: Option<HashMap<String, String>>,
    devDependencies: Option<HashMap<String, String>>
}

#[derive(Show, Clone)]
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
            Err(err) => panic!("Failed to parse bower.json: {}", err)
        };

        let requires = bower_json.dependencies.clone().unwrap_or(HashMap::new());
        let require_devs = bower_json.devDependencies.clone().unwrap_or(HashMap::new());

        requires.iter().chain(require_devs.iter()).filter_map(|(name, version)|
            match VersionReq::parse(version[].trim_left_matches('v')) {
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
