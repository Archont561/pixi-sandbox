//! Explicit local executable embedding; no catalogue, cache or network lookup.
use crate::{pack::support, user_tools::make_executable};
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::{
    manifest::ToolEntry,
    platform::executable_filename,
    shard,
    verify::{self, Linkage},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug)]
pub struct ToolSource {
    pub path: PathBuf,
    pub version: String,
    pub url: Option<String>,
    pub pinned_sha256: Option<String>,
}

/// Embed one helper tool into the payload: copy, chmod, size-limit, digest-pin and
/// linkage checks.
///
/// # Errors
///
/// Returns an error if the tool cannot be copied or made executable, exceeds the shard
/// limit, fails its pin, or is dynamically linked.
///
/// # Panics
///
/// Panics if the computed destination has no parent directory (a bug in this function,
/// not an input: the path is built from fixed segments).
pub fn embed_tool(
    payload: &Path,
    platform: &str,
    name: &str,
    source: &ToolSource,
    shard_limit: u64,
) -> Result<ToolEntry> {
    let file_name = executable_filename(name, platform);
    let relative = format!("tools/{platform}/{file_name}");
    let destination = payload.join(&relative);
    let parent = destination
        .parent()
        .expect("tool path always has a parent directory");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::copy(&source.path, &destination).with_context(|| {
        format!(
            "copying {name} from {} to {}",
            source.path.display(),
            destination.display()
        )
    })?;
    make_executable(&destination)?;

    let metadata = fs::metadata(&destination)?;
    if metadata.len() > shard_limit {
        bail!(
            "embedded tool {name} is {} bytes, above the {} byte shard limit; tools cannot be split",
            metadata.len(),
            shard_limit
        );
    }
    let actual = shard::sha256_file(&destination)?;
    if let Some(expected) = &source.pinned_sha256
        && &actual != expected
    {
        bail!(
            "integrity: embedded {name} does not match its pin (expected {expected}, got {actual})"
        );
    }

    let linkage = verify::linkage_of(&destination);
    if linkage == Linkage::Dynamic {
        bail!(
            "{name} is dynamically linked — ship the static release asset instead (decision ADR-4)"
        );
    }
    Ok(ToolEntry {
        version: source.version.clone(),
        url: source.url.clone(),
        pinned_sha256: Some(actual),
        linkage: linkage.as_str().to_string(),
        size_bytes: metadata.len(),
        path: Some(relative),
    })
}

/// Read an explicitly selected executable version.
///
/// # Errors
/// Returns an error if version reporting fails.
pub fn reported_version(path: &Path) -> Result<String> {
    let mut command = Command::new(path);
    command.arg("--version");
    let output = support::run(&mut command)
        .with_context(|| format!("asking {} for its version", path.display()))?;
    Ok(output
        .split_whitespace()
        .last()
        .unwrap_or("unknown")
        .to_string())
}
