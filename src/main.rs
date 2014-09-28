extern crate http;
extern crate puppetfile;
extern crate semver;
extern crate serialize;
extern crate url;

use std::os;
use std::str;
use std::io::File;
use std::sync::Future;
use dependency::{ComposerDependency, Dependency, PuppetDependency};
use semver::Version;

mod dependency;

fn get_published_versions(dependencies_to_check: &Vec<Box<Dependency>>)
    -> Vec<(String, Version)>
{
    let mut version_ftrs = dependencies_to_check.iter().map(|d| {
        let dependency = d.clone_dep();

        Future::spawn(proc() {
            (dependency.name().clone(), dependency.registry_version())
        })
    }).collect::<Vec<Future<(String, Option<Version>)>>>();

    version_ftrs.iter_mut().map(
        |ftr| ftr.get()
    ).filter_map(
        |tpl| match tpl {
            (name, Some(version)) => Some((name, version)),
            (_, None) => None
        }
    ).collect()
}

fn main() {
    let args = os::args();
    let path = &Path::new(args[1].as_slice());
    let file_raw_bytes = match File::open(path).read_to_end() {
        Ok(bytes) => bytes,
        Err(err)  => {
            println!("{}", err);
            return;
        }
    };
    let dependency_file_contents = str::from_utf8(file_raw_bytes.as_slice()).unwrap();

    let dependencies_to_check: Vec<Box<Dependency>> = match path.filename() {
        Some(name) if name == "composer.json".as_bytes() => {
            let composer_dependencies_to_check: Vec<ComposerDependency> = Dependency::to_check(dependency_file_contents);
            composer_dependencies_to_check.into_iter().map(
                |d| d.to_dependency_box()
            ).collect()
        }
        Some(name) if name == "Puppetfile".as_bytes() => {
            let puppet_dependencies_to_check: Vec<PuppetDependency> = Dependency::to_check(dependency_file_contents);
            puppet_dependencies_to_check.into_iter().map(
                |d| d.to_dependency_box()
            ).collect()
        }
        _ => vec![]
    };
    let published_versions = get_published_versions(&dependencies_to_check);

    for dependency in dependencies_to_check.iter() {
        for &(ref name, ref version) in published_versions.iter() {
            if dependency.name() == name {
                if dependency.version_req().is_none() || !dependency.version_req().unwrap().matches(version) {
                    println!("{}: {} doesn't match {}", dependency.name(), version, dependency.version_req())
                } else {
                    println!("{}: {} matches {}", dependency.name(), version, dependency.version_req())
                }
            }
        }
    }
}
