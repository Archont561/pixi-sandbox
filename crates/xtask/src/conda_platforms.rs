//! Fail the release unless every supported platform contributed exactly one conda package.
//!
//! task-23 AC#4, ported from the retired shell gate (task-28). The release build matrix runs
//! with `fail-fast: false` so one broken platform does not cancel the other four and hide
//! which one broke. That makes the *release* job the only place that can still turn a missing
//! platform into a red build: without this check, a runner that produced no package (an
//! unsolvable manifest, a silently skipped step) would let the job upload four packages and
//! attach a GitHub Release that looks complete.
//!
//! The layout this reads is `dir/conda-<platform>/*.conda`, which is what
//! `actions/download-artifact` with `pattern: conda-*` and no `merge-multiple` produces. The
//! five packages carry the *same* filename, so the per-platform directory is the only thing
//! that tells them apart — a flat merge would overwrite four of them and this check would
//! pass on one platform's bytes.

use anyhow::{Result, bail};
use std::fmt::Write as _;
use std::path::Path;

/// The release platform set, matching the `release.yml` build matrix. The checker itself takes
/// the list as an argument so a fixture (or a future platform change) never has to edit the
/// validation logic.
pub const SUPPORTED_PLATFORMS: [&str; 5] =
    ["linux-64", "linux-aarch64", "osx-64", "osx-arm64", "win-64"];

/// Validate the artifact root. Returns the per-platform ok lines for logging; every defect is
/// a returned diagnostic rather than an early exit, so one run names every broken platform.
pub fn validate(dir: &Path, platforms: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut ok = Vec::new();
    let mut diagnostics = Vec::new();

    if !dir.is_dir() {
        diagnostics.push(format!(
            "{} does not exist — the conda package artifacts were not downloaded",
            dir.display()
        ));
        return (ok, diagnostics);
    }

    let mut missing = Vec::new();
    for platform in platforms {
        let platform_dir = dir.join(format!("conda-{platform}"));
        let packages = conda_files(&platform_dir);
        match packages.len() {
            0 => missing.push(*platform),
            1 => ok.push(format!("ok: {platform} -> {}", packages[0])),
            n => diagnostics.push(format!(
                "conda-{platform} holds {n} packages, expected exactly 1"
            )),
        }
    }
    if !missing.is_empty() {
        let mut msg = format!("no conda package for: {}", missing.join(" "));
        let _ = write!(
            msg,
            "\n  every supported platform must build and smoke-test its own package (task-23);\
             \n  read the failing platform's build job for why it produced nothing"
        );
        diagnostics.push(msg);
    }

    // A package outside the per-platform directories is a layout the download step never
    // produces, so it means something merged or misplaced artifacts — and because all five
    // files share a name, a misplaced one is indistinguishable from the right one by name.
    for entry in walkdir::WalkDir::new(dir).into_iter().flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "conda") {
            let in_platform_dir = path.parent().and_then(|p| p.file_name()).is_some_and(|d| {
                let d = d.to_string_lossy();
                platforms
                    .iter()
                    .any(|platform| d == format!("conda-{platform}"))
            });
            if !in_platform_dir {
                diagnostics.push(format!(
                    "{} sits outside the conda-<platform> directories — the artifacts were merged or misplaced, and identically-named packages cannot be told apart",
                    path.display()
                ));
            }
        }
    }

    (ok, diagnostics)
}

/// `xtask check-conda-platforms <dir>`: GitHub-annotated adapter over [`validate`].
pub fn check(dir: &Path) -> Result<()> {
    let platforms: Vec<&str> = SUPPORTED_PLATFORMS.to_vec();
    let (ok, diagnostics) = validate(dir, &platforms);
    for line in &ok {
        eprintln!("{line}");
    }
    if !diagnostics.is_empty() {
        for diagnostic in &diagnostics {
            eprintln!("::error::{diagnostic}");
        }
        bail!("{} platform package problem(s)", diagnostics.len());
    }
    eprintln!("all {} platform packages present", ok.len());
    Ok(())
}

fn conda_files(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_file())
        .filter(|e| e.path().extension().is_some_and(|x| x == "conda"))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::validate;
    use std::fs;
    use std::path::Path;

    const PLATFORMS: [&str; 2] = ["linux-64", "osx-arm64"];

    fn place(root: &Path, platform_dir: &str, name: &str) {
        let dir = root.join(platform_dir);
        fs::create_dir_all(&dir).expect("platform dir");
        fs::write(dir.join(name), b"conda bytes").expect("package");
    }

    #[test]
    fn one_package_per_platform_passes() {
        let dir = tempfile::tempdir().expect("tempdir");
        place(dir.path(), "conda-linux-64", "p-1.0.0-h_0.conda");
        place(dir.path(), "conda-osx-arm64", "p-1.0.0-h_0.conda");
        let (ok, diagnostics) = validate(dir.path(), &PLATFORMS);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(ok.len(), 2);
    }

    #[test]
    fn a_missing_platform_is_named() {
        let dir = tempfile::tempdir().expect("tempdir");
        place(dir.path(), "conda-linux-64", "p-1.0.0-h_0.conda");
        let (_, diagnostics) = validate(dir.path(), &PLATFORMS);
        assert!(
            diagnostics.iter().any(|d| d.contains("osx-arm64")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn a_duplicate_package_fails_that_platform() {
        let dir = tempfile::tempdir().expect("tempdir");
        place(dir.path(), "conda-linux-64", "a.conda");
        place(dir.path(), "conda-linux-64", "b.conda");
        place(dir.path(), "conda-osx-arm64", "p.conda");
        let (_, diagnostics) = validate(dir.path(), &PLATFORMS);
        assert!(
            diagnostics.iter().any(|d| d.contains("holds 2 packages")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn an_absent_root_is_reported_as_undownloaded_artifacts() {
        let dir = tempfile::tempdir().expect("tempdir");
        let (_, diagnostics) = validate(&dir.path().join("nope"), &PLATFORMS);
        assert!(diagnostics[0].contains("not downloaded"), "{diagnostics:?}");
    }

    #[test]
    fn a_package_outside_the_platform_directories_fails() {
        let dir = tempfile::tempdir().expect("tempdir");
        place(dir.path(), "conda-linux-64", "p.conda");
        place(dir.path(), "conda-osx-arm64", "p.conda");
        place(dir.path(), "merged", "p.conda");
        let (_, diagnostics) = validate(dir.path(), &PLATFORMS);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.contains("outside the conda-<platform>")),
            "{diagnostics:?}"
        );
    }
}
