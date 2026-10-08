//! A minimal repository that passes every check, shared by every check module's own tests.
//! Each test then breaks exactly one policy and asserts that policy alone fires. Built in a
//! tempdir: D10 forbids these tests from ever looking at the real checkout. (Moved from
//! `repo_checks/test_support.rs`, TASK-83.)

use std::fs;
use std::path::Path;
use xtask::repo_checks::check_repository;
use xtask::workflow::{RELOCK_PATH, relock_render};

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

pub fn headlines(root: &Path) -> Vec<String> {
    check_repository(root)
        .expect("checks run")
        .into_iter()
        .map(|f| f.headline)
        .collect()
}
