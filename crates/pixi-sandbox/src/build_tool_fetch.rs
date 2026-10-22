//! Independent, on-demand package-builder fetching and verification (ADR-28).

use crate::embedded_tools::reported_version;
use crate::pack::support;
use crate::user_tools::make_executable;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::build_tool_pins::BuildToolPins;
use pixi_sandbox_core::platform::executable_filename;
use pixi_sandbox_core::shard;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

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
    lock: &BuildToolPins,
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
pub fn build_cache(explicit: Option<&Path>, home: Option<&std::ffi::OsStr>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        support::absolute(path)
    } else {
        let home =
            home.ok_or_else(|| anyhow::anyhow!("cannot choose a build cache: HOME is not set"))?;
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
    lock: &BuildToolPins,
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
