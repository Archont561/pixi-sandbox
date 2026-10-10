//! Pinned helper fetching and verification (D4); embedding never changes the selected bytes.

use crate::pack::support;
use crate::user_tools::make_executable;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::ToolEntry;
use pixi_sandbox_core::shard;
use pixi_sandbox_core::tools_lock::{ToolsLock, executable_filename};
use pixi_sandbox_core::verify::{self, Linkage};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::Command;

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
    if let Some(expected) = &source.pinned_sha256 {
        if &actual != expected {
            bail!(
                "integrity: embedded {name} does not match its pin (expected {expected}, got {actual})"
            );
        }
    }

    let linkage = verify::linkage_of(&destination);
    if linkage == Linkage::Dynamic {
        bail!("{name} is dynamically linked — ship the static release asset instead (decision D4)");
    }
    Ok(ToolEntry {
        version: source.version.clone(),
        url: source.url.clone(),
        pinned_sha256: source.pinned_sha256.clone(),
        linkage: linkage.as_str().to_string(),
        size_bytes: metadata.len(),
        path: Some(relative),
    })
}

/// Load the embedded pins, or a deliberate override resolved relative to the project.
///
/// # Errors
///
/// Returns an error if the override or the embedded pins cannot be read or parsed.
pub fn resolve_lock(root: &Path, override_path: Option<&Path>) -> Result<ToolsLock> {
    match override_path {
        Some(path) => {
            let path = if path.is_absolute() {
                path.to_path_buf()
            } else {
                root.join(path)
            };
            ToolsLock::load(&path)
                .with_context(|| format!("reading tool-pin override {}", path.display()))
        }
        None => ToolsLock::embedded().context("loading embedded helper-tool pins"),
    }
}

/// The external download boundary. Policy still owns checksum verification, atomic cache
/// replacement, permissions and version checking; an adapter supplies only the byte stream.
pub trait ToolDownload {
    /// # Errors
    ///
    /// Returns an error if the URL cannot be opened (a failed request, a non-2xx
    /// response).
    fn open(&self, url: &str) -> Result<Box<dyn Read>>;
}

/// The CLI's HTTP adapter, using the same request and streaming body as before extraction.
#[derive(Debug, Default)]
pub struct HttpDownload;

impl ToolDownload for HttpDownload {
    fn open(&self, url: &str) -> Result<Box<dyn Read>> {
        Ok(Box::new(ureq::get(url).call()?.into_body().into_reader()))
    }
}

/// Download, verify, cache and version-check one fully pinned helper.
///
/// # Errors
///
/// Returns an error if the download, the checksum verification, the cache write, or the
/// version check fails.
pub fn fetch_tool(
    lock: &ToolsLock,
    name: &str,
    platform: &str,
    cache: &Path,
) -> Result<FetchedTool> {
    fetch_tool_with(lock, name, platform, cache, &HttpDownload)
}

#[derive(Debug)]
pub struct FetchedTool {
    pub path: PathBuf,
    pub version: String,
    pub url: String,
    pub sha256: String,
}

/// The tools cache directory: `explicit` when given, else `$HOME/.cache/pixi-sandbox/tools`.
///
/// # Errors
///
/// Returns an error if no explicit path is given and HOME is not set, or the path cannot
/// be made absolute.
pub fn tools_cache(explicit: Option<&Path>, home: Option<&std::ffi::OsStr>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        support::absolute(path)
    } else {
        let home =
            home.ok_or_else(|| anyhow::anyhow!("cannot choose a tools cache: HOME is not set"))?;
        Ok(PathBuf::from(home)
            .join(".cache")
            .join("pixi-sandbox")
            .join("tools"))
    }
}

/// [`fetch_tool`] with an explicit download adapter — the seam tests fake.
///
/// # Errors
///
/// Returns an error if the download, the checksum verification, the cache write, or the
/// version check fails.
///
/// # Panics
///
/// Panics if the selected pin is missing (a bug: the pin was checked before download).
pub fn fetch_tool_with(
    lock: &ToolsLock,
    name: &str,
    platform: &str,
    cache: &Path,
    download: &dyn ToolDownload,
) -> Result<FetchedTool> {
    let tool = lock
        .tools
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("{name} is not pinned in the tools lock"))?;
    let pin = lock
        .pin(name, platform)
        .ok_or_else(|| anyhow::anyhow!("{name} has no pin for platform {platform}"))?;
    let url = lock
        .url(name, platform)
        .expect("pin and tool were checked above");

    fs::create_dir_all(cache)
        .with_context(|| format!("creating tools cache {}", cache.display()))?;
    let cached_name = executable_filename(&format!("{name}-{}-{platform}", tool.version), platform);
    let destination = cache.join(cached_name);
    let cached = destination.is_file()
        && shard::sha256_file(&destination).is_ok_and(|actual| actual == pin.sha256);

    if !cached {
        println!("  fetch {name} {} ({})", tool.version, pin.target);
        let temporary = destination.with_file_name(format!(
            ".{}.download-{}",
            destination
                .file_name()
                .and_then(|file| file.to_str())
                .unwrap_or(name),
            std::process::id()
        ));
        support::remove_path(&temporary)?;
        let result = (|| -> Result<()> {
            let mut source = download
                .open(&url)
                .map_err(|error| anyhow::anyhow!("downloading {name} from {url}: {error}"))?;
            let mut output = File::create(&temporary)
                .with_context(|| format!("creating {}", temporary.display()))?;
            io::copy(&mut source, &mut output)
                .with_context(|| format!("writing download for {name}"))?;
            drop(output);
            let actual = shard::sha256_file(&temporary)?;
            if actual != pin.sha256 {
                bail!(
                    "integrity: {name} from {url} does not match the selected tool pins (expected {}, got {actual})",
                    pin.sha256
                );
            }
            support::remove_path(&destination)?;
            fs::rename(&temporary, &destination).with_context(|| {
                format!(
                    "moving verified {name} into the tools cache at {}",
                    destination.display()
                )
            })?;
            Ok(())
        })();
        if result.is_err() {
            let _ = support::remove_path(&temporary);
        }
        result?;
    }

    make_executable(&destination)?;
    let reported = reported_version(&destination)?;
    if !reported.contains(&tool.version) {
        bail!(
            "{name}: selected tool pins require {} but {} reports {reported:?}",
            tool.version,
            destination.display()
        );
    }
    Ok(FetchedTool {
        path: destination,
        version: tool.version.clone(),
        url,
        sha256: pin.sha256.clone(),
    })
}

/// Ask a tool binary for its `--version` output.
///
/// # Errors
///
/// Returns an error if the tool cannot be run or its output cannot be read.
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
