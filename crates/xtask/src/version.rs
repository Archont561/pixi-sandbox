//! The project version, from the one place it is written down.
//!
//! `[workspace.package] version` in the root Cargo.toml is the single source of truth: every
//! crate inherits it, `prepare-release` stamps it, and everything else DERIVES it rather than
//! restating it. The docs site reads the same table itself at build time
//! (docs/astro.config.mjs), so a release rewrites zero documentation lines there.

use anyhow::{Context, Result, bail};
use std::path::Path;

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
