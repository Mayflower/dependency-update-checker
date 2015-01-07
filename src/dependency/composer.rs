use std::cmp::max;
use std::collections::BTreeMap;

use hyper::Client;
use semver::{Version, VersionReq};
use rustc_serialize::json::{self, Json};

use super::Dependency;


#[derive(Show, Clone)]
pub struct ComposerDependency {
    name: String,
    version_req: VersionReq
}

impl ComposerDependency {
    fn packagist_version_from_json(&self, json: &Json) -> Option<Version> {
        json.find_path(&["package", "versions"])
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

    fn packagist_url(&self) -> String {
        format!("https://packagist.org/packages/{}.json", self.name)
    }
}

impl Dependency for ComposerDependency {
    fn to_check(composer_json_contents: &str) -> Vec<ComposerDependency> {
        let composer_json = Json::from_str(composer_json_contents).unwrap();
        let default_map = BTreeMap::new();

        let requires = composer_json.find("require").map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);
        let require_devs = composer_json.find("require-dev").map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);

        requires.iter().chain(require_devs.iter()).map(
            |(k, v)| {
                match v {
                    &json::Json::String(ref version) => Some((k.clone(), version.clone())),
                    _ => None
                }
            }
        ).filter_map(|opt| match opt {
            Some((ref name, ref version)) => {
                match VersionReq::parse(version[].trim_left_matches('v')) {
                    Ok(vr) => Some(ComposerDependency { name: name.clone(), version_req: vr }),
                    Err(err) => {
                        println!("{} ignored (could not parse {}: {})", name, version, err);
                        None
                    }
                }
            },
            _ => None
        }).collect()
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn version_req(&self) -> Option<&VersionReq> {
        Some(&self.version_req)
    }

    fn registry_version(&self) -> Option<Version> {
        let mut response = Client::new().get(self.packagist_url()[]).send().unwrap();
        let response_string = response.read_to_string().unwrap();
        match Json::from_str(response_string[]) {
            Ok(version_struct) => self.packagist_version_from_json(&version_struct),
            Err(_)             => None
        }
    }
}
