//! Full fixture-backed integration tests for the offline lifecycle.
//!
//! These tests are the cold proof: they use the committed transport fixture, real CLI commands,
//! local Git only, and (where the runner permits it) a severed network namespace.

#![cfg(unix)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

fn fixture_transport() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/transport")
}

fn copy_tree(source: &Path, destination: &Path) {
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

#[test]
fn fixture_doctor_publish_and_restore_is_the_complete_offline_proof() {
    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let remote = temp.path().join("remote.git");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK — every declared byte matches"));

    assert!(
        StdCommand::new("git")
            .args(["init", "-q", "--bare"])
            .arg(&remote)
            .status()
            .unwrap()
            .success()
    );
    bin()
        .args([
            "publish",
            "--input-dir",
            transport.to_str().unwrap(),
            "--branch-name",
            "sandbox/demo-linux-64",
            "--remote",
            remote.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("published"));

    bin()
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("relocated 1 text file(s)"))
        .stdout(predicate::str::contains("restore complete"));

    let prefix = airlock.join(".pixi/envs/demo");
    assert!(prefix.join("conda-meta/fake-package.json").is_file());
    assert!(prefix.join("conda-meta/pixi_env_prefix").is_file());
    let pkg_config = fs::read_to_string(prefix.join("lib/pkgconfig/fixture.pc")).unwrap();
    assert!(pkg_config.contains(prefix.to_str().unwrap()));
    assert!(!pkg_config.contains(".restore-work/stage-demo"));
    assert!(
        airlock
            .join(".pixi-sandbox/vendor/demo-dep-1.0.0/Cargo.toml")
            .is_file()
    );
    assert!(airlock.join(".cargo/config.toml").is_file());
}

#[test]
#[cfg(target_os = "linux")]
fn fixture_restore_succeeds_in_a_severed_network_namespace() {
    if !StdCommand::new("unshare")
        .args(["-rn", "true"])
        .status()
        .is_ok_and(|status| status.success())
    {
        return;
    }

    let temp = tempfile::tempdir().unwrap();
    let transport = temp.path().join("transport");
    let airlock = temp.path().join("airlock-project");
    copy_tree(&fixture_transport(), &transport);
    fs::create_dir_all(&airlock).unwrap();

    let status = StdCommand::new("unshare")
        .args([
            "-rn",
            assert_cmd::cargo::cargo_bin("pixi-sandbox")
                .to_str()
                .unwrap(),
        ])
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--output-path",
            airlock.to_str().unwrap(),
        ])
        .status()
        .expect("unshare runs");

    assert!(status.success(), "fixture restore must not need a network");
    assert!(
        airlock
            .join(".pixi/envs/demo/conda-meta/pixi_env_prefix")
            .is_file()
    );
}
