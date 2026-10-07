#![cfg(unix)]

use std::{fs, os::unix::fs::PermissionsExt, process::Command};

use tempfile::tempdir;

const REPO: &str = "Archont561/pixi-sandbox-starter";
const EXPECTED_ENDPOINT: &str = "repos/Archont561/pixi-sandbox-starter/git/ref/heads/main";
const COMMIT: &str = "8193c58068c3245e5267c4c604f989237e4d6e49";

fn run_with_gh(body: &str) -> std::process::Output {
    let temp = tempdir().expect("temporary directory");
    let bin = temp.path().join("bin");
    fs::create_dir(&bin).expect("create bin directory");
    let gh = bin.join("gh");
    let script = format!(
        "#!/bin/sh\n\
         if [ \"$1\" != api ] || [ \"$2\" != \"{EXPECTED_ENDPOINT}\" ]; then\n\
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
        .args(["starter-check-main", "--repo", REPO])
        .env("PATH", &bin)
        .output()
        .expect("run xtask starter-check-main")
}

#[test]
fn the_cli_checks_the_actual_main_ref_and_reports_an_owner_action_for_an_empty_repo() {
    let seeded = run_with_gh(&format!(
        "printf '%s' '{{\"ref\":\"refs/heads/main\",\"object\":{{\"sha\":\"{COMMIT}\"}}}}'"
    ));
    assert!(
        seeded.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&seeded.stdout),
        String::from_utf8_lossy(&seeded.stderr)
    );
    assert!(
        String::from_utf8_lossy(&seeded.stdout)
            .contains(&format!("{REPO} has refs/heads/main at {COMMIT}")),
        "{}",
        String::from_utf8_lossy(&seeded.stdout)
    );

    let empty = run_with_gh("printf '%s\\n' 'gh: Git Repository is empty. (HTTP 409)' >&2\nexit 1");
    assert!(!empty.status.success());
    let error = String::from_utf8_lossy(&empty.stderr);
    assert!(error.contains("refs/heads/main"), "{error}");
    assert!(error.contains("owner must seed main"), "{error}");
    assert!(error.contains("Git Repository is empty"), "{error}");
    assert!(error.contains("default branch does not prove"), "{error}");
}
