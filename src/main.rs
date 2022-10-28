use clap::Parser;
use dependency::{CargoDependency, ComposerDependency, Dependency, NpmDependency};
use std::io::{stderr, Read};
use std::{fs::File, path::PathBuf};
use tracing::metadata::LevelFilter;

use anyhow::{anyhow, bail, Context, Result};
use futures::{future::join_all, FutureExt};
use semver::Version;
use tokio::signal::unix::{signal, SignalKind};
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

mod dependency;

async fn get_published_versions<Dep: Dependency>(
    dependencies_to_check: &Vec<Dep>,
) -> Vec<(&str, Version)> {
    let version_ftrs = join_all(
        dependencies_to_check
            .iter()
            .map(|d| d.registry_version().map(|v| (d.name(), v))),
    )
    .await;

    version_ftrs
        .into_iter()
        .filter_map(|tpl| match tpl {
            (name, Some(version)) => Some((name, version)),
            _ => None,
        })
        .collect()
}

fn filter_dependencies<'a, 'b, Dep: Dependency>(
    dependencies_to_check: &'a Vec<Dep>,
    published_versions: Vec<(&'b str, Version)>,
) -> (Vec<(&'a Dep, Version)>, Vec<(&'a Dep, Version)>) {
    let mut outdated_dependencies = dependencies_to_check
        .iter()
        .filter_map(|d| {
            match published_versions
                .iter()
                .find(|&&(name, _)| d.name() == name)
            {
                Some(&(_, ref ver)) if !d.version_req().matches(ver) => Some((d, ver.clone())),
                _ => None,
            }
        })
        .collect::<Vec<_>>();

    let mut up_to_date_dependencies = dependencies_to_check
        .iter()
        .filter_map(|d| {
            match published_versions
                .iter()
                .find(|&&(name, _)| d.name() == name)
            {
                Some(&(_, ref ver)) if d.version_req().matches(ver) => Some((d, ver.clone())),
                _ => None,
            }
        })
        .collect::<Vec<_>>();

    outdated_dependencies.sort_by(|&(d1, _), &(d2, _)| d1.name().cmp(d2.name()));
    up_to_date_dependencies.sort_by(|&(d1, _), &(d2, _)| d1.name().cmp(d2.name()));

    (outdated_dependencies, up_to_date_dependencies)
}

fn out<Dep>(
    (outdated_dependencies, up_to_date_dependencies): (Vec<(&Dep, Version)>, Vec<(&Dep, Version)>),
) where
    Dep: Dependency,
{
    if up_to_date_dependencies.len() != 0 {
        println!("");

        for (dependency, version) in up_to_date_dependencies {
            println!(
                "{}: {} matches {}",
                dependency.name(),
                version,
                dependency.version_req()
            );
        }
    }

    if outdated_dependencies.len() != 0 {
        println!("");

        for (dependency, version) in outdated_dependencies {
            println!(
                "{}: {} doesn't match {}",
                dependency.name(),
                version,
                dependency.version_req()
            );
        }
    }
}

async fn check<Dep: Dependency>(dependencies: &Vec<Dep>) {
    let published_versions = get_published_versions(dependencies).await;
    debug!("published: {published_versions:#?}");
    out(filter_dependencies(dependencies, published_versions));
}

async fn check_file(path: PathBuf) -> Result<()> {
    let mut dependency_file_contents = String::new();
    File::open(&path)?.read_to_string(&mut dependency_file_contents)?;

    info!("File to check: {}", path.display());
    match path.file_name() {
        Some(name) if name.to_str() == Some("Cargo.toml") => {
            check(&CargoDependency::to_check(
                &dependency_file_contents,
                &path,
            )?)
            .await
        }
        Some(name) if name.to_str() == Some("composer.json") => {
            check(&ComposerDependency::to_check(
                &dependency_file_contents,
                &path,
            )?)
            .await
        }
        // Some(name) if name.to_str() == Some("Puppetfile") => {
        //     check(&PuppetDependency::to_check(&dependency_file_contents, &path));
        // }
        Some(name) if name.to_str() == Some("package.json") => {
            check(&NpmDependency::to_check(&dependency_file_contents, &path)?).await
        }
        _ => bail!("File type not recognized"),
    };
    println!("\n");

    Ok(())
}

#[derive(Parser)]
struct Config {
    files: Vec<String>,
}

/// initialize the default global logging subscriber for events and spans, using the environment
/// variable `RUST_LOG` for configuring the log level
fn init_tracing() -> Result<()> {
    let tracing_subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::WARN.into())
                .with_env_var("DUCK_LOG")
                .from_env_lossy(),
        )
        .with_writer(stderr)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(tracing_subscriber)
        .with_context(|| format!("failed to set global default tracing subscriber"))?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing()?;

    let config = Config::parse();
    let check_handle = join_all(
        config
            .files
            .into_iter()
            .map(|s| PathBuf::from(&s))
            .map(check_file),
    );

    let sigwait = tokio::spawn(async move { term_signal().await });

    tokio::select! {
        res = sigwait => {
            match res {
                Ok(Ok(sig)) => info!("Received signal {sig}"),
                e => error!("{e:?}")
            }
        },
        res = tokio::spawn(check_handle) => {
            let foo = res.map(|v| v.into_iter().collect::<Result<Vec<_>>>()).map_err(|e| anyhow!(e));
            match foo {
                Ok(Ok(_)) => info!("Success"),
                Ok(Err(e)) | Err(e) => error!("{e:?}"),
            }
        }
    }
    info!("Terminating");

    Ok(())
}

/// returns if any of the listed signals is received
async fn term_signal() -> Result<&'static str> {
    let mut sighup = signal(SignalKind::hangup())?;
    let mut sigint = signal(SignalKind::interrupt())?;
    let mut sigquit = signal(SignalKind::quit())?;
    let mut sigterm = signal(SignalKind::terminate())?;

    let signal = tokio::select! {
        _ = sighup.recv() => { "SIGHUP" }
        _ = sigint.recv() => { "SIGINT" }
        _ = sigquit.recv() => { "SIGQUIT" }
        _ = sigterm.recv() => { "SIGTERM" }
    };

    Ok(signal)
}
