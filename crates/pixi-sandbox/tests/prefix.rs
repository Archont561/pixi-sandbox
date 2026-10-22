//! Shared promotion policy, observed through its public API and real filesystem paths.
mod support;

use pixi_sandbox::prefix::{check_output, promote_prefix};
use rstest::rstest;
use std::fs;
use support::isolated_home;
use tempfile::TempDir;

#[rstest]
#[case::absent(false, false, false)]
#[case::empty(true, false, false)]
#[case::forced(true, true, true)]
fn a_verified_stage_replaces_only_an_allowed_destination(
    isolated_home: TempDir,
    #[case] existing: bool,
    #[case] populated: bool,
    #[case] force: bool,
) {
    let root = isolated_home.path();
    let stage = root.join("stage");
    let output = root.join("output");
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("new.txt"), b"verified prefix").unwrap();
    if existing {
        fs::create_dir(&output).unwrap();
    }
    if populated {
        fs::write(output.join("old.txt"), b"previous prefix").unwrap();
    }
    check_output(&output, force).unwrap();
    promote_prefix(&stage, &output, force).unwrap();
    assert_eq!(
        fs::read(output.join("new.txt")).unwrap(),
        b"verified prefix"
    );
    assert!(!output.join("old.txt").exists());
    assert!(!stage.exists());
    assert_eq!(fs::read_dir(root).unwrap().count(), 1, "no backup remains");
}

#[rstest]
fn promotion_rechecks_and_refuses_a_newly_occupied_destination(isolated_home: TempDir) {
    let root = isolated_home.path();
    let stage = root.join("stage");
    let output = root.join("output");
    check_output(&output, false).unwrap();
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("new.txt"), b"verified prefix").unwrap();
    fs::create_dir(&output).unwrap();
    fs::write(output.join("old.txt"), b"previous prefix").unwrap();
    let error = promote_prefix(&stage, &output, false).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!(
            "{} already exists and is not empty — pass --force to replace it",
            output.display()
        )
    );
    assert_eq!(
        fs::read(output.join("old.txt")).unwrap(),
        b"previous prefix"
    );
    assert_eq!(fs::read(stage.join("new.txt")).unwrap(), b"verified prefix");
}

#[rstest]
fn a_failed_promotion_restores_the_previous_destination(isolated_home: TempDir) {
    let root = isolated_home.path();
    let output = root.join("output");
    fs::create_dir(&output).unwrap();
    fs::write(output.join("old.txt"), b"previous prefix").unwrap();
    // A missing caller-owned stage fails the real rename after the old prefix is retired.
    let error = promote_prefix(&root.join("missing-stage"), &output, true).unwrap_err();
    assert_eq!(
        error.to_string(),
        "promoting unpacked prefix; previous output restored"
    );
    assert_eq!(
        fs::read(output.join("old.txt")).unwrap(),
        b"previous prefix"
    );
    assert_eq!(
        fs::read_dir(root).unwrap().count(),
        1,
        "successful rollback leaves no backup"
    );
}

#[rstest]
fn a_failed_promotion_to_an_absent_destination_reports_that_path(isolated_home: TempDir) {
    let output = isolated_home.path().join("output");
    let error =
        promote_prefix(&isolated_home.path().join("missing-stage"), &output, false).unwrap_err();
    assert_eq!(
        error.to_string(),
        format!("promoting unpacked prefix to {}", output.display())
    );
    assert!(!output.exists());
}

#[cfg(unix)]
#[rstest]
#[case::live(false)]
#[case::dangling(true)]
fn force_replaces_the_link_not_its_target(isolated_home: TempDir, #[case] dangling: bool) {
    let root = isolated_home.path();
    let target = root.join("target");
    let output = root.join("output");
    let stage = root.join("stage");
    if !dangling {
        fs::create_dir(&target).unwrap();
        fs::write(target.join("untouched.txt"), b"not the destination").unwrap();
    }
    std::os::unix::fs::symlink(&target, &output).unwrap();
    assert!(
        check_output(&output, false).is_err(),
        "even a dangling link requires force"
    );
    fs::create_dir(&stage).unwrap();
    fs::write(stage.join("new.txt"), b"verified prefix").unwrap();
    promote_prefix(&stage, &output, true).unwrap();
    assert!(
        !fs::symlink_metadata(&output)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read(output.join("new.txt")).unwrap(),
        b"verified prefix"
    );
    if dangling {
        assert!(!target.exists());
    } else {
        assert_eq!(
            fs::read(target.join("untouched.txt")).unwrap(),
            b"not the destination"
        );
    }
}
