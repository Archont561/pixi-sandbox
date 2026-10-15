#![cfg(unix)]

//! Manual starter dispatch resolution, exercised through the CLI with an offline GitHub stub.

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

mod support;
use support::{STARTER_COMMIT as COMMIT, STARTER_SOURCE as SOURCE, STARTER_TAG as TAG};

/// Return the process result and the outputs a downstream workflow job would receive.
fn dispatch(args: &[&str], latest: &str, commit: &str) -> (std::process::Output, String) {
    let temp = tempfile::tempdir().expect("tempdir");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).expect("bin dir");
    let gh = bin.join("gh");
    fs::write(
        &gh,
        format!(
            "#!/bin/sh\n\
             case \"$*\" in\n\
               'api repos/{SOURCE}/releases/latest --jq .tag_name') {latest} ;;\n\
               'api repos/{SOURCE}/commits/{TAG} --jq .sha') {commit} ;;\n\
               *) printf '%s\\n' 'unexpected gh arguments' >&2; exit 90 ;;\n\
             esac\n"
        ),
    )
    .expect("write gh stub");
    fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).expect("executable stub");
    let outputs = temp.path().join("outputs");
    let run = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(["starter-check-dispatch", "--repo", SOURCE])
        .args(args)
        .env("PATH", &bin)
        .env("GITHUB_OUTPUT", &outputs)
        .env("GITHUB_STEP_SUMMARY", temp.path().join("summary"))
        .output()
        .expect("run starter-check-dispatch");
    (run, fs::read_to_string(outputs).unwrap_or_default())
}

#[test]
fn omitted_inputs_resolve_latest_once_and_export_its_immutable_commit() {
    let (run, outputs) = dispatch(
        &[],
        &format!("printf '%s' '{TAG}'"),
        &format!("printf '%s' '{COMMIT}'"),
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(outputs, format!("tag={TAG}\ncommit={COMMIT}\n"));
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains(TAG) && stdout.contains(COMMIT), "{stdout}");
}

#[test]
fn explicitly_blank_workflow_inputs_use_the_same_defaults() {
    let (run, outputs) = dispatch(
        &["--tag", "", "--commit", ""],
        &format!("printf '%s' '{TAG}'"),
        &format!("printf '%s' '{COMMIT}'"),
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(outputs, format!("tag={TAG}\ncommit={COMMIT}\n"));
}

#[test]
fn an_explicit_tag_skips_latest_and_resolves_its_commit() {
    let (run, outputs) = dispatch(
        &["--tag", TAG],
        "exit 91",
        &format!("printf '%s' '{COMMIT}'"),
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(outputs, format!("tag={TAG}\ncommit={COMMIT}\n"));
}

#[test]
fn an_explicit_abbreviated_commit_is_checked_but_exports_the_full_sha() {
    let (run, outputs) = dispatch(
        &["--tag", TAG, "--commit", &COMMIT[..7]],
        "exit 91",
        &format!("printf '%s' '{COMMIT}'"),
    );
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(outputs, format!("tag={TAG}\ncommit={COMMIT}\n"));
}

#[test]
fn an_explicit_commit_cannot_override_the_commit_of_latest() {
    let other = "00000000000000000000000000000000000000aa";
    let (run, outputs) = dispatch(
        &["--commit", other],
        &format!("printf '%s' '{TAG}'"),
        &format!("printf '%s' '{COMMIT}'"),
    );
    assert!(!run.status.success());
    assert!(outputs.is_empty(), "a refused dispatch must export nothing");
    let error = String::from_utf8_lossy(&run.stderr);
    assert!(error.contains(COMMIT) && error.contains(other), "{error}");
}

#[test]
fn failed_or_invalid_latest_lookups_never_export_a_release() {
    for latest in [
        "printf '%s' 'Not Found' >&2; exit 1",
        "printf '%s' ''",
        "printf '%s' 'v1.2.3-rc.1'",
    ] {
        let (run, outputs) = dispatch(&[], latest, "exit 91");
        assert!(!run.status.success());
        assert!(outputs.is_empty(), "{outputs}");
        let error = String::from_utf8_lossy(&run.stderr);
        assert!(error.contains("latest published stable release"), "{error}");
    }
}

#[test]
fn an_unresolvable_release_tag_never_exports_a_commit() {
    let (run, outputs) = dispatch(&[], &format!("printf '%s' '{TAG}'"), "exit 1");
    assert!(!run.status.success());
    assert!(outputs.is_empty());
    let error = String::from_utf8_lossy(&run.stderr);
    assert!(error.contains("does not resolve to a commit"), "{error}");
}
