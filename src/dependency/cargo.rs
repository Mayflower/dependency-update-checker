use semver::{Version, VersionReq};

use cargo::core::{Shell, MultiShell, ShellConfig};
use cargo::core::dependency::Dependency as CargoOrigDependency;
use cargo::core::registry::Registry;
use cargo::core::source::SourceId;
use cargo::ops::read_manifest;
use cargo::sources::registry::RegistrySource;
use cargo::util::config::Config;
use cargo::util::toml::project_layout;

use std::old_io::util::NullWriter;

use super::Dependency;

#[derive(Clone, Debug)]
pub struct CargoDependency {
    name: String,
    orig_dependency: CargoOrigDependency
}

fn get_multi_shell() -> MultiShell {
        let shell_config = ShellConfig { color: false, verbose: false, tty: false };
        let shell_a = Shell::create(Box::new(NullWriter), shell_config);
        let shell_b = Shell::create(Box::new(NullWriter), shell_config);
        MultiShell::new(shell_a, shell_b, false)
}

fn get_config_source_id<'a>(multi_shell: &'a mut MultiShell) -> (Config<'a>, SourceId) {
        let config = Config::new(multi_shell).unwrap();
        let source_id = SourceId::for_central(&config).unwrap();

        (config, source_id)
}

impl Dependency for CargoDependency {
    fn to_check(cargo_toml_contents: &str, path: &Path) -> Vec<CargoDependency> {
        let layout = project_layout(&path.dir_path());
        let mut multi_shell = get_multi_shell();
        let (config, source_id) = get_config_source_id(&mut multi_shell);

        match read_manifest(cargo_toml_contents.as_bytes(), layout, &source_id, &config) {
            Ok((manifest, _)) => manifest.dependencies().iter().map(|dep| CargoDependency {
                name: dep.name().to_string(),
                orig_dependency: dep.clone()
            }).collect(),
            _ => vec![]
        }
    }

    fn name(&self) -> &String {
        &self.name
    }

    fn version_req(&self) -> &VersionReq {
        &self.orig_dependency.version_req()
    }

    fn registry_version(&self) -> Option<Version> {
        let mut multi_shell = get_multi_shell();
        let (config, source_id) = get_config_source_id(&mut multi_shell);
        let mut registry = RegistrySource::new(&source_id, &config);
        let summaries = match registry.query(&self.orig_dependency) {
            Ok(summaries) => summaries,
            Err(_) => return None
        };
        summaries.into_iter().map(|s| s.version().clone()).max()
    }
}
