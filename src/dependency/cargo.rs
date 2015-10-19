use std::io::{sink, Read};
use std::path::Path;

use hyper::Client;
use rustc_serialize::json;
use semver::{Version, VersionReq};

use cargo::core::{ColorConfig, Shell, MultiShell, ShellConfig, Verbosity};
use cargo::core::dependency::Dependency as CargoOrigDependency;
use cargo::core::source::SourceId;
use cargo::ops::read_manifest;
use cargo::util::config::Config;
use cargo::util::toml::project_layout;

use super::Dependency;

#[derive(Clone, Debug)]
pub struct CargoDependency {
    name: String,
    orig_dependency: CargoOrigDependency,
}

#[derive(RustcDecodable, Debug)]
struct CratesIoVersion {
    num: String
}

#[derive(RustcDecodable, Debug)]
struct CratesIoResponse {
    versions: Vec<CratesIoVersion>
}


fn get_multi_shell() -> MultiShell {
    let shell_config = ShellConfig { color_config: ColorConfig::Never, tty: false };
    let shell = Shell::create(Box::new(sink()), shell_config);
    let shell2 = Shell::create(Box::new(sink()), shell_config);
    MultiShell::new(shell, shell2, Verbosity::Quiet)
}

fn get_config_source_id() -> (Config, SourceId) {
    let config = Config::new(get_multi_shell()).unwrap();
    let source_id = SourceId::for_central(&config).unwrap();

    (config, source_id)
}

impl Dependency for CargoDependency {
    fn to_check(cargo_toml_contents: &str, path: &Path) -> Vec<CargoDependency> {
        let layout = project_layout(&path.parent().unwrap());
        let (config, source_id) = get_config_source_id();

        match read_manifest(cargo_toml_contents.as_bytes(), layout, &source_id, &config) {
            Ok((manifest, _)) => manifest.dependencies().iter().map(|dep| CargoDependency {
                name: dep.name().to_string(),
                orig_dependency: dep.clone()
            }).collect(),
            _ => vec![],
        }
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn version_req(&self) -> &VersionReq {
        &self.orig_dependency.version_req()
    }

    fn registry_version(&self) -> Option<Version> {
        let client = Client::new();
        let mut response = client.get(
            &*format!("https://crates.io/api/v1/crates/{}", self.name)
        ).send().unwrap();
        let ref mut response_string = String::new();
        response.read_to_string(response_string).unwrap();

        json::decode::<CratesIoResponse>(response_string).map(
            |r| r.versions.iter().filter_map(
                |crio_v| Version::parse(&*crio_v.num).ok()
            ).max()
        ).ok().and_then(|id| id)
    }
}
