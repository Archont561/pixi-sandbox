//! Standalone release-binary assets: staging and checksums (task-36).
//!
//! The release matrix used to carry these as per-platform shell blocks — a bash dialect on
//! Unix and a PowerShell dialect on Windows for what is the same three operations (find the
//! built binary, strip it if a strip exists, copy it under its asset name). One Rust runtime
//! covers all five runners; the v0.3.6 release died on macOS's Bash 3.2 over exactly this
//! class of step. The functions here are pure over paths so tests drive them against tempdir
//! fixtures, never this checkout (D10).

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::shard::sha256_file;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[doc(hidden)] // test boundary (tests/release_assets.rs)
pub const SUMS_NAME: &str = "SHA256SUMS";

/// Asset name of a staged binary: `pixi-sandbox-<target>`, with the `.exe` suffix Windows
/// targets require. Derived from the triple so the naming is testable without a Windows host.
#[doc(hidden)] // test boundary (tests/release_assets.rs)
#[must_use]
pub fn staged_name(target: &str) -> String {
    if target.contains("windows") {
        format!("pixi-sandbox-{target}.exe")
    } else {
        format!("pixi-sandbox-{target}")
    }
}

/// The binary name cargo produces (`pixi-sandbox`, `pixi-sandbox.exe` on Windows targets).
#[doc(hidden)] // test boundary (tests/release_assets.rs)
#[must_use]
pub fn binary_name(target: &str) -> String {
    if target.contains("windows") {
        "pixi-sandbox.exe".to_string()
    } else {
        "pixi-sandbox".to_string()
    }
}

/// Where cargo puts the release binary: `target/<triple>/release/` when `--target` was
/// passed, `target/release/` for a host build — the host triple *names* the binary but does
/// not nest it, which is exactly the difference the staging has to honour.
#[doc(hidden)] // test boundary (tests/release_assets.rs)
#[must_use]
pub fn built_binary_path(target_dir: &Path, target: Option<&str>, host_triple: &str) -> PathBuf {
    let release = match target {
        Some(triple) => target_dir.join(triple).join("release"),
        None => target_dir.join("release"),
    };
    release.join(binary_name(target.unwrap_or(host_triple)))
}

/// The host triple from `rustc -vV`, so a local `stage-release-binary` with no `--target`
/// stages what `cargo build --release` just produced. CI always passes `--target` explicitly.
#[doc(hidden)] // test boundary (tests/release_assets.rs)
pub fn host_triple() -> Result<String> {
    let output = Command::new("rustc")
        .arg("-vV")
        .output()
        .context("running `rustc -vV` to detect the host triple")?;
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(host) = line.strip_prefix("host: ") {
            return Ok(host.trim().to_string());
        }
    }
    bail!("`rustc -vV` did not report a host triple");
}

/// Copy the built release binary to `<out_dir>/pixi-sandbox-<target>[.exe]`.
///
/// Stripping is best-effort, like the `strip "$BIN" || true` this replaces: the release
/// profile already strips (`strip = true` in the workspace profile), so a missing or failing
/// strip tool must never fail a release — it is a size optimisation, not a correctness step.
pub fn stage_release_binary(
    target: Option<&str>,
    target_dir: &Path,
    out_dir: &Path,
    strip: &str,
) -> Result<PathBuf> {
    let host = host_triple()?;
    let triple = target.map_or_else(|| host.clone(), str::to_string);
    let source = built_binary_path(target_dir, target, &host);
    if !source.is_file() {
        let remedy = match target {
            Some(_) => format!("pixi run build-release-binary -- --target {triple}"),
            None => "pixi run build-release-binary".to_string(),
        };
        bail!(
            "release binary not found at {} — run `{remedy}` first",
            source.display()
        );
    }

    match Command::new(strip).arg(&source).status() {
        Ok(status) if status.success() => {}
        Ok(status) => eprintln!(
            "warning: {strip} exited with {status} on {} — the release profile already strips, continuing",
            source.display()
        ),
        Err(_) => eprintln!(
            "warning: no {strip} on PATH — the release profile already strips, continuing"
        ),
    }

    let staged = out_dir.join(staged_name(&triple));
    fs::copy(&source, &staged)
        .with_context(|| format!("copying {} to {}", source.display(), staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o755))
            .with_context(|| format!("marking {} executable", staged.display()))?;
    }
    let size = fs::metadata(&staged)
        .with_context(|| format!("statting {}", staged.display()))?
        .len();
    println!("staged {} ({size} bytes)", staged.display());
    Ok(staged)
}

/// Top-level standalone release binaries in `dir`: the assets the GitHub Release ships.
///
/// `.conda` files are excluded on purpose — the similarly named conda package
/// (`pixi-sandbox-<version>-<build>_<n>.conda`) has its own prefix.dev/GitHub Release
/// integrity path, and the per-platform `conda-<platform>/` directories hold them, not this
/// listing. The v0.3.7 and earlier workflow checksummed only `*-unknown-*` names, which
/// silently dropped both Apple binaries and the Windows `.exe` from SHA256SUMS.
fn standalone_binaries(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut binaries: Vec<PathBuf> = fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.is_file()
                && path.file_name().is_some_and(|name| {
                    let name = name.to_string_lossy();
                    name.starts_with("pixi-sandbox-") && !name.ends_with(".conda")
                })
        })
        .collect();
    binaries.sort();
    Ok(binaries)
}

/// Write `SHA256SUMS` over the standalone release binaries in `dir`, then verify the written
/// file covers every one of them — the completeness check the shell did with `grep`, kept as
/// a real second pass over the bytes on disk so a bug in the writer cannot pass unnoticed.
pub fn release_checksums(dir: &Path) -> Result<PathBuf> {
    let binaries = standalone_binaries(dir)?;
    if binaries.is_empty() {
        bail!(
            "no standalone release binaries (pixi-sandbox-*) in {} — the build matrix produced nothing to checksum",
            dir.display()
        );
    }
    let mut body = String::new();
    for binary in &binaries {
        let name = binary.file_name().map_or_else(
            || binary.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let digest =
            sha256_file(binary).with_context(|| format!("hashing {}", binary.display()))?;
        body.push_str(&digest);
        body.push_str("  ");
        body.push_str(&name);
        body.push('\n');
    }
    let sums = dir.join(SUMS_NAME);
    fs::write(&sums, &body).with_context(|| format!("writing {}", sums.display()))?;
    verify_completeness(&sums, &binaries)?;
    print!("{body}");
    eprintln!(
        "wrote {} covering {} standalone binaries",
        sums.display(),
        binaries.len()
    );
    Ok(sums)
}

/// Assert every binary has a line in `SHA256SUMS` (`<digest>  <name>`, two spaces), and that
/// the file holds no stale extra lines. Separate from the writer so a fixture can prove the
/// check fires on a tampered file.
pub fn verify_completeness(sums: &Path, binaries: &[PathBuf]) -> Result<()> {
    let body = fs::read_to_string(sums).with_context(|| format!("reading {}", sums.display()))?;
    let covered: Vec<String> = body
        .lines()
        .filter_map(|line| line.split_once("  ").map(|(_, name)| name.to_string()))
        .collect();
    for binary in binaries {
        let name = binary.file_name().map_or_else(
            || binary.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        if !covered.contains(&name) {
            bail!("missing checksum for {name} in {}", sums.display());
        }
    }
    if covered.len() != binaries.len() {
        bail!(
            "{} covers {} file(s) but {} standalone binaries were found — stale line(s)?",
            sums.display(),
            covered.len(),
            binaries.len()
        );
    }
    Ok(())
}
