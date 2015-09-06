extern crate cargo;
extern crate eventual;
extern crate hyper;
extern crate puppetfile;
extern crate semver;
extern crate toml;
extern crate rustc_serialize;

use std::fs::File;
use std::env;
use std::io::Read;
use std::path::Path;
use dependency::{CargoDependency, ComposerDependency, Dependency, NpmDependency, PuppetDependency};

use eventual::{join, Async, Future};
use semver::Version;

mod dependency;

fn get_published_versions<Dep: Dependency>(dependencies_to_check: &Vec<Dep>)
                                           -> Vec<(String, Version)> {

    let version_ftrs = dependencies_to_check.iter().map(|d| {
        let dependency = d.clone();

        Future::spawn(move || {
            (dependency.name().clone(), dependency.registry_version())
        })
    }).collect::<Vec<_>>();

    join(version_ftrs).await().unwrap().into_iter().filter_map(|tpl| match tpl {
        (name, Some(version)) => Some((name, version)),
        _ => None
    }).collect()
}

fn filter_dependencies<Dep: Dependency>(dependencies_to_check: &Vec<Dep>,
                                        published_versions: Vec<(String, Version)>)
                                        -> (Vec<(&Dep, Version)>, Vec<(&Dep, Version)>) {
    let mut outdated_dependencies = dependencies_to_check.iter()
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref ver)) if !d.version_req().matches(ver) => Some((d, ver.clone())),
            _ => None
        })
        .collect::<Vec<_>>();

    let mut up_to_date_dependencies = dependencies_to_check.iter()
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref ver)) if d.version_req().matches(ver) => Some((d, ver.clone())),
            _ => None
        })
        .collect::<Vec<_>>();

    outdated_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));
    up_to_date_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));

    (outdated_dependencies, up_to_date_dependencies)
}

fn out<Dep>((outdated_dependencies, up_to_date_dependencies): (Vec<(&Dep, Version)>, Vec<(&Dep, Version)>))
    where Dep: Dependency
{
    if up_to_date_dependencies.len() != 0 {
        println!("");

        for (dependency, version) in up_to_date_dependencies {
            println!("{}: {} matches {}", dependency.name(), version, dependency.version_req());
        }
    }

    if outdated_dependencies.len() != 0 {
        println!("");

        for (dependency, version) in outdated_dependencies {
            println!("{}: {} doesn't match {}", dependency.name(), version, dependency.version_req());
        }
    }
}

fn check<Dep: Dependency>(dependencies: &Vec<Dep>) {
    out(filter_dependencies(dependencies, get_published_versions(dependencies)))
}

fn main() {
    env::args().skip(1).map(|arg| {
        let path = Path::new(&arg);
        let mut dependency_file_contents = String::new();
        if let Err(err) = File::open(path).map(|mut f|
           if let Err(err) = f.read_to_string(&mut dependency_file_contents) {
               println!("{}", err); return;
           }
        ) {
            println!("{}", err); return;
        };

        println!("File to check: {}", path.display());
        match path.file_name() {
            Some(name) if name.to_str() == Some("Cargo.toml") => {
                check(&CargoDependency::to_check(&dependency_file_contents, &path));
            }
            Some(name) if name.to_str() == Some("composer.json") => {
                check(&ComposerDependency::to_check(&dependency_file_contents, &path));
            }
            Some(name) if name.to_str() == Some("Puppetfile") => {
                check(&PuppetDependency::to_check(&dependency_file_contents, &path));
            }
            Some(name) if name.to_str() == Some("package.json") => {
                check(&NpmDependency::to_check(&dependency_file_contents, &path));
            }
            _ => {
                println!("File type not recognized");
            }
        };
        println!("\n");
    }).collect::<Vec<_>>();
}
