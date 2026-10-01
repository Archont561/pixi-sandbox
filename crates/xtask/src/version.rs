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
pub fn print_version(root: &Path) -> Result<()> {
    print!("{}", workspace_version(root)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::workspace_version;
    use std::fs;

    #[test]
    fn the_workspace_package_version_is_read_not_the_first_version_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace.dependencies]\nserde = { version = \"1\" }\n\n[workspace.package]\nversion = \"9.8.7\"\n",
        )
        .expect("manifest");
        assert_eq!(workspace_version(dir.path()).expect("version"), "9.8.7");
    }

    #[test]
    fn a_manifest_without_the_table_is_an_error_naming_the_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").expect("manifest");
        let err = workspace_version(dir.path()).expect_err("must fail");
        assert!(format!("{err:#}").contains("[workspace.package]"));
    }
}
