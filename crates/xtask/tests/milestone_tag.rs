//! `milestone_tag.rs` — the `resolve-milestone-tag` CLI surface that powers the auto-release
//! trigger of doc-18 §5.2. Black-box: the binary is the contract the workflow YAML depends on.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn xtask() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_xtask"));
    // ADR-10: never let the host environment leak into a black-box test.
    cmd.env_clear();
    if let Some(path) = env::var_os("PATH") {
        cmd.env("PATH", path);
    }
    if let Some(home) = env::var_os("HOME") {
        cmd.env("HOME", home);
    }
    if let Some(profile) = env::var_os("USERPROFILE") {
        cmd.env("USERPROFILE", profile);
    }
    cmd
}

#[test]
fn a_milestone_tag_prints_and_emits_the_release_selector() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outputs = dir.path().join("github-output");

    let output = xtask()
        .env("GITHUB_OUTPUT", &outputs)
        .args(["resolve-milestone-tag", "--", "milestone-v0.9.0"])
        .output()
        .expect("run xtask");
    assert!(output.status.success(), "refusal: {output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert_eq!(
        stdout.lines().next_back().expect("last stdout line"),
        "v0.9.0",
        "the selector is the last stdout line, matching prepare-release's contract"
    );
    assert_eq!(
        fs::read_to_string(&outputs).expect("outputs file"),
        "version=v0.9.0\n",
        "the GITHUB_OUTPUT contract matches prepare-release's explicit-version arm"
    );
}

#[test]
fn malformed_tags_fail_closed_without_touching_outputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let outputs = dir.path().join("github-output");

    for bad in [
        "v0.9.0",
        "milestone-0.9.0",
        "milestone-v0.9",
        "milestone-v0.9.0-rc.1",
        "milestone-v",
        "m-1",
    ] {
        let output = xtask()
            .env("GITHUB_OUTPUT", &outputs)
            .args(["resolve-milestone-tag", "--", bad])
            .output()
            .expect("run xtask");
        assert!(!output.status.success(), "{bad} must be refused");
        let stderr = String::from_utf8(output.stderr).expect("stderr");
        assert!(
            stderr.contains(bad),
            "the error names the bad tag: {stderr}"
        );
    }
    assert!(
        !outputs.exists(),
        "a refused tag must never publish a release selector"
    );
}

#[test]
fn running_without_github_output_still_resolves_for_local_use() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = xtask()
        .env_remove("GITHUB_OUTPUT")
        .current_dir(PathBuf::from(dir.path()))
        .args(["resolve-milestone-tag", "--", "milestone-v1.2.3"])
        .output()
        .expect("run xtask");
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).expect("stdout");
    assert_eq!(
        stdout.lines().next_back().expect("last stdout line"),
        "v1.2.3"
    );
}
