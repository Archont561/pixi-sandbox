//! Install the conda package(s) that `pixi run package` just built and run the packaged
//! binary on *this* runner (task-23, AC#2).
//!
//! Rust, not shell, on purpose: the predecessor script used a Bash-4 builtin and GitHub's
//! macOS runners run scripts with Apple's /bin/bash 3.2 (the empty `package` environment adds
//! no shell of its own), so both darwin legs of the v0.3.6 release died with exit 127 — run
//! 36865921206. An xtask has one runtime on all five runners, which is the cross-platform
//! story this smoke test exists to prove for the package itself.
//!
//! `pixi global install --path <file.conda>` is used rather than a channel URL because it
//! takes a plain filesystem path, so the identical invocation works on the Unix runners and
//! on windows-latest (where a `file://` URL would have to be assembled from a Windows path).
//! The installed binary is then run from pixi's global bin directory (the `trampoline`),
//! which does not require `$PIXI_HOME/bin` to be on the caller's PATH.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn smoke_conda_package(root: &Path, out_dir: &Path) -> Result<()> {
    // The version the tree declares is the only version a package built from this tree may
    // carry. `check-repository` proves the package *manifest* agrees; this proves the artifact
    // that was actually produced agrees, which is the claim the release makes.
    let expected = crate::version::workspace_version(root)
        .context("cannot read the workspace version from Cargo.toml")?;

    let packages = conda_packages(out_dir)?;
    if packages.is_empty() {
        bail!(
            "no .conda in {} — `pixi run package` produced nothing on this runner",
            out_dir.display()
        );
    }

    for package in &packages {
        let name = package.file_name().map_or_else(
            || package.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        eprintln!("installing {name}");

        let install = Command::new("pixi")
            .args(["global", "install", "--path"])
            .arg(package)
            .arg("--force-reinstall")
            .stdout(Stdio::null())
            .status()
            .context("pixi is not on PATH — install it before smoke-testing the package")?;
        if !install.success() {
            bail!(
                "the package for this platform does not install: {}",
                package.display()
            );
        }

        let bin = installed_binary()?;
        let version_out = Command::new(&bin)
            .arg("--version")
            .output()
            .with_context(|| {
                format!(
                    "the packaged binary does not run on this platform: {}",
                    package.display()
                )
            })?;
        if !version_out.status.success() {
            bail!(
                "the packaged binary does not run on this platform: {}",
                package.display()
            );
        }
        let stdout = String::from_utf8_lossy(&version_out.stdout);
        let reported = stdout.split_whitespace().last().unwrap_or("").to_string();
        if reported != expected {
            bail!("packaged binary reports {reported}, but this tree is {expected} ({name})");
        }

        // --version proves it links and starts; a second invocation proves the CLI surface
        // survived packaging.
        let help = Command::new(&bin)
            .arg("--help")
            .stdout(Stdio::null())
            .status()
            .with_context(|| {
                format!("the packaged binary cannot parse its own arguments ({name})")
            })?;
        if !help.success() {
            bail!("the packaged binary cannot parse its own arguments ({name})");
        }

        eprintln!("ok: {name} runs on this runner and reports {reported}");
    }
    Ok(())
}

/// Every `*.conda` directly inside `out_dir`, sorted for a deterministic log.
#[doc(hidden)] // test boundary: the listing rule the smoke probe reports on (tests/smoke.rs)
pub fn conda_packages(out_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut packages = Vec::new();
    if let Ok(entries) = std::fs::read_dir(out_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|e| e == "conda") {
                packages.push(path);
            }
        }
    }
    packages.sort();
    Ok(packages)
}

/// Where pixi exposes the installed binary: `$PIXI_HOME/bin` (default `~/.pixi/bin`), with the
/// `.exe` suffix tried for the Windows runner.
fn installed_binary() -> Result<PathBuf> {
    let pixi_home = std::env::var_os("PIXI_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|h| h.join(".pixi")))
        .context(
            "neither PIXI_HOME, HOME nor USERPROFILE is set — cannot locate pixi's global bin dir",
        )?;
    let bare = pixi_home.join("bin").join("pixi-sandbox");
    let exe = bare.with_extension("exe");
    for candidate in [&bare, &exe] {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }
    bail!(
        "pixi installed the package but exposed no runnable binary at {}",
        bare.display()
    );
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}
