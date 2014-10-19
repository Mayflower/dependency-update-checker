use std::cmp::max;
use std::collections::TreeMap;

use http::client::RequestWriter;
use http::method::Get;
use semver::{Version, VersionReq};
use serialize::json::{mod, Json};
use url::Url;

use super::Dependency;


#[deriving(Show, Clone)]
pub struct ComposerDependency {
    name: String,
    version_req: VersionReq
}

impl ComposerDependency {
    fn packagist_version_from_json(&self, json: &Json) -> Option<Version> {
        json.find_path(&[&"package".to_string(), &"versions".to_string()])
            .and_then(|versions_json| versions_json.as_object())
            .and_then(|versions_map| {
                versions_map.keys().map(|version_string| {
                    Version::parse(version_string[].trim_left_chars('v')).ok()
                }).fold(None, |a, b| {
                    match (a, b) {
                        (None, b@_) => b,
                        (a@Some(_), None) => a,
                        (Some(a), Some(b)) => Some(max(a, b))
                    }
                })
            })
    }

    fn packagist_url(&self) -> Url {
        Url::parse(
            format!("https://packagist.org/packages/{}.json", self.name)[]
        ).unwrap()
    }
}

impl Dependency for ComposerDependency {
    fn to_check(composer_json_contents: &str) -> Vec<ComposerDependency> {
        let composer_json = json::from_str(composer_json_contents).unwrap();
        let default_map = TreeMap::new();

        let requires = composer_json.find(&"require".to_string()).map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);
        let require_devs = composer_json.find(&"require-dev".to_string()).map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);

        requires.iter().chain(require_devs.iter()).map(
            |(k, v)| {
                match v {
                    &json::String(ref version) => Some((k.clone(), version.clone())),
                    _ => None
                }
            }
        ).filter_map(|opt| match opt {
            Some((ref name, ref version)) => {
                match VersionReq::parse(version[].trim_left_chars('v')) {
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
        let request: RequestWriter = RequestWriter::new(Get, self.packagist_url()).unwrap();
        let mut response = match request.read_response() {
            Ok(response)           => response,
            Err((_request, error)) => fail!(":-( {}", error),
        };
        let response_string = response.read_to_string().unwrap();
        match json::from_str(response_string[]) {
            Ok(version_struct) => self.packagist_version_from_json(&version_struct),
            Err(_)             => None
        }
    }
}
