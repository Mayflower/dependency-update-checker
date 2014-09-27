extern crate http;
extern crate url;
extern crate semver;
extern crate serialize;

use std::os;
use std::str;
use std::cmp::max;
use std::io::File;
use std::sync::Future;
use std::collections::TreeMap;
use semver::{Version, VersionReq};
use http::client::RequestWriter;
use http::method::Get;
use serialize::json::{mod, Json};
use url::Url;

#[deriving(Show, Clone)]
struct Dependency {
    name    : String,
    version : VersionReq
}
impl Dependency {
    fn to_check_from_json(composer_json: Json) -> Vec<Dependency> {
        let default_map = TreeMap::new();
        let requires    = composer_json.find(&"require".to_string()).map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);
        let require_devs = composer_json.find(&"require-dev".to_string()).map(
            |r| r.as_object().unwrap()
        ).unwrap_or(&default_map);

        requires.iter().chain(require_devs.iter()).map(
            |(k, v)| {
                match v {
                    &json::String(ref version) => Some((k.clone(), version.clone())),
                    _                          => None
                }
            }
        ).filter_map(|opt| match opt {
            Some((ref name, ref version)) => {
                match VersionReq::parse(version.as_slice()) {
                    Ok(vr) => Some(Dependency { name: name.clone(), version: vr }),
                    Err(err) => {
                        println!("{} ignored (could not parse {}: {})", name, version, err);
                        None
                    }
                }
            },
            _ => None
        }).collect()
    }

    fn packagist_version(&self) -> Option<Version> {
        let request: RequestWriter = RequestWriter::new(Get, self.packagist_url()).unwrap();
        let mut response = match request.read_response() {
            Ok(response)           => response,
            Err((_request, error)) => fail!(":-( {}", error),
        };
        let response_string = response.read_to_string().unwrap();
        match json::from_str(response_string.as_slice()) {
            Ok(version_struct) => self.packagist_version_from_json(&version_struct),
            Err(_)             => None
        }
    }

    fn packagist_version_from_json(&self, json: &Json) -> Option<Version> {
        json.find_path(&[&"package".to_string(), &"versions".to_string()])
            .and_then(|versions_json| versions_json.as_object())
            .and_then(|versions_map| {
                versions_map.keys().map(
                    |version_string| Version::parse(version_string.as_slice()).ok()
                ).fold(None, |a, b| {
                    match (a, b) {
                        (None, b@_) => b,
                        (a@Some(_), None) => a,
                        (Some(a), Some(b)) => Some(max(a, b))
                    }
                })
            })
    }

    fn packagist_url(&self) -> Url {
        Url::parse(format!("https://packagist.org/packages/{}.json", self.name).as_slice()).unwrap()
    }
}

fn main() {
    let args                   = os::args();
    let file_raw_bytes         = match File::open(&Path::new(args[1].as_slice())).read_to_end() {
        Ok(bytes) => bytes,
        Err(err)  => {
            println!("{}", err);
            return;
        }
    };
    let composer_json_contents = str::from_utf8(file_raw_bytes.as_slice()).unwrap();
    let composer_json          = json::from_str(composer_json_contents).unwrap();
    let dependencies_to_check  = Dependency::to_check_from_json(composer_json);

    let mut version_ftrs: Vec<Future<(String, Option<Version>)>> = dependencies_to_check.clone().into_iter().map(|d| {
        Future::spawn(proc() {;
            (d.name.clone(), d.packagist_version())
        })
    }).collect();

    let versions: Vec<(String, Version)> = version_ftrs.iter_mut().map(
        |ftr| ftr.get()
    ).filter_map(
        |tpl| match tpl {
            (name, Some(version)) => Some((name, version)),
            (_, None) => None
        }
    ).collect();

    for dependency in dependencies_to_check.iter() {
        for &(ref name, ref version) in versions.iter() {
            if dependency.name == *name {
                if !dependency.version.matches(version) {
                    println!("{}: {} doesn't match {}", dependency.name, version, dependency.version)
                } else {
                    println!("{}: {} matches {}", dependency.name, version, dependency.version)
                }
            }
        }
    }
}
