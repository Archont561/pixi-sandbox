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

const SUMS_NAME: &str = "SHA256SUMS";

/// Asset name of a staged binary: `pixi-sandbox-<target>`, with the `.exe` suffix Windows
/// targets require. Derived from the triple so the naming is testable without a Windows host.
fn staged_name(target: &str) -> String {
    if target.contains("windows") {
        format!("pixi-sandbox-{target}.exe")
    } else {
        format!("pixi-sandbox-{target}")
    }
}

/// The binary name cargo produces (`pixi-sandbox`, `pixi-sandbox.exe` on Windows targets).
fn binary_name(target: &str) -> String {
    if target.contains("windows") {
        "pixi-sandbox.exe".to_string()
    } else {
        "pixi-sandbox".to_string()
    }
}

/// Where cargo puts the release binary: `target/<triple>/release/` when `--target` was
/// passed, `target/release/` for a host build — the host triple *names* the binary but does
/// not nest it, which is exactly the difference the staging has to honour.
fn built_binary_path(target_dir: &Path, target: Option<&str>, host_triple: &str) -> PathBuf {
    let release = match target {
        Some(triple) => target_dir.join(triple).join("release"),
        None => target_dir.join("release"),
    };
    release.join(binary_name(target.unwrap_or(host_triple)))
}

/// The host triple from `rustc -vV`, so a local `stage-release-binary` with no `--target`
/// stages what `cargo build --release` just produced. CI always passes `--target` explicitly.
fn host_triple() -> Result<String> {
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
    let triple = target.map(str::to_string).unwrap_or_else(|| host.clone());
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
        let name = binary
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| binary.display().to_string());
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
        let name = binary
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| binary.display().to_string());
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, bytes).expect("write");
    }

    #[test]
    fn staged_names_carry_the_exe_suffix_only_for_windows_targets() {
        assert_eq!(
            staged_name("x86_64-unknown-linux-musl"),
            "pixi-sandbox-x86_64-unknown-linux-musl"
        );
        assert_eq!(
            staged_name("aarch64-apple-darwin"),
            "pixi-sandbox-aarch64-apple-darwin"
        );
        assert_eq!(
            staged_name("x86_64-pc-windows-msvc"),
            "pixi-sandbox-x86_64-pc-windows-msvc.exe"
        );
    }

    /// `staged_name` stays pure over *any* target triple on purpose (D10: testable without a
    /// Windows host, and tolerant of a target this project does not yet publish for). It is
    /// deliberately not migrated onto `Platform` (task-55) the way `conda_platforms.rs` and
    /// `airlock.rs` were (task-59) — doing so would narrow it to the five known platforms. This
    /// test is the structural guarantee that keeps the two independent implementations from
    /// drifting apart for the platforms they do share: run it instead of a doc comment's promise.
    #[test]
    fn staged_name_agrees_with_platform_asset_name_for_every_known_platform() {
        use pixi_sandbox_core::platform::Platform;

        for platform in Platform::ALL {
            assert_eq!(
                staged_name(platform.target_triple()),
                platform.asset_name(),
                "staged_name({:?}) must agree with Platform::asset_name for {platform:?}",
                platform.target_triple()
            );
        }
    }

    #[test]
    fn built_binaries_nest_the_triple_directory_only_for_explicit_targets() {
        assert_eq!(
            built_binary_path(
                Path::new("target"),
                Some("x86_64-unknown-linux-musl"),
                "irrelevant"
            ),
            PathBuf::from("target/x86_64-unknown-linux-musl/release/pixi-sandbox")
        );
        assert_eq!(
            built_binary_path(
                Path::new("target"),
                Some("x86_64-pc-windows-msvc"),
                "irrelevant"
            ),
            PathBuf::from("target/x86_64-pc-windows-msvc/release/pixi-sandbox.exe")
        );
        // A host build: the triple names the file (exe on a Windows host) but cargo puts it
        // straight under target/release/, with no triple directory.
        assert_eq!(
            built_binary_path(Path::new("target"), None, "x86_64-pc-windows-msvc"),
            PathBuf::from("target/release/pixi-sandbox.exe")
        );
        assert_eq!(
            built_binary_path(Path::new("target"), None, "x86_64-unknown-linux-gnu"),
            PathBuf::from("target/release/pixi-sandbox")
        );
    }

    #[test]
    fn a_host_build_stages_from_target_release_under_the_host_asset_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target_dir = dir.path().join("target");
        write(
            &target_dir.join("release/pixi-sandbox"),
            b"host build payload",
        );
        let out = dir.path().join("out");
        fs::create_dir_all(&out).expect("mkdir");

        let staged = stage_release_binary(None, &target_dir, &out, "true").expect("host staging");

        let expected_name = format!("pixi-sandbox-{}", host_triple().expect("rustc"));
        assert_eq!(staged, out.join(expected_name));
        assert_eq!(fs::read(&staged).expect("read"), b"host build payload");
    }

    #[test]
    fn staging_copies_under_the_asset_name_and_keeps_it_executable() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target_dir = dir.path().join("target");
        write(
            &target_dir.join("x86_64-unknown-linux-musl/release/pixi-sandbox"),
            b"#!/bin/true\nfake ELF payload",
        );
        let out = dir.path().join("out");
        fs::create_dir_all(&out).expect("mkdir");

        let staged =
            stage_release_binary(Some("x86_64-unknown-linux-musl"), &target_dir, &out, "true")
                .expect("staging succeeds");

        assert_eq!(staged, out.join("pixi-sandbox-x86_64-unknown-linux-musl"));
        assert_eq!(
            fs::read(&staged).expect("read"),
            b"#!/bin/true\nfake ELF payload"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&staged).expect("stat").permissions().mode();
            assert_ne!(mode & 0o111, 0, "a staged binary must stay executable");
        }
    }

    #[test]
    fn a_failing_strip_is_not_fatal_because_the_release_profile_already_strips() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target_dir = dir.path().join("target");
        write(
            &target_dir.join("aarch64-apple-darwin/release/pixi-sandbox"),
            b"mach-o payload",
        );
        let staged = stage_release_binary(
            Some("aarch64-apple-darwin"),
            &target_dir,
            dir.path(),
            "false", // exits non-zero: strip must be advisory, never fatal
        )
        .expect("a failing strip must not fail the release");
        assert!(staged.is_file());
    }

    #[test]
    fn a_missing_binary_is_an_error_naming_the_path_and_the_remedy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = stage_release_binary(
            Some("aarch64-apple-darwin"),
            &dir.path().join("target"),
            dir.path(),
            "true",
        )
        .expect_err("nothing was built");
        let message = format!("{error:#}");
        assert!(
            message.contains("release binary not found"),
            "must say what is missing: {message}"
        );
        assert!(
            message.contains("aarch64-apple-darwin"),
            "must name the target: {message}"
        );
        assert!(
            message.contains("build-release-binary"),
            "must name the remedy: {message}"
        );
    }

    #[test]
    fn checksums_cover_every_platform_including_the_ones_the_old_glob_dropped() {
        let dir = tempfile::tempdir().expect("tempdir");
        // All five release platforms, exactly the asset set the GitHub Release uploads — the
        // old `*-unknown-*` glob checksummed only the two musl lines of this set.
        for (name, bytes) in [
            (
                "pixi-sandbox-x86_64-unknown-linux-musl",
                b"linux x86_64" as &[u8],
            ),
            ("pixi-sandbox-aarch64-unknown-linux-musl", b"linux aarch64"),
            ("pixi-sandbox-x86_64-apple-darwin", b"darwin x86_64"),
            ("pixi-sandbox-aarch64-apple-darwin", b"darwin aarch64"),
            ("pixi-sandbox-x86_64-pc-windows-msvc.exe", b"windows"),
        ] {
            write(&dir.path().join(name), bytes);
        }

        let sums = release_checksums(dir.path()).expect("checksums succeed");

        let body = fs::read_to_string(&sums).expect("read sums");
        let lines: Vec<&str> = body.lines().collect();
        assert_eq!(lines.len(), 5, "every platform binary gets a line");
        let names: Vec<&str> = lines
            .iter()
            .map(|line| line.split_once("  ").expect("sha256sum line format").1)
            .collect();
        let mut sorted = names.clone();
        sorted.sort_unstable();
        assert_eq!(names, sorted, "lines must be sorted by asset name");
        for line in lines {
            let (digest, name) = line
                .split_once("  ")
                .expect("sha256sum line format: two spaces");
            assert_eq!(digest.len(), 64, "lowercase hex sha256: {digest}");
            let expected = sha256_file(&dir.path().join(name)).expect("digest of the same file");
            assert_eq!(digest, expected, "digest must match the file bytes");
        }
    }

    #[test]
    fn conda_packages_are_not_checksummer() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            &dir.path().join("pixi-sandbox-0.3.7-h4778241_0.conda"),
            b"conda payload",
        );
        write(
            &dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl"),
            b"binary",
        );

        let sums = release_checksums(dir.path()).expect("checksums succeed");

        let body = fs::read_to_string(&sums).expect("read sums");
        assert_eq!(body.lines().count(), 1, "only the standalone binary");
        assert!(
            body.contains("pixi-sandbox-x86_64-unknown-linux-musl"),
            "the binary is the one covered: {body}"
        );
    }

    #[test]
    fn an_empty_directory_is_an_error_not_an_empty_sums_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let error = release_checksums(dir.path()).expect_err("nothing to checksum");
        let message = format!("{error:#}");
        assert!(
            message.contains("no standalone release binaries"),
            "must say the matrix produced nothing: {message}"
        );
        assert!(
            !dir.path().join(SUMS_NAME).exists(),
            "the shell this replaced wrote an empty file and exited 0"
        );
    }

    #[test]
    fn the_completeness_check_names_the_binary_a_tampered_sums_file_omits() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = dir.path().join("pixi-sandbox-x86_64-apple-darwin");
        let second = dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl");
        write(&first, b"darwin");
        write(&second, b"linux");
        let sums = dir.path().join(SUMS_NAME);
        fs::write(&sums, "0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-x86_64-apple-darwin\n")
            .expect("write sums");

        let error = verify_completeness(&sums, &[first, second]).expect_err("one is missing");
        let message = format!("{error:#}");
        assert!(
            message.contains("pixi-sandbox-x86_64-unknown-linux-musl"),
            "must name the uncovered binary: {message}"
        );
        assert!(
            message.contains("missing checksum"),
            "must say what is wrong: {message}"
        );
    }

    #[test]
    fn stale_extra_lines_in_the_sums_file_fail_the_completeness_check() {
        let dir = tempfile::tempdir().expect("tempdir");
        let binary = dir.path().join("pixi-sandbox-x86_64-apple-darwin");
        write(&binary, b"darwin");
        let sums = dir.path().join(SUMS_NAME);
        fs::write(
            &sums,
            "0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-x86_64-apple-darwin\n\
             0000000000000000000000000000000000000000000000000000000000000000  pixi-sandbox-deleted-target\n",
        )
        .expect("write sums");

        let error = verify_completeness(&sums, &[binary]).expect_err("stale line");
        assert!(
            format!("{error:#}").contains("stale line"),
            "must flag the surplus: {error:#}"
        );
    }
}
