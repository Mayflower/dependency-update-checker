#![feature(env, fs, os, path, io)]

extern crate cargo;
extern crate hyper;
extern crate puppetfile;
extern crate semver;
extern crate toml;
extern crate "rustc-serialize" as rustc_serialize;

use std::fs::File;
use std::env;
use std::io::Read;
use std::path::Path;
use std::old_path::Path as OldPath;
use std::sync::Future;
use dependency::{CargoDependency, ComposerDependency, Dependency, NpmDependency, PuppetDependency};
use semver::Version;

mod dependency;

fn get_published_versions<Dep: Dependency>(dependencies_to_check: &Vec<Dep>)
    -> Vec<(String, Version)> {

    let mut version_ftrs = dependencies_to_check.iter().map(|d| {
        let dependency = d.clone();

        Future::spawn(move || {
            (dependency.name().clone(), dependency.registry_version())
        })
    }).collect::<Vec<_>>();

    version_ftrs.iter_mut().map(|ftr| ftr.get()).filter_map(|tpl| match tpl {
        (name, Some(version)) => Some((name, version)),
        _ => None
    }).collect()
}

fn filter_dependencies<Dep: Dependency>(dependencies_to_check: &Vec<Dep>,
                                        published_versions: Vec<(String, Version)>)
    -> (Vec<(&Dep, Version)>, Vec<(&Dep, Version)>)
{
    let mut outdated_dependencies = dependencies_to_check.iter()
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if !d.version_req().matches(version) => Some((d, version.clone())),
            _ => None
        })
        .collect::<Vec<_>>();

    let mut up_to_date_dependencies = dependencies_to_check.iter()
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if d.version_req().matches(version) => Some((d, version.clone())),
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
        let new_path = Path::new(&arg);
        let path = &OldPath::new(&arg);
        let mut dependency_file_contents = String::new();
        if let Err(err) = File::open(path).map(|mut f|
           if let Err(err) = f.read_to_string(&mut dependency_file_contents) {
               println!("{}", err); return;
           }
        ) {
            println!("{}", err); return;
        };

        println!("File to check: {}", new_path.display());
        match new_path.file_name() {
            Some(name) if name.to_str() == Some("Cargo.toml") => {
                check(&<CargoDependency as Dependency>::to_check(&dependency_file_contents, path));
            }
            Some(name) if name.to_str() == Some("composer.json") => {
                check(&<ComposerDependency as Dependency>::to_check(&dependency_file_contents, path));
            }
            Some(name) if name.to_str() == Some("Puppetfile") => {
                check(&<PuppetDependency as Dependency>::to_check(&dependency_file_contents, path));
            }
            Some(name) if name.to_str() == Some("package.json") => {
                check(&<NpmDependency as Dependency>::to_check(&dependency_file_contents, path));
            }
            _ => {
                println!("File type not recognized");
            }
        };
        println!("\n");
    }).collect::<Vec<_>>();
}
