#![cfg(unix)]

//! The consumer-proof verbs run as the workflow runs them.
//!
//! `release.yml` reaches this code only through `pixi run -e package xtask consumer-proof-<verb>
//! -- …`, so the flag wiring is part of what has to stay true: a subcommand that reads the wrong
//! argument fails on a runner forty minutes into a release, not on a developer's machine. These
//! tests drive the built binary over tempdir fixtures with a stand-in CLI, and touch neither
//! this repository nor the network (ADR-10).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Output};

fn xtask(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(args)
        .output()
        .expect("xtask runs")
}

fn executable(path: &Path, body: &str) {
    fs::write(path, body).expect("write");
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod");
}

/// A stand-in `pixi-sandbox`: reports a version, and generates a publisher carrying both
/// properties the proof asserts.
fn stand_in_cli(dir: &Path) -> String {
    let binary = dir.join("artifact");
    executable(
        &binary,
        r#"#!/bin/sh
case "$1" in
  --version) echo "pixi-sandbox 9.9.9" ;;
  --help) echo "usage: pixi-sandbox [OPTIONS]" ;;
  init)
    mkdir -p .github/workflows
    printf 'pixi-sandbox sha256sum --check selected.sha256 "$BIN"\n# SHA256SUMS\n' > .github/workflows/publish-sandbox.yml
    ;;
esac
"#,
    );
    binary.display().to_string()
}

#[test]
fn the_cli_runs_the_whole_pre_lifecycle_sequence_at_the_paths_it_is_given() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();
    let consumer = root.join("consumer");
    let evidence = root.join("evidence");
    let home = root.join("consumer-home");
    let artifact = stand_in_cli(root);

    let fixture = xtask(&[
        "consumer-proof-fixture",
        "--consumer",
        &consumer.display().to_string(),
        "--evidence",
        &evidence.display().to_string(),
        "--label",
        "v9.9.9",
    ]);
    assert!(
        fixture.status.success(),
        "{}",
        String::from_utf8_lossy(&fixture.stderr)
    );
    assert_eq!(
        fs::read_to_string(consumer.join("README.md")).expect("README"),
        "consumer fixture: v9.9.9\n"
    );

    let dest = consumer.join("pixi-sandbox");
    let adopt = xtask(&[
        "consumer-proof-adopt",
        "--artifact",
        &artifact,
        "--dest",
        &dest.display().to_string(),
    ]);
    assert!(
        adopt.status.success(),
        "{}",
        String::from_utf8_lossy(&adopt.stderr)
    );

    let version = xtask(&[
        "consumer-proof-version",
        "--binary",
        &dest.display().to_string(),
        "--home",
        &home.display().to_string(),
        "--evidence",
        &evidence.display().to_string(),
    ]);
    assert!(
        version.status.success(),
        "{}",
        String::from_utf8_lossy(&version.stderr)
    );
    assert_eq!(
        fs::read_to_string(evidence.join("version.txt")).expect("version"),
        "pixi-sandbox 9.9.9\n",
        "the version step wrote the evidence the workflow uploads"
    );

    let init = xtask(&[
        "consumer-proof-init",
        "--binary",
        &dest.display().to_string(),
        "--consumer",
        &consumer.display().to_string(),
        "--home",
        &home.display().to_string(),
        "--evidence",
        &evidence.display().to_string(),
    ]);
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    assert!(
        evidence.join("publish-sandbox.first.yml").exists()
            && evidence.join("publish-sandbox.yml").exists(),
        "both renders are evidence"
    );
}

#[test]
fn the_cli_refuses_a_lifecycle_whose_fixture_is_missing_and_names_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path();

    let run = xtask(&[
        "consumer-proof-lifecycle",
        "--consumer",
        &root.join("consumer").display().to_string(),
        "--transport",
        &root.join("transport").display().to_string(),
        "--remote",
        &root.join("consumer-proof.git").display().to_string(),
        "--checkout",
        &root.join("fresh-checkout").display().to_string(),
        "--restored",
        &root.join("restored").display().to_string(),
        "--home",
        &root.join("consumer-home").display().to_string(),
        "--evidence",
        &root.join("evidence").display().to_string(),
    ]);

    assert!(!run.status.success(), "the lifecycle must not run");
    let stderr = String::from_utf8_lossy(&run.stderr).to_string();
    assert!(stderr.contains("pixi.toml"), "{stderr}");
    assert!(
        stderr.contains("::error::"),
        "the refusal reaches the run summary as an annotation: {stderr}"
    );
    assert!(
        !root.join("transport").exists() && !root.join("consumer-proof.git").exists(),
        "nothing was created: the refusal came before any tool ran"
    );
}

#[test]
fn the_cli_refuses_to_adopt_an_artifact_the_build_did_not_produce() {
    let dir = tempfile::tempdir().expect("tempdir");

    let run = xtask(&[
        "consumer-proof-adopt",
        "--artifact",
        &dir.path().join("no-such-artifact").display().to_string(),
        "--dest",
        &dir.path().join("pixi-sandbox").display().to_string(),
    ]);

    assert!(!run.status.success(), "a missing artifact is a failure");
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("no-such-artifact"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}
