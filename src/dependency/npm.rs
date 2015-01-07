use std::cmp::max;
use std::collections::HashMap;

use hyper::Client;
use semver::{Version, VersionReq};
use rustc_serialize::json::{self, Json};

use super::Dependency;


#[derive(RustcDecodable, Show)]
#[allow(non_snake_case)]
struct PackageJson {
    dependencies: Option<HashMap<String, String>>,
    devDependencies: Option<HashMap<String, String>>
}

#[derive(Show, Clone)]
pub struct NpmDependency {
    name: String,
    version_req: VersionReq
}

impl NpmDependency {
    fn npm_version_from_json(&self, json: &Json) -> Option<Version> {
        json.find("versions")
            .and_then(|versions_json| versions_json.as_object())
            .and_then(|versions_map| {
                versions_map.keys().map(|version_string| {
                    Version::parse(version_string[].trim_left_matches('v')).ok()
                }).fold(None, |a, b| {
                    match (a, b) {
                        (None, b@_) => b,
                        (a@Some(_), None) => a,
                        (Some(a), Some(b)) => Some(max(a, b))
                    }
                })
            })
    }

    fn npm_url(&self) -> String {
        format!("https://registry.npmjs.org/{}", self.name)
    }
}

impl Dependency for NpmDependency {
    fn to_check(package_json_contents: &str) -> Vec<NpmDependency> {
        let package_json = match json::decode::<PackageJson>(package_json_contents) {
            Ok(json) => json,
            Err(err) => panic!("Failed to parse bower.json: {}", err)
        };

        let requires = package_json.dependencies.clone().unwrap_or(HashMap::new());
        let require_devs = package_json.devDependencies.clone().unwrap_or(HashMap::new());

        requires.iter().chain(require_devs.iter()).filter_map(|(name, version)|
            match VersionReq::parse(version[].trim_left_matches('v')) {
                Ok(vr) => Some(NpmDependency { name: name.clone(), version_req: vr }),
                Err(err) => {
                    println!("{} ignored (could not parse {}: {})", name, version, err);
                    None
                }
            }
        ).collect::<Vec<NpmDependency>>()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn version_req(&self) -> Option<&VersionReq> {
        Some(&self.version_req)
    }

    fn registry_version(&self) -> Option<Version> {
        let mut response = Client::new().get(self.npm_url()[]).send().unwrap();
        let response_string = response.read_to_string().unwrap();
        match Json::from_str(response_string[]) {
            Ok(version_struct) => self.npm_version_from_json(&version_struct),
            Err(_)             => None
        }
    }
}
