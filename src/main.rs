#![feature(slicing_syntax)]
#![feature(phase)]

extern crate hyper;
extern crate puppetfile;
extern crate semver;
extern crate serialize;
extern crate term;

#[phase(plugin, link)] extern crate log;

use std::os;
use std::str;
use std::io::File;
use std::sync::Future;
use dependency::{ComposerDependency, Dependency, NpmDependency, PuppetDependency};
use semver::Version;

mod dependency;

fn get_published_versions<Dep: Dependency>(dependencies_to_check: &Vec<Dep>)
    -> Vec<(String, Version)>
{
    let mut version_ftrs = dependencies_to_check.iter().map(|d| {
        let dependency = d.clone();

        Future::spawn(move || {
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

fn filter_dependencies<Dep: Dependency>(dependencies_to_check: &Vec<Dep>, published_versions: Vec<(String, Version)>)
    -> (Vec<(&Dep, &Version)>, Vec<(&Dep, &Version)>)
{
    let mut outdated_dependencies = dependencies_to_check.iter()
        .filter(|d| d.version_req().is_some())
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if !d.version_req().unwrap().matches(version) => Some((d, version)),
            _ => None
        })
        .collect::<Vec<(&Dep, &Version)>>();

    let mut up_to_date_dependencies = dependencies_to_check.iter()
        .filter(|d| d.version_req().is_some())
        .filter_map(|d| match published_versions.iter().find(|&&(ref name, _)| d.name() == name) {
            Some(&(_, ref version)) if d.version_req().unwrap().matches(version) => Some((d, version)),
            _ => None
        })
        .collect::<Vec<(&Dep, &Version)>>();

    outdated_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));
    for &(ref dep, ref version) in outdated_dependencies.iter() {
        debug!("Name: {}", dep.name());
        debug!("Version: {}", version);
    }
    up_to_date_dependencies.sort_by(|&(ref d1, _), &(ref d2, _)| d1.name().cmp(d2.name()));
    for &(ref dep, ref version) in up_to_date_dependencies.iter() {
        debug!("Name: {}", dep.name());
        debug!("Version: {}", version);
    }

    (outdated_dependencies, up_to_date_dependencies)
}

fn out<Dep: Dependency>(dependencies: (Vec<(&Dep, &Version)>, Vec<(&Dep, &Version)>)) {
    let mut t = term::stdout().unwrap();

    println!("");

    let (outdated_dependencies, up_to_date_dependencies) = dependencies;

    t.fg(term::color::GREEN).unwrap();
    for &(dependency, version) in up_to_date_dependencies.iter() {
        debug!("{}", dependency.name());
        writeln!(t, "{}: {} matches {}", dependency.name(), version, dependency.version_req().unwrap()).unwrap();
    }

    println!("");

    t.fg(term::color::RED).unwrap();
    for &(dependency, version) in outdated_dependencies.iter() {
        writeln!(t, "{}: {} doesn't match {}", dependency.name(), version, dependency.version_req().unwrap()).unwrap();
    }
}

fn main() {
    let args = os::args();
    let path = &Path::new(args[1][]);
    let file_raw_bytes = match File::open(path).read_to_end() {
        Ok(bytes) => bytes,
        Err(err)  => {
            println!("{}", err);
            return;
        }
    };
    let dependency_file_contents = str::from_utf8(file_raw_bytes[]).unwrap();

    match path.filename() {
        Some(name) if name == "composer.json".as_bytes() => {
            let composer_dependencies_to_check: Vec<ComposerDependency> = Dependency::to_check(dependency_file_contents);
            let published_versions = get_published_versions(&composer_dependencies_to_check);
            out(filter_dependencies(&composer_dependencies_to_check, published_versions))
        }
        Some(name) if name == "Puppetfile".as_bytes() => {
            let puppet_dependencies_to_check: Vec<PuppetDependency> = Dependency::to_check(dependency_file_contents);
            let published_versions = get_published_versions(&puppet_dependencies_to_check);
            out(filter_dependencies(&puppet_dependencies_to_check, published_versions))
        }
        Some(name) if name == "package.json".as_bytes() => {
            let npm_dependencies_to_check: Vec<NpmDependency> = Dependency::to_check(dependency_file_contents);
            let published_versions = get_published_versions(&npm_dependencies_to_check);
            out(filter_dependencies(&npm_dependencies_to_check, published_versions))
        }
        _ => {
            println!("File type not recognized");
            return
        }
    };
}
