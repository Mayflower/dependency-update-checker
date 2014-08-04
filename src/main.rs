extern crate semver;
extern crate serialize;

use std::os;
use std::str;
use std::io::File;
use std::collections::TreeMap;
use semver::Version;
use serialize::json::{mod, Json};

#[deriving(Show)]
struct Dependency {
    name    : String,
    version : Version
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
            Some((ref name, ref version)) if !version.as_slice().contains("dev") => {
                println!("{}", version);
                Some(Dependency { name: name.clone(), version: semver::parse(version.as_slice()).unwrap() })
            },
            _ => None
        }).collect()
    }
}

fn main() {
    let args                   = os::args();
    let file_raw_bytes         = File::open(&Path::new(args[1].as_slice())).read_to_end().unwrap();
    let composer_json_contents = str::from_utf8(file_raw_bytes.as_slice()).unwrap();
    let composer_json          = json::from_str(composer_json_contents).unwrap();
    let dependencies_to_check  = Dependency::to_check_from_json(composer_json);

    //let mut version_ftrs: Vec<Future<(String, Option<Version>)>> = modules.move_iter().filter(
        //|m| m.user_name_pair().is_some()
    //).map(|m| {
        //let forge_url = puppetfile.forge.clone();
        //Future::spawn(proc() {;
            //(m.name.clone(), m.forge_version(forge_url))
        //})
    //}).collect();

    println!("{}", dependencies_to_check);
}
