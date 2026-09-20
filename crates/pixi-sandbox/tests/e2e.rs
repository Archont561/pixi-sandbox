//! Full end-to-end integration tests for pixi-sandbox airlock lifecycle.
//!
//! Replaces external bash/python proof scripts with native Rust tests under `cargo nextest`.

#![cfg(unix)]

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;

fn bin() -> Command {
    Command::cargo_bin("pixi-sandbox").expect("binary builds")
}

fn write_executable(path: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;

    fs::write(path, script).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn setup_mock_tools(dir: &Path) {
    fs::create_dir_all(dir).unwrap();
    write_executable(
        &dir.join("pixi-pack"),
        r#"#!/bin/sh
set -eu
out=''
env=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -e) env="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "$out/channel/noarch"
printf '{"version":"fake"}\n' > "$out/pixi-pack.json"
printf '# fake package for %s\n' "$env" > "$out/channel/noarch/$env-0.1.0-0.conda"
printf 'unpacked\n'
"#,
    );
    write_executable(
        &dir.join("pixi"),
        r#"#!/bin/sh
if [ "${1:-}" = '--version' ]; then echo 'pixi 0.81.0'; else echo 'pixi fake'; fi
"#,
    );
    write_executable(
        &dir.join("pixi-unpack"),
        r#"#!/bin/sh
set -eu
if [ "${1:-}" = '--version' ]; then echo 'pixi-unpack 0.7.11'; exit 0; fi
pack="${1:-}"
out=''
env=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o) out="$2"; shift 2 ;;
    -e) env="$2"; shift 2 ;;
    *) shift ;;
  esac
done
mkdir -p "$out/$env/conda-meta"
printf 'unpacked %s\n' "$env" > "$out/$env/conda-meta/fake-package.json"
"#,
    );
}

fn path_with_tools(dir: &Path) -> std::ffi::OsString {
    let mut paths = vec![dir.to_path_buf()];
    paths.extend(std::env::split_paths(
        &std::env::var_os("PATH").unwrap_or_default(),
    ));
    std::env::join_paths(paths).unwrap()
}

#[test]
fn e2e_synthetic_pack_doctor_publish_and_restore() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();

    let project = root.join("demo-project");
    let tools = root.join("tools");
    let transport = root.join("transport");
    let remote_git = root.join("remote.git");
    let airlock = root.join("airlock-project");

    fs::create_dir_all(&project).unwrap();
    fs::write(
        project.join("pixi.toml"),
        "[workspace]\nname = \"demo\"\nplatforms = [\"linux-64\"]\n",
    )
    .unwrap();
    fs::write(project.join("pixi.lock"), "version: 6\n").unwrap();

    setup_mock_tools(&tools);
    let path = path_with_tools(&tools);

    // 1. Pack
    bin()
        .env("PATH", &path)
        .args([
            "pack",
            "--repo-root",
            project.to_str().unwrap(),
            "--envs",
            "default",
            "--output-dir",
            transport.to_str().unwrap(),
            "--platform",
            "linux-64",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("packed transport"));

    // 2. Doctor verify
    bin()
        .args([
            "doctor",
            "--branch-location",
            transport.to_str().unwrap(),
            "--verify",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("OK"));

    // 3. Publish to local bare repo
    let mut git_init = StdCommand::new("git");
    git_init.args(["init", "-q", "--bare"]);
    git_init.arg(&remote_git);
    assert!(git_init.status().expect("git init succeeds").success());

    bin()
        .args([
            "publish",
            "--input-dir",
            transport.to_str().unwrap(),
            "--branch-name",
            "sandbox/default-linux-64",
            "--remote",
            remote_git.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("published"));

    // 4. Restore
    fs::create_dir_all(&airlock).unwrap();
    bin()
        .args([
            "restore",
            "--branch-location",
            transport.to_str().unwrap(),
            "--path-to-main-repo-code",
            airlock.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("restore complete"));

    // 5. Assertions on airlock filesystem
    let prefix_marker = airlock
        .join(".pixi")
        .join("envs")
        .join("default")
        .join("conda-meta")
        .join("pixi_env_prefix");
    assert!(prefix_marker.is_file(), "pixi_env_prefix must be written");

    let env_script = airlock.join(".pixi").join("sandbox-env.sh");
    assert!(env_script.is_file(), "sandbox-env.sh must be written");
}

#[test]
#[cfg(target_os = "linux")]
fn e2e_network_isolated_restore_proof() {
    // Check if `unshare -rn` is supported on this system
    let unshare_check = StdCommand::new("unshare").args(["-rn", "true"]).status();

    if unshare_check.map_or(false, |s| s.success()) {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();

        let tools = root.join("tools");
        let project = root.join("demo-project");
        let transport = root.join("transport");
        let airlock = root.join("airlock-project");

        fs::create_dir_all(&project).unwrap();
        fs::write(
            project.join("pixi.toml"),
            "[workspace]\nname = \"demo\"\nplatforms = [\"linux-64\"]\n",
        )
        .unwrap();
        fs::write(project.join("pixi.lock"), "version: 6\n").unwrap();

        setup_mock_tools(&tools);
        let path = path_with_tools(&tools);

        bin()
            .env("PATH", &path)
            .args([
                "pack",
                "--repo-root",
                project.to_str().unwrap(),
                "--envs",
                "default",
                "--output-dir",
                transport.to_str().unwrap(),
                "--platform",
                "linux-64",
            ])
            .assert()
            .success();

        fs::create_dir_all(&airlock).unwrap();

        // Run restore in a severed network namespace
        let pixi_sandbox_bin = assert_cmd::cargo::cargo_bin("pixi-sandbox");
        let status = StdCommand::new("unshare")
            .args([
                "-rn",
                pixi_sandbox_bin.to_str().unwrap(),
                "restore",
                "--branch-location",
                transport.to_str().unwrap(),
                "--path-to-main-repo-code",
                airlock.to_str().unwrap(),
            ])
            .status()
            .expect("unshare runs");

        assert!(status.success(), "offline restore must succeed in unshare -rn");
        assert!(
            airlock
                .join(".pixi/envs/default/conda-meta/pixi_env_prefix")
                .is_file()
        );
    }
}
