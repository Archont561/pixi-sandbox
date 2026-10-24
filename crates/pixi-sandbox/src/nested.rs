//! Nested Pixi workspaces as `plan` sees them (TASK-129).
//!
//! `SandboxConfig` validates the *shape* of a `[[bundle.workspaces]]` declaration without
//! touching the filesystem, because every other caller of `SandboxConfig::plan` behaves that way.
//! Everything that needs a repository root lives here: does the declared manifest exist, does it
//! resolve the environments the bundle asks for, on the platforms the bundle publishes, and can
//! its bytes be counted before a runner spends an hour packing them.
//!
//! A nested workspace is deliberately *not* required to agree with the root about channels or
//! versions. That disagreement is the reason the feature exists — a subproject pins what it pins,
//! and it keeps its own lock and its own prefix. What is refused is anything that would make a
//! restore ambiguous: two declarations claiming one destination, a platform the nested lock
//! cannot serve, or a manifest the root workspace already owns.

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::sandbox_config::{Bundle, SandboxConfig, WorkspacePlan};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// Resolve every nested workspace the config declares into the report `plan` prints.
///
/// # Errors
///
/// Returns an error naming the declaration that cannot be carried: a nested manifest that is a
/// member of the root Pixi workspace, a platform the nested manifest does not declare, or an
/// environment its lock cannot resolve for a platform the bundle publishes.
pub fn plan_workspaces(config: &SandboxConfig, repo_root: &Path) -> Result<Vec<WorkspacePlan>> {
    let mut plans = Vec::new();
    let root_members = root_workspace_members(repo_root);
    for bundle in &config.bundles {
        for declaration in &bundle.workspaces {
            let directory = declaration.directory()?;
            if root_members.contains(directory) {
                bail!(
                    "bundle {:?} declares nested workspace {:?}, but the root pixi.toml lists it \
                     as a workspace member; a member resolves from the root lock and \
                     .pixi/envs, so declaring it separately would duplicate a prefix — remove \
                     the declaration or the membership",
                    bundle.name,
                    declaration.manifest
                );
            }
            let project = repo_root.join(directory);
            let manifest = read_manifest(&project.join("pixi.toml"))?;
            check_platforms(bundle, declaration, &manifest)?;
            let environments = declaration.selected_environments();
            let facts =
                collect_lock_facts(&project, &environments, &bundle.platforms, declaration)?;
            plans.push(WorkspacePlan {
                bundle: bundle.name.clone(),
                manifest: declaration.manifest.clone(),
                directory: directory.to_string(),
                slug: declaration.slug()?,
                environments,
                packages: facts.packages,
                size_bytes: facts.size_bytes,
                channels: facts.channels,
                identities: facts.identities,
            });
        }
    }
    Ok(plans)
}

/// The directories the root `pixi.toml` claims as workspace members.
///
/// pixi 0.83 rejects `[workspace] members` outright, so on the pinned version this is always
/// empty — the check exists for the day it is not, and for a repository whose root manifest is
/// written for a version that does accept it. A parse failure is not an error here: this function
/// answers "is the declared directory already owned", and a manifest this code cannot read is a
/// question for `pixi install`, not for a plan-time refusal.
fn root_workspace_members(repo_root: &Path) -> BTreeSet<String> {
    let path = repo_root.join("pixi.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeSet::new();
    };
    let Ok(manifest) = text.parse::<toml::Table>() else {
        return BTreeSet::new();
    };
    let mut members = BTreeSet::new();
    if let Some(list) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("members"))
        .and_then(|members| members.as_array())
    {
        for member in list {
            if let Some(member) = member.as_str() {
                members.insert(member.trim_end_matches('/').to_string());
            }
        }
    }
    members
}

fn read_manifest(path: &Path) -> Result<toml::Table> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading nested workspace manifest {}", path.display()))?;
    text.parse::<toml::Table>()
        .with_context(|| format!("parsing nested workspace manifest {}", path.display()))
}

/// A bundle publishes a fixed platform list; a nested manifest that declares a different one
/// cannot be packed for the platforms the bundle names, and finding that out on a runner is a
/// wasted matrix job.
fn check_platforms(
    bundle: &Bundle,
    declaration: &pixi_sandbox_core::sandbox_config::WorkspaceDecl,
    manifest: &toml::Table,
) -> Result<()> {
    let Some(declared) = manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("platforms"))
        .and_then(|platforms| platforms.as_array())
    else {
        // No `[workspace] platforms`: pixi falls back to the host platform, which a bundle
        // cannot plan for. Report it rather than guessing one.
        bail!(
            "bundle {:?} declares nested workspace {:?}, whose pixi.toml names no platforms; \
             add a `[workspace] platforms` list so the bundle's platforms can be checked",
            bundle.name,
            declaration.manifest
        );
    };
    let declared: BTreeSet<&str> = declared.iter().filter_map(|value| value.as_str()).collect();
    let missing: Vec<&str> = bundle
        .platforms
        .iter()
        .map(String::as_str)
        .filter(|platform| !declared.contains(platform))
        .collect();
    if !missing.is_empty() {
        bail!(
            "bundle {:?} publishes {:?}, but nested workspace {:?} declares only {:?}; add the \
             platform there or drop it from this bundle",
            bundle.name,
            missing,
            declaration.manifest,
            declared.into_iter().collect::<Vec<_>>()
        );
    }
    Ok(())
}

/// What `plan` reports about one nested workspace's lock: the package count and recorded sizes
/// the cost review needs, plus the channels and resolved identities AC#3's transparency control
/// surfaces. Every collection is de-duplicated and sorted, so two environments sharing a package
/// report it once while the count still sums per-environment selections.
struct LockFacts {
    packages: usize,
    size_bytes: u64,
    channels: Vec<String>,
    identities: Vec<String>,
}

/// Collect what the nested lock resolves for `environments` across `platforms`.
///
/// Counted, never estimated: the number is what a reviewer compares against the bundle's cost.
/// A lock that records no `size` field contributes nothing to the byte count, so the total is
/// reported as `0` (unknown) rather than as a plausible-looking guess.
fn collect_lock_facts(
    project: &Path,
    environments: &[String],
    platforms: &[String],
    declaration: &pixi_sandbox_core::sandbox_config::WorkspaceDecl,
) -> Result<LockFacts> {
    let lock_path = project.join("pixi.lock");
    let lock: serde_yaml::Value = serde_yaml::from_slice(
        &fs::read(&lock_path).with_context(|| format!("reading {}", lock_path.display()))?,
    )
    .with_context(|| format!("parsing {}", lock_path.display()))?;
    let records = lock["packages"].as_sequence();
    let mut packages = 0usize;
    let mut size_bytes = 0u64;
    let mut channels: BTreeSet<String> = BTreeSet::new();
    let mut identities: BTreeSet<String> = BTreeSet::new();
    for environment in environments {
        if let Some(urls) = lock["environments"][environment]["channels"].as_sequence() {
            for channel in urls {
                if let Some(url) = channel["url"].as_str() {
                    channels.insert(url.to_string());
                }
            }
        }
        for platform in platforms {
            let Some(selected) =
                lock["environments"][environment]["packages"][platform].as_sequence()
            else {
                bail!(
                    "nested workspace {:?} cannot serve bundle environment {:?} for {platform}; \
                     add it there with `pixi install -e {environment}` or drop it from the \
                     declaration",
                    declaration.manifest,
                    environment
                );
            };
            packages += selected.len();
            for entry in selected {
                let location = entry["conda"]
                    .as_str()
                    .or_else(|| entry["pypi"].as_str())
                    .unwrap_or_default();
                let Some(record) = records.and_then(|records| {
                    records.iter().find(|record| {
                        record["conda"].as_str() == Some(location)
                            || record["pypi"].as_str() == Some(location)
                    })
                }) else {
                    continue;
                };
                if let Some(size) = record["size"].as_u64() {
                    size_bytes += size;
                }
                if let Some(identity) = package_identity(record, location) {
                    identities.insert(identity);
                }
            }
        }
    }
    Ok(LockFacts {
        packages,
        size_bytes,
        channels: channels.into_iter().collect(),
        identities: identities.into_iter().collect(),
    })
}

/// Resolve one locked record to a reviewable identity: `name==version=build` for a Conda
/// package, `name==version` for a `PyPI` wheel.
///
/// A pixi-written v7 lock stores no name/version/build on the record — rattler takes them from
/// the archive — so the fallback parses the location's filename. Conda archives follow
/// `<name>-<version>-<build>.(conda|tar.bz2)` and wheels the PEP 427
/// stale-ref-allowed: PEP 427 filename tag naming the interpreter, not the deleted 0.2.0 tool.
/// `<name>-<version>-<python>-<abi>-<platform>.whl` layout; a name or version may itself carry
/// dashes/underscores, so the split is anchored on the right side.
fn package_identity(record: &serde_yaml::Value, location: &str) -> Option<String> {
    let filename = location.rsplit('/').next().unwrap_or(location);
    if record["conda"].is_string() {
        if let (Some(name), Some(version), Some(build)) = (
            record["name"].as_str(),
            record["version"].as_str(),
            record["build"].as_str(),
        ) {
            return Some(format!("{name}=={version}={build}"));
        }
        let stem = filename
            .strip_suffix(".tar.bz2")
            .or_else(|| filename.strip_suffix(".conda"))?;
        let mut parts = stem.rsplit('-');
        let build = parts.next()?;
        let version = parts.next()?;
        let name = parts
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("-");
        (!name.is_empty()).then(|| format!("{name}=={version}={build}"))
    } else if record["pypi"].is_string() {
        if let (Some(name), Some(version)) = (record["name"].as_str(), record["version"].as_str()) {
            return Some(format!("{name}=={version}"));
        }
        let stem = filename.strip_suffix(".whl")?;
        let mut parts = stem.split('-');
        let name = parts.next()?;
        let version = parts.next()?;
        Some(format!("{name}=={version}"))
    } else {
        None
    }
}
