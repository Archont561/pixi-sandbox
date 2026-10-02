//! Shared integration-test fixtures.
//!
//! This is the only home for the filesystem, process, fixture-payload, and temporary-git setup
//! that more than one integration test needs. Every path starts under a test-owned tempdir and
//! every project payload comes from `tests/fixtures`, never the repository checkout (D10).
//!
//! ## API convention
//!
//! Ask for a zero-argument setup value directly in an `#[rstest]` test (`bin`,
//! `fixture_transport`, `isolated_home`, and `host_platform`). Helpers that need a path or
//! caller-selected argument stay ordinary functions, paired with a `*_fixture` that returns a
//! typed function pointer. This is the rstest-compatible way to expose a parameterised helper:
//! annotating a function such as `copy_tree(source, destination)` directly as a fixture makes
//! rstest interpret those runtime arguments as fixture names. Keep properties bounded, add their
//! committed `.proptest-regressions` file, and use `#[case]` rather than assertion-table loops.

use assert_cmd::Command;
use rstest::fixture;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

#[fixture]
pub fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[fixture]
pub fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

/// An owned home and temporary directory for any restore or launcher test (D10).
#[fixture]
pub fn isolated_home() -> tempfile::TempDir {
    tempfile::tempdir().expect("isolated test home")
}

/// A synthetic, already-packed payload committed for integration tests.
#[fixture]
pub fn fixture_transport() -> PathBuf {
    crate_dir().join("tests/fixtures/transport")
}

/// The complete fixture Pixi project that pack-oriented tests may copy into a tempdir.
#[fixture]
pub fn demo_project() -> PathBuf {
    crate_dir().join("tests/fixtures/demo-project")
}

/// The duplicate-source fixture for the pack-time cargo-vendor rejection.
#[fixture]
pub fn duplicate_source_project() -> PathBuf {
    crate_dir().join("tests/fixtures/duplicate-source-project")
}

/// task-33: a restore must never observe a real user's home or shell profile.
pub fn isolated_bin(home: &Path) -> Command {
    let temporary = home.join("tmp");
    fs::create_dir_all(&temporary).expect("isolated test temporary directory");

    let mut command = bin();
    command
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env("SHELL", "/usr/bin/bash")
        .env("TMPDIR", temporary);
    command
}

/// Copy a fixture out of the checkout before a test writes it, preserving executable bits.
pub fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap().flatten() {
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).unwrap();
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path);
        } else {
            fs::copy(&source_path, &destination_path).unwrap();
            fs::set_permissions(&destination_path, metadata.permissions()).unwrap();
        }
    }
}

/// The host's Pixi platform name, matching the generated launchers and `restore.sh`.
#[fixture]
pub fn host_platform() -> &'static str {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => "linux-64",
        ("linux", "aarch64") => "linux-aarch64",
        ("macos", "aarch64") => "osx-arm64",
        ("macos", "x86_64") => "osx-64",
        ("windows", "x86_64") => "win-64",
        (os, arch) => panic!("unsupported test host {os}-{arch}"),
    }
}

/// Run a git command in a temporary repository and return trimmed stdout.
pub fn git(dir: &Path, args: &[&str]) -> String {
    let output = StdCommand::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_owned()
}

/// The argument order used by the CLI tests: command first, repository second.
pub fn run_git(args: &[&str], cwd: &Path) -> String {
    git(cwd, args)
}

/// Commit a temporary repository with a deterministic test identity.
pub fn commit(dir: &Path, message: &str) {
    git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}

#[cfg(unix)]
pub fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(permissions.mode() | 0o111);
    fs::set_permissions(path, permissions).unwrap();
}

/// Fixtures returning parameterised helpers let tests compose setup declaratively while keeping
/// their call sites explicit about the paths they mutate.
#[fixture]
pub fn isolated_bin_fixture() -> fn(&Path) -> Command {
    isolated_bin
}

#[fixture]
pub fn copy_tree_fixture() -> fn(&Path, &Path) {
    copy_tree
}

#[fixture]
pub fn git_fixture() -> fn(&Path, &[&str]) -> String {
    git
}

#[fixture]
pub fn run_git_fixture() -> fn(&[&str], &Path) -> String {
    run_git
}

#[fixture]
pub fn commit_fixture() -> fn(&Path, &str) {
    commit
}

#[cfg(unix)]
#[fixture]
pub fn make_executable_fixture() -> fn(&Path) {
    make_executable
}

/// Which packed bootstrap the script-based restore tests should simulate.
#[cfg(unix)]
#[allow(dead_code)] // This module is compiled into every integration-test binary; only restore_script uses it.
#[derive(Clone, Copy)]
pub enum Bundled {
    /// 0.3.7 or newer: honours `PIXI_SANDBOX_USER_TOOLS`.
    Current,
    /// A branch packed before 0.3.7: it does not understand the registration policy.
    PreUserTools,
}

/// A temporary repository whose named branch carries the fixture transport and a shim around the
/// binary this test built. The checkout is reset to a developer-looking project after committing
/// the payload branch, matching a real airlock repository.
#[cfg(unix)]
pub fn transport_repo(root: &Path, branch: &str, bundled: Bundled) -> PathBuf {
    fs::create_dir_all(root).unwrap();
    copy_tree(&fixture_transport(), root);

    let real = assert_cmd::cargo::cargo_bin("pixi-sandbox");
    let tool = root.join(".pixi-sandbox/tools/linux-64/pixi-sandbox");
    let shim = match bundled {
        Bundled::Current => format!("#!/bin/sh\nexec \"{}\" \"$@\"\n", real.display()),
        Bundled::PreUserTools => format!(
            r#"#!/bin/sh
# Plays a bootstrap packed before 0.3.7: it reports that version, and because it has no
# --user-tools it registers nothing however loudly the caller asks (an unknown environment
# variable is ignored, which is the whole reason the policy travels as one).
if [ "$1" = "--version" ]; then echo "pixi-sandbox 0.3.6"; exit 0; fi
case " $* " in
  *" restore "*) exec "{real}" "$@" --user-tools skip ;;
  *) exec "{real}" "$@" ;;
esac
"#,
            real = real.display()
        ),
    };
    fs::write(&tool, shim).unwrap();
    make_executable(&tool);

    // The fixture manifest verifies tool size before restore writes anything, so its declared
    // size follows the shim while every other fixture byte stays unchanged.
    let manifest_path = root.join(".pixi-sandbox/manifest.json");
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest["tools"]["pixi-sandbox"]["size_bytes"] =
        serde_json::json!(fs::metadata(&tool).unwrap().len());
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).unwrap(),
    )
    .unwrap();

    git(root, &["init", "-q", "-b", "main"]);
    git(root, &["add", "."]);
    commit(root, "payload");
    git(root, &["branch", branch]);

    git(root, &["rm", "-qrf", "."]);
    fs::write(root.join("README.md"), "the developer's checkout\n").unwrap();
    git(root, &["add", "."]);
    commit(root, "main");

    root.to_path_buf()
}

#[cfg(unix)]
#[fixture]
pub fn transport_repo_fixture() -> fn(&Path, &str, Bundled) -> PathBuf {
    transport_repo
}
