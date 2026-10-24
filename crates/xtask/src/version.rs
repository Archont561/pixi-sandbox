//! The project version, from the one place it is written down.
//!
//! `[workspace.package] version` in the root Cargo.toml is the single source of truth: every
//! crate inherits it, `prepare-release` stamps it, and everything else DERIVES it rather than
//! restating it. The docs site reads the same table itself at build time
//! (docs/astro.config.mjs), so a release rewrites zero documentation lines there.

use anyhow::{Context, Result, bail, ensure};
use std::path::Path;

/// The prefix of the tag that starts the auto-release lane (doc-18 §5.2).
pub const MILESTONE_TAG_PREFIX: &str = "milestone-";

/// Read `[workspace.package].version` from `<root>/Cargo.toml`, parsed as TOML rather than
/// matched by line shape, so a dependency pin or a member manifest can never be mistaken for
/// the declared version.
///
/// # Errors
///
/// Fails when `Cargo.toml` cannot be read or parsed, or when it has no `[workspace.package].version`.
pub fn workspace_version(root: &Path) -> Result<String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest: toml::Table = crate::util::read(&manifest_path)?
        .parse()
        .with_context(|| format!("parsing {}", manifest_path.display()))?;
    let Some(version) = manifest
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
    else {
        bail!(
            "no [workspace.package] version in {}",
            manifest_path.display()
        );
    };
    Ok(version.to_string())
}

/// `xtask version`: print the bare semver (no leading `v`, no trailing newline chatter),
/// so `$(cargo run -q -p xtask -- version)` is a plain command substitution.
///
/// # Errors
///
/// Fails when the workspace version cannot be read.
pub fn print_version(root: &Path) -> Result<()> {
    print!("{}", workspace_version(root)?);
    Ok(())
}

/// Resolve a milestone tag into the explicit release selector it promises.
///
/// The workflow only starts on the `milestone-v*` glob, so every tag reaching this command
/// claims to carry a `vX.Y.Z`. A tag that does not is operator error: fail closed rather than
/// silently cutting an `auto`-bumped release from an unexpected ref. The returned selector
/// (`vX.Y.Z`) is the explicit-version arm `prepare-release` already understands.
///
/// # Errors
///
/// Fails when `tag` is not `milestone-v` followed by a strict `X.Y.Z` version.
pub fn milestone_tag_version(tag: &str) -> Result<String> {
    let rest = tag
        .strip_prefix(MILESTONE_TAG_PREFIX)
        .filter(|rest| rest.starts_with('v'))
        .with_context(|| format!("{tag:?} is not a milestone-vX.Y.Z tag"))?;
    let version = &rest[1..];
    ensure!(
        crate::util::is_strict_semver(version),
        "{tag:?} does not carry a strict X.Y.Z version after milestone-v"
    );
    Ok(format!("v{version}"))
}
