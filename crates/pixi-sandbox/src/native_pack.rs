//! Local pack installation without an external unpacker (TASK-125).
//!
//! This module has no downloader. Archives are read only from the supplied local
//! channel and are verified against its repodata before a prefix is created.
//! Callers retain ownership of staging, transport verification and atomic promotion.

mod wheels;

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::shard;
// stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
use rattler::install::{InstallDriver, InstallOptions, PythonInfo, link_package};
use rattler_conda_types::{
    PackageRecord, PrefixRecord, RepoDataRecord, Subdir, package::DistArchiveIdentifier,
    prefix::Prefix,
};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};
use url::Url;

struct LocalPackage {
    archive: PathBuf,
    record: RepoDataRecord,
}

/// Verify a bare local pack without creating a prefix or cache.
///
/// # Errors
/// Returns an error for unsupported metadata or a missing/corrupt/unsafe artifact.
pub fn verify(pack: &Path) -> Result<()> {
    inspect(pack).map(|_| ())
}

/// Install one local pack into a fresh staging prefix, relocating directly to its
/// eventual destination (including fixed-width binary prefixes).
///
/// # Errors
/// Refuses unsupported metadata, missing/modified archives or an existing prefix.
/// On a linking failure, the caller-owned stage is retained for diagnosis; no final
/// destination or transport file is modified.
pub fn install(pack: &Path, stage: &Path, destination: &Path, cache: &Path) -> Result<()> {
    let plan = inspect(pack)?;
    if stage.exists() {
        bail!("native install stage already exists: {}", stage.display());
    }
    if !stage.is_absolute() || !destination.is_absolute() || !cache.is_absolute() {
        bail!("native install paths must be absolute");
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("creating the local package-install runtime")?;
    runtime.block_on(link_packages(
        plan.packages,
        stage,
        destination,
        cache,
        plan.platform,
        plan.interpreter.clone(),
    ))?;
    wheels::install(
        plan.wheels,
        plan.interpreter.as_ref(),
        stage,
        destination,
        cache,
    )
}

struct InstallPlan {
    packages: Vec<LocalPackage>,
    platform: Subdir,
    // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
    interpreter: Option<PythonInfo>,
    wheels: Vec<wheels::Wheel>,
}

fn inspect(pack: &Path) -> Result<InstallPlan> {
    let metadata: Value = serde_json::from_slice(
        &fs::read(pack.join("pixi-pack.json")).context("reading local pack metadata")?,
    )
    .context("parsing local pack metadata")?;
    if metadata["version"].as_str() != Some("1") {
        bail!("unsupported local pack format; expected pixi-pack format 1");
    }
    let platform = metadata["platform"]
        .as_str()
        .context("pack has no platform")?;
    if crate::standalone::host_platform() != Some(platform) {
        bail!("local pack platform {platform} does not match this host");
    }
    let subdir: Subdir = platform.parse().context("invalid local pack platform")?;
    let packages = inspect_channel(pack, platform)?;
    let interpreter = packages
        .iter()
        // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
        .find(|package| package.record.package_record.name.as_normalized() == "python")
        // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
        .map(|package| PythonInfo::from_python_record(&package.record.package_record, subdir))
        .transpose()?;
    let wheels = wheels::inspect(pack, interpreter.as_ref())?;
    Ok(InstallPlan {
        packages,
        platform: subdir,
        interpreter,
        wheels,
    })
}

fn inspect_channel(pack: &Path, platform: &str) -> Result<Vec<LocalPackage>> {
    let mut packages = Vec::new();
    let mut names = BTreeSet::new();
    for subdir in ["noarch", platform] {
        let channel = pack.join("channel").join(subdir);
        let repodata = channel.join("repodata.json");
        if !repodata.exists() {
            continue;
        }
        let data: Value = serde_json::from_slice(&fs::read(&repodata)?)
            .with_context(|| format!("reading {}", repodata.display()))?;
        for section in ["packages", "packages.conda"] {
            let Some(entries) = data.get(section) else {
                continue;
            };
            let entries = entries
                .as_object()
                .context("repodata packages must be an object")?;
            for (filename, record) in entries {
                let mut components = Path::new(filename).components();
                if !matches!(components.next(), Some(Component::Normal(_)))
                    || components.next().is_some()
                    || filename.contains(['\\', ':'])
                    || filename.starts_with('.')
                {
                    bail!("unsafe local package filename: {filename}");
                }
                let archive = channel.join(filename);
                let metadata = fs::symlink_metadata(&archive)
                    .with_context(|| format!("missing local package {}", archive.display()))?;
                if !metadata.file_type().is_file() {
                    bail!("local package is not a regular file: {}", archive.display());
                }
                let expected = record["sha256"]
                    .as_str()
                    .context("local package has no SHA256")?;
                if shard::sha256_file(&archive)? != expected {
                    bail!("SHA256 mismatch for local package {filename}");
                }
                let package_record: PackageRecord = serde_json::from_value(record.clone())
                    .with_context(|| format!("invalid package record for {filename}"))?;
                let identifier: DistArchiveIdentifier =
                    filename.parse().map_err(|error: String| {
                        anyhow::anyhow!("invalid Conda archive filename: {error}")
                    })?;
                let identity = format!(
                    "{}-{}-{}",
                    package_record.name.as_normalized(),
                    package_record.version,
                    package_record.build
                );
                if identity != identifier.identifier.to_string() {
                    bail!("package identity disagrees with archive filename {filename}");
                }
                if package_record.subdir.as_str() != subdir {
                    bail!("package {filename} disagrees with channel subdir {subdir}");
                }
                if !names.insert(package_record.name.as_normalized().to_string()) {
                    bail!(
                        "duplicate local package: {}",
                        package_record.name.as_normalized()
                    );
                }
                packages.push(LocalPackage {
                    record: RepoDataRecord {
                        package_record,
                        identifier,
                        url: Url::from_file_path(&archive)
                            .map_err(|()| anyhow::anyhow!("invalid local archive path"))?,
                        channel: None,
                    },
                    archive,
                });
            }
        }
    }
    if packages.is_empty() {
        bail!("local pack contains no Conda packages");
    }
    Ok(packages)
}

async fn link_packages(
    packages: Vec<LocalPackage>,
    stage: &Path,
    destination: &Path,
    cache: &Path,
    platform: Subdir,
    // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
    interpreter: Option<PythonInfo>,
) -> Result<()> {
    let prefix = Prefix::create(stage)?;
    fs::create_dir_all(cache)?;
    fs::create_dir_all(stage.join("conda-meta"))?;
    let driver = InstallDriver::default();
    for (index, package) in packages.into_iter().enumerate() {
        let extracted = cache.join(index.to_string());
        rattler_package_streaming::fs::extract(&package.archive, &extracted)
            .with_context(|| format!("extracting {}", package.archive.display()))?;
        let paths = link_package(
            &extracted,
            &prefix,
            &driver,
            InstallOptions {
                target_prefix: Some(destination.to_path_buf()),
                platform: Some(platform),
                // stale-ref-allowed: installed Conda runtime/API, not the retired implementation.
                python_info: interpreter.clone(),
                // A mutable installed prefix must never mutate the extracted cache of
                // another environment through a shared hardlink.
                allow_hard_links: Some(false),
                allow_ref_links: Some(false),
                ..InstallOptions::default()
            },
        )
        .await
        .with_context(|| format!("linking {}", package.archive.display()))?;
        let record = PrefixRecord::from_repodata_record(package.record, paths);
        record.write_to_path(stage.join("conda-meta").join(record.file_name()), true)?;
    }
    fs::write(
        stage.join("conda-meta/history"),
        "// not relevant for pixi but for `conda run -p`",
    )?;
    Ok(())
}
