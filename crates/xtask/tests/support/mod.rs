//! A minimal repository that passes every check, shared by every check module's own tests.
//! Each test then breaks exactly one policy and asserts that policy alone fires. Built in a
//! tempdir: D10 forbids these tests from ever looking at the real checkout. (Moved from
//! `repo_checks/test_support.rs`, TASK-83.)

use std::fs;
use std::path::Path;
use xtask::repo_checks::check_repository;
use xtask::starter::scaffold;
use xtask::workflow::{RELOCK_PATH, relock_render};

// Compiled into every check module's test binary; not every one uses both helpers.
#[allow(dead_code)]
pub fn valid_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let write = |rel: &str, text: &str| {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
        fs::write(path, text).expect("file");
    };

    write("Cargo.toml", "[workspace.package]\nversion = \"1.0.0\"\n");
    write(
        "pixi.toml",
        "[workspace]\nname = \"fixture\"\nplatforms = [\"linux-64\", \"osx-arm64\"]\n",
    );
    write(
        ".pixi-sandbox.toml",
        "schema = 1\n\n[[bundle]]\nname = \"developer\"\nplatforms = [\"linux-64\"]\n",
    );
    write(
        "README.md",
        "<img src=\"https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg\" alt=\"Platforms\">\n\nwin-64 is unsupported: conda-forge ships no bun build (D11).\n\npixi global install --channel https://prefix.dev/archont561/archont561 --channel conda-forge pixi-sandbox\npixi-sandbox init\n",
    );
    write(
        "crates/pixi-sandbox/pixi.toml",
        "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\n",
    );
    write("crates/pixi-sandbox/src/lib.rs", "// clean\n");
    write(
        "docs/src/content/docs/installation.mdx",
        "pixi global install --channel https://prefix.dev/archont561/archont561 --channel conda-forge pixi-sandbox\npixi-sandbox init\n",
    );
    write(
        "crates/pixi-sandbox/src/generated/github_workflow.rs",
        "https://prefix.dev/archont561/archont561\npixi global install pixi-sandbox\n",
    );
    write(
        ".github/workflows/ci.yml",
        "jobs:\n  ci:\n    steps:\n      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n",
    );
    write(
        "scripts/restore.sh",
        "#!/usr/bin/env bash\nset -euo pipefail\necho restore\n",
    );
    write(
        RELOCK_PATH,
        &relock_render(root).expect("render the relock workflow"),
    );

    dir
}

// Compiled into every check module's test binary; not every one uses both helpers.
#[allow(dead_code)]
pub fn headlines(root: &Path) -> Vec<String> {
    check_repository(root)
        .expect("checks run")
        .into_iter()
        .map(|f| f.headline)
        .collect()
}

/// Run real `git` in a temporary directory with a deterministic test identity.
///
/// Tests here drive the repository's own `git` only to *build the world under test* — the
/// carve-out invariant 8 gives fixture builders, so an oracle never shares code with the thing
/// it judges. Nothing in these tests reads `pixi-sandbox` itself (D10).
#[allow(dead_code)]
pub fn run_git(cwd: &Path, args: &[&str]) -> String {
    let identity = [
        "-c",
        "user.name=test",
        "-c",
        "user.email=test@example.invalid",
    ];
    let mut argv: Vec<&str> = identity.to_vec();
    argv.extend_from_slice(args);
    let out = std::process::Command::new("git")
        .args(&argv)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {argv:?} failed in {}: {}",
        cwd.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Stage everything `git` would take and commit it — the way the starter lane's own publish step
/// does, so a test's "previous revision" is not a fiction.
#[allow(dead_code)]
pub fn commit_all(cwd: &Path, message: &str) {
    run_git(cwd, &["add", "-A"]);
    run_git(cwd, &["commit", "-qm", message]);
}

/// How many commits `HEAD` can see, as a string because that is what `git` prints.
#[allow(dead_code)]
pub fn commit_count(cwd: &Path) -> String {
    run_git(cwd, &["rev-list", "--count", "HEAD"])
}

// --------------------------------------------------------------------- the starter lane's fixture

/// The starter tag and commits the lane's tests hand to one another.
#[allow(dead_code)]
pub const STARTER_TAG: &str = "v1.2.3";
#[allow(dead_code)]
pub const STARTER_COMMIT: &str = "8193c58068c3245e5267c4c604f989237e4d6e49";
#[allow(dead_code)]
pub const STARTER_SOURCE: &str = "Archont561/pixi-sandbox";
#[allow(dead_code)]
pub const STARTER_REPO: &str = "Archont561/pixi-sandbox-starter";

/// A starter tree as the lane's scaffold + generate + lock steps leave it, with the publisher
/// carrying `workflow_version` in both places the verifier reads.
///
/// Shared because three test binaries need the same tree and only one of them may decide what a
/// valid one is: the marker line and the version pin are the contract, and a fixture that drifts
/// from it tests the fixture.
#[allow(dead_code)]
pub fn starter_tree(dir: &Path, workflow_version: &str) {
    scaffold(
        dir,
        "starter",
        STARTER_TAG,
        STARTER_COMMIT,
        STARTER_SOURCE,
        STARTER_REPO,
    )
    .expect("scaffold");
    fs::create_dir_all(dir.join(".github/workflows")).expect("mkdir");
    fs::write(dir.join("pixi.lock"), "version: 6\n").expect("write");
    fs::write(dir.join("pixi-sandbox.toml"), "schema = 1\n").expect("write");
    fs::write(
        dir.join(".github/workflows/publish-sandbox.yml"),
        format!(
            "# Generated by pixi-sandbox init\n\
             # pixi-sandbox-version: {workflow_version}\n\
             name: publish sandbox\n\
             env:\n  PIXI_SANDBOX_VERSION: {workflow_version}\n\
             jobs:\n  x:\n    runs-on: ubuntu-latest\n"
        ),
    )
    .expect("write");
    fs::write(dir.join(".github/workflows/relock.yml"), "name: relock\n").expect("write");
    fs::write(dir.join("restore.sh"), "echo restore\n").expect("write");
}

/// The runtime state `pixi lock` and `pixi run dev` necessarily leave in the assembled directory:
/// bytes on disk that the starter's own `.gitignore` keeps out of the revision (task-85).
#[allow(dead_code)]
pub fn runtime_state(dir: &Path) {
    fs::create_dir_all(dir.join(".pixi/envs/default")).expect("mkdir");
    fs::write(dir.join(".pixi/envs/default/rg"), "installed by pixi run\n").expect("write");
    fs::create_dir_all(dir.join(".pixi-sandbox")).expect("mkdir");
    fs::write(dir.join(".pixi-sandbox/manifest.json"), "{}\n").expect("write");
}
