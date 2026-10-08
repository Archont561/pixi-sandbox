//! `conda_platforms.rs` — one package per platform (moved from the module's inline
//! `#[cfg(test)]`, TASK-83: names kept, bodies verbatim).

use std::fs;
use std::path::Path;
use xtask::conda_platforms::validate;

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
