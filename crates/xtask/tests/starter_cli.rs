#![cfg(unix)]

//! The two starter verbs the publish lane calls, run as the binary runs them.
//!
//! `starter.yml` reaches this code only through `xtask starter-clone` and
//! `xtask starter-check-dispatch`, so the flag wiring is part of what has to stay true — a guard
//! that reads the wrong argument is the same class of incident task-85 was written from. Real
//! `git` in tempdirs, a `gh` stub on `PATH`: neither test touches this repository or the network
//! (D10), and the `gh` stub is the shape `starter_check_main.rs` established.

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    process::{Command, Output},
};

mod support;
use support::{STARTER_COMMIT as COMMIT, STARTER_TAG as TAG, runtime_state, starter_tree};

const SOURCE: &str = "Archont561/pixi-sandbox";
const OTHER: &str = "00000000000000000000000000000000000000aa";
const VERSION: &str = "1.2.3";

/// A committed starter tree with runtime state written into it afterwards, which is what the
/// assembly steps leave behind and what a clone must not inherit.
fn assembled_starter(dir: &std::path::Path) {
    starter_tree(dir, VERSION);
    support::run_git(dir, &["init", "-q"]);
    support::commit_all(dir, "seed the starter");
    runtime_state(dir);
}

#[test]
fn the_cli_assembles_a_fresh_clone_and_leaves_the_runtime_state_out_of_it() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);
    let out = root.path().join("clone");

    let run = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "starter-clone",
            "--dir",
            &dir.display().to_string(),
            "--out",
            &out.display().to_string(),
        ])
        .output()
        .expect("run xtask starter-clone");

    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        out.join("starter/pixi.toml").exists(),
        "the clone is what the next step runs the dev task in"
    );
    assert!(
        !out.join("starter/.pixi").exists(),
        "runtime state the ignore file covers must not be carried"
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("fresh clone at"),
        "the step has to say where it put the tree: {}",
        String::from_utf8_lossy(&run.stdout)
    );
}

/// A starter tree with no repository has no answer to give, and `git init` is not this verb's job.
#[test]
fn the_cli_refuses_a_starter_tree_that_is_not_a_work_tree() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("loose");
    fs::create_dir_all(&dir).expect("mkdir");
    starter_tree(&dir, VERSION);

    let run = Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "starter-clone",
            "--dir",
            &dir.display().to_string(),
            "--out",
            &root.path().join("clone").display().to_string(),
        ])
        .output()
        .expect("run xtask starter-clone");
    assert!(!run.status.success());
    assert!(
        String::from_utf8_lossy(&run.stderr).contains("not a git work tree"),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
}

fn run_with_gh(body: &str) -> Output {
    let temp = tempfile::tempdir().expect("temporary directory");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).expect("create bin directory");
    let endpoint = format!("repos/{SOURCE}/commits/{TAG}");
    let gh = bin.join("gh");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" != api ] || [ \"$2\" != \"{endpoint}\" ] || [ \"$3\" != --jq ] || [ \"$4\" != .sha ]; then\n\
           printf '%s\\n' 'unexpected gh api arguments' >&2\n\
           exit 90\n\
         fi\n\
         {body}\n"
    );
    fs::write(&gh, script).expect("write gh stub");
    let mut permissions = fs::metadata(&gh).expect("gh metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&gh, permissions).expect("make gh stub executable");

    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args([
            "starter-check-dispatch",
            "--repo",
            SOURCE,
            "--tag",
            TAG,
            "--commit",
            COMMIT,
        ])
        .env("PATH", &bin)
        .output()
        .expect("run xtask starter-check-dispatch")
}

#[test]
fn the_cli_reports_which_commit_the_tag_names_when_the_dispatch_agrees() {
    let run = run_with_gh(&format!("printf '%s' '{COMMIT}'"));
    assert!(
        run.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let stdout = String::from_utf8_lossy(&run.stdout);
    assert!(stdout.contains(TAG) && stdout.contains(COMMIT), "{stdout}");
    assert!(stdout.contains("names commit"), "{stdout}");
}

#[test]
fn the_cli_refuses_a_dispatch_naming_another_commit_and_prints_both() {
    let run = run_with_gh(&format!("printf '%s' '{OTHER}'"));
    assert!(
        !run.status.success(),
        "a mismatched dispatch must fail closed"
    );
    let error = String::from_utf8_lossy(&run.stderr);
    assert!(error.contains(COMMIT) && error.contains(OTHER), "{error}");
    assert!(error.contains("no starter work was started"), "{error}");
}
