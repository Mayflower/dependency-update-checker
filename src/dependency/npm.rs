use std::cmp::max;
use std::collections::TreeMap;

use http::client::RequestWriter;
use http::method::Get;
use semver::{Version, VersionReq};
use serialize::json::{mod, Json};
use url::Url;

use super::Dependency;


#[deriving(Decodable, Show)]
#[allow(non_snake_case)]
struct PackageJson {
    dependencies: Option<TreeMap<String, String>>,
    devDependencies: Option<TreeMap<String, String>>
}

#[deriving(Show, Clone)]
pub struct NpmDependency {
    name: String,
    version_req: VersionReq
}

impl NpmDependency {
    fn npm_version_from_json(&self, json: &Json) -> Option<Version> {
        json.find(&"versions".to_string())
            .and_then(|versions_json| versions_json.as_object())
            .and_then(|versions_map| {
                versions_map.keys().map(|version_string| {
                    Version::parse(version_string.as_slice().trim_left_chars('v')).ok()
                }).fold(None, |a, b| {
                    match (a, b) {
                        (None, b@_) => b,
                        (a@Some(_), None) => a,
                        (Some(a), Some(b)) => Some(max(a, b))
                    }
                })
            })
    }

    fn npm_url(&self) -> Url {
        Url::parse(
            format!("https://registry.npmjs.org/{}", self.name).as_slice()
        ).unwrap()
    }
}

impl Dependency for NpmDependency {
    fn to_check(package_json_contents: &str) -> Vec<NpmDependency> {
        let package_json = match json::decode::<PackageJson>(package_json_contents) {
            Ok(json) => json,
            Err(err) => fail!("Failed to parse bower.json: {}", err)
        };

        let requires = package_json.dependencies.clone().unwrap_or(TreeMap::new());
        let require_devs = package_json.devDependencies.clone().unwrap_or(TreeMap::new());

        requires.iter().chain(require_devs.iter()).filter_map(|(name, version)|
            match VersionReq::parse(version.as_slice().trim_left_chars('v')) {
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
        let request: RequestWriter = RequestWriter::new(Get, self.npm_url()).unwrap();
        let mut response = match request.read_response() {
            Ok(response)           => response,
            Err((_request, error)) => fail!(":-( {}", error),
        };
        let response_string = response.read_to_string().unwrap();
        match json::from_str(response_string.as_slice()) {
            Ok(version_struct) => self.npm_version_from_json(&version_struct),
            Err(_)             => None
        }
    }
}


