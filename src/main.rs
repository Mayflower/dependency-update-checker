extern crate http;
extern crate puppetfile;
extern crate semver;
extern crate serialize;
extern crate term;
extern crate url;

use std::os;
use std::str;
use std::io::File;
use std::sync::Future;
use dependency::{BowerDependency, ComposerDependency, Dependency, NpmDependency, PuppetDependency};
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
    let mut t = term::stdout().unwrap();

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
        Some(name) if name == "package.json".as_bytes() => {
            let npm_dependencies_to_check: Vec<NpmDependency> = Dependency::to_check(dependency_file_contents);
            npm_dependencies_to_check.into_iter().map(
                |d| d.to_dependency_box()
            ).collect()
        }
//        Some(name) if name == "bower.json".as_bytes() => {
//            let puppet_dependencies_to_check: Vec<BowerDependency> = Dependency::to_check(dependency_file_contents);
//            puppet_dependencies_to_check.into_iter().map(
//                |d| d.to_dependency_box()
//            ).collect()
//        }
        _ => {
            println!("File type not recognized")
            return
        }
    };
    let published_versions = get_published_versions(&dependencies_to_check);

    let mut outdated_dependencies = dependencies_to_check.iter()
        .filter(|d| d.version_req().is_some())
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if !d.version_req().unwrap().matches(version) => Some((d, version)),
            _ => None
        })
        .collect::<Vec<(&Box<Dependency>, &Version)>>();

    let mut up_to_date_dependencies = dependencies_to_check.iter()
        .filter(|d| d.version_req().is_some())
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if d.version_req().unwrap().matches(version) => Some((d, version)),
            _ => None
        })
        .collect::<Vec<(&Box<Dependency>, &Version)>>();

    outdated_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));
    up_to_date_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));

    println!("");

    t.fg(term::color::GREEN).unwrap();
    for &(dependency, version) in up_to_date_dependencies.iter() {
        (writeln!(t, "{}: {} matches {}", dependency.name(), version, dependency.version_req())).unwrap();
    }

    println!("");

    t.fg(term::color::RED).unwrap();
    for &(dependency, version) in outdated_dependencies.iter() {
        (writeln!(t, "{}: {} doesn't match {}", dependency.name(), version, dependency.version_req())).unwrap();
    }
}
