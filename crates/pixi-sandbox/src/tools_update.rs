//! `tools update` — the testable core: refresh the helper-tool pins from the same upstream the
//! pins name, verify every asset, and render the candidate lock.
//!
//! The CLI wrapper (`commands::tools::update`) owns the clock, the GitHub source and the choice
//! between printing and writing; everything that decides a hash lives here so it can be tested
//! through the public API.

use crate::release::{PIXI_CHECKSUM_MANIFEST, ReleaseSource};
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::tools_lock::{PlatformPin, Tool, ToolsLock};
use pixi_sandbox_core::verify::{Linkage, linkage_of_bytes};
use std::collections::BTreeMap;
use std::path::Path;

/// The release asset name: the last path segment of a fully substituted download URL.
#[must_use]
pub fn asset_name_of(url: &str) -> Option<&str> {
    let name = url.rsplit('/').next()?;
    if name.is_empty() || name.contains('{') {
        return None;
    }
    Some(name)
}

/// The `owner/repo` a pin downloads from, read out of its own URL template.
///
/// The template is the single source of truth for *where* a build came from (AC#1): the same
/// source the existing pin was compiled from, never a hardcoded repository somewhere in this
/// file that could drift away from the data it is meant to serve.
#[must_use]
pub fn github_repo(url_template: &str) -> Option<&str> {
    let rest = url_template.strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    // A GitHub release asset is always at /owner/repo/releases/download/<tag>/<name>. Requiring
    // the literal `releases` segment keeps a repo-less path from being read as a repository
    // called "releases" — which would resolve "latest" against the wrong project entirely.
    if parts.next()? != "releases" {
        return None;
    }
    if owner.is_empty() || repo.is_empty() || owner.contains('{') || repo.contains('{') {
        return None;
    }
    Some(&rest[..owner.len() + 1 + repo.len()])
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// (tool, platform, `old_version`, `new_version`) for every asset that changed.
    pub changed: Vec<(String, String, String, String)>,
    /// Assets whose version and hash were already current.
    pub unchanged: usize,
    pub checked: usize,
    pub skipped: Vec<String>,
}

impl Report {
    /// Whether any asset changed, so a caller knows the catalogue needs rewriting.
    #[must_use]
    pub fn is_change(&self) -> bool {
        !self.changed.is_empty()
    }
}

/// Print the check report and fail when any pin is out of date.
///
/// # Errors
///
/// Fails when at least one asset changed, so a scheduled check run goes red until the pins are
/// refreshed.
pub fn report_check(report: &Report) -> Result<()> {
    for (tool, platform, from, to) in &report.changed {
        println!("update {tool}/{platform}: {from} -> {to}");
    }
    for note in &report.skipped {
        println!("skipped {note}");
    }
    println!(
        "{} asset(s) checked, {} unchanged, {} to update",
        report.checked,
        report.unchanged,
        report.changed.len()
    );
    if report.is_change() {
        // Non-zero is the point: this is what a scheduled workflow keys on.
        bail!("tool pins are out of date; run `pixi-sandbox tools update`");
    }
    Ok(())
}

/// Rebuild the catalogue against the newest releases, verifying every asset on the way.
///
/// `generated_at` is stamped onto the catalogue only when something changed; the caller owns the
/// clock, so the same inputs always produce the same lock.
///
/// # Errors
///
/// Fails when a tool's release cannot be resolved or its checksums cannot be read, when an asset
/// cannot be downloaded, or when a hash disagrees with the upstream manifest or a static pin turns
/// out to be dynamic. Nothing is written in that case: the caller keeps the previous lock.
pub fn refresh(
    lock: &ToolsLock,
    source: &dyn ReleaseSource,
    only: &[String],
    generated_at: &str,
) -> Result<(ToolsLock, Report)> {
    let mut updated = lock.clone();
    let mut report = Report::default();

    for name in lock.names() {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == name) {
            report.skipped.push(format!("{name} (not selected)"));
            continue;
        }
        let tool = &lock.tools[name];
        let Some(repo) = github_repo(&tool.url_template) else {
            // An organisation mirror's URL is not a GitHub release path, so there is no
            // "latest" to resolve. Leaving the pin untouched is the safe reading: we cannot
            // know what a mirror's newest build is, and guessing would replace reviewed data.
            report.skipped.push(format!(
                "{name}: url_template is not a GitHub release URL, so the latest version cannot be \
                 resolved; keep this pin or point --tools-lock at a GitHub-hosted mirror"
            ));
            continue;
        };

        let tag = source
            .latest_tag(repo)
            .with_context(|| format!("resolving the latest {name} release from {repo}"))?;
        let version = tag.trim_start_matches('v').to_string();
        if version == tool.version {
            report.unchanged += tool.platforms.len();
            report.checked += tool.platforms.len();
            continue;
        }

        let checksums = source
            .published_checksums(repo, &tag)
            .with_context(|| format!("reading published checksums for {repo} {tag}"))?;
        let mut platforms = BTreeMap::new();

        for platform in tool.platforms.keys() {
            let pin = refresh_platform(&mut PlatformRefresh {
                source,
                report: &mut report,
                name,
                tool,
                version: &version,
                platform,
                checksums: checksums.as_ref(),
                repo,
                tag: &tag,
            })?;
            platforms.insert(platform.clone(), pin);
        }

        updated.tools.insert(
            name.to_string(),
            Tool {
                version,
                url_template: tool.url_template.clone(),
                platforms,
            },
        );
    }

    if report.is_change() {
        updated.generated_at = Some(generated_at.to_string());
    }
    Ok((updated, report))
}

/// What one platform refresh needs, bundled so the helper stays under the argument limit.
struct PlatformRefresh<'a> {
    source: &'a dyn ReleaseSource,
    report: &'a mut Report,
    name: &'a str,
    tool: &'a Tool,
    version: &'a str,
    platform: &'a str,
    checksums: Option<&'a BTreeMap<String, String>>,
    repo: &'a str,
    tag: &'a str,
}

/// Refresh one platform pin of a tool whose version changed: download the release asset,
/// record its hash (cross-checked against the published manifest when it lists the asset),
/// verify the linkage claim, and return the new pin.
fn refresh_platform(ctx: &mut PlatformRefresh<'_>) -> Result<PlatformPin> {
    let PlatformRefresh {
        source,
        report,
        name,
        tool,
        version,
        platform,
        checksums,
        repo,
        tag,
    } = ctx;
    let previous = &tool.platforms[*platform];
    let url = tool
        .url_template
        .replace("{version}", version)
        .replace("{target}", &previous.target);
    // The release asset is the *file name* of the substituted URL, which the template
    // builds as e.g. `pixi-x86_64-unknown-linux-musl`. Deriving it from the template
    // rather than from `target` alone is what keeps the tool prefix in the name; using
    // the target directly 404s against real GitHub.
    let asset_name = asset_name_of(&url).with_context(|| {
        format!("the url_template for {name} has no file name to download: {url}")
    })?;
    let asset = source
        .asset(repo, tag, asset_name)
        .with_context(|| format!("downloading {name} {version} for {platform}"))?;
    report.checked += 1;

    let actual = asset.sha256();
    let published = checksums.and_then(|map| map.get(asset_name));
    let note = match (checksums, published) {
        (Some(_), Some(expected)) if expected != &actual => bail!(
            "integrity: {name} {version} for {platform} hashes to {actual}, but {repo} \
             publishes {expected} for {asset_name} in {PIXI_CHECKSUM_MANIFEST}. Refusing \
             to write the lock; the download or the upstream manifest is not trustworthy."
        ),
        (_, Some(_)) => {
            format!("cross-checked against {PIXI_CHECKSUM_MANIFEST} in {repo} {tag}")
        }
        // Measured 2026-09-29: pixi publishes `sha256.sum` covering only the archives
        // (`pixi-x86_64-unknown-linux-musl.tar.gz`, `.zip`, `.msi`), while the catalogue
        // pins the *bare* binary that is not listed. Saying "cross-checked" there would be
        // a claim no one verified, so the weaker guarantee is written into the pin.
        (Some(_), None) => format!(
            "{PIXI_CHECKSUM_MANIFEST} in {repo} {tag} does not list {asset_name} (it \
             covers archives); hash observed and linkage verified locally, not \
             cross-checked"
        ),
        (None, _) => format!(
            "no upstream checksum manifest; hash observed on {tag} and linkage verified \
             locally, not cross-checked"
        ),
    };

    let linkage = linkage_of_bytes(&asset.bytes);
    if previous.linkage == Linkage::Static.as_str() && linkage != Linkage::Static {
        bail!(
            "integrity: {name} {version} for {platform} is a {} binary, but the pin \
             declares it static ({url}). A dynamic helper works on the build machine and \
             dies in the airlock (D4) — this is the failure the pin exists to prevent. \
             Refusing to write the lock.",
            linkage.as_str()
        );
    }

    report.changed.push((
        name.to_string(),
        platform.to_string(),
        tool.version.clone(),
        version.to_string(),
    ));
    Ok(PlatformPin {
        target: previous.target.clone(),
        sha256: actual,
        // Write what was observed. A pin that can carry a stale `linkage` claim is a
        // pin that will lie to whoever reads it next.
        linkage: linkage.as_str().to_string(),
        note: Some(note),
    })
}

/// Serialise a catalogue as pretty JSON with a trailing newline, the form the asset file is kept in.
///
/// # Errors
///
/// Fails when the catalogue cannot be serialised.
pub fn render(lock: &ToolsLock) -> Result<String> {
    let mut text = serde_json::to_string_pretty(lock).context("serialising the tools lock")?;
    text.push('\n');
    Ok(text)
}

/// Write via a sibling temp file and rename, so a reader never sees a half-written lock and a
/// failure never truncates the existing one (invariant 1: verify before write).
///
/// # Errors
///
/// Fails when the temp file cannot be written or cannot be renamed over `path`.
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let temporary = path.with_file_name(format!(
        ".{}.update-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tools.lock.json"),
        std::process::id()
    ));
    let result = (|| -> Result<()> {
        std::fs::write(&temporary, contents)
            .with_context(|| format!("writing {}", temporary.display()))?;
        std::fs::rename(&temporary, path)
            .with_context(|| format!("moving the new tools lock into place at {}", path.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}
