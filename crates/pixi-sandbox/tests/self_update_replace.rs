//! Unit tests for stage-before-replace (`src/self_update/replace.rs`).

use pixi_sandbox::self_update::replace::*;
use std::path::{Path, PathBuf};

use rstest::rstest;
use std::fs;

const VERSION: &str = "0.4.4";

fn existing(dir: &Path, body: &[u8]) -> PathBuf {
    let bin = dir.join("pixi-sandbox");
    fs::write(&bin, body).expect("write");
    bin
}

#[rstest]
#[case(ReplaceStrategy::Unix)]
#[case(ReplaceStrategy::Windows)]
fn both_strategies_replace_the_destination_with_the_new_bytes(#[case] strategy: ReplaceStrategy) {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"old binary");
    let result = install(strategy, &bin, b"new binary", VERSION).expect("installed");
    assert_eq!(fs::read(&bin).expect("read"), b"new binary");
    assert_eq!(result.destination, bin);
}

#[rstest]
#[case(ReplaceStrategy::Unix)]
#[case(ReplaceStrategy::Windows)]
fn both_strategies_create_a_destination_that_does_not_exist_yet(#[case] strategy: ReplaceStrategy) {
    // The CI-managed case: bootstrap into empty runner scratch.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("scratch/pixi-sandbox");
    install(strategy, &bin, b"fresh", VERSION).expect("installed");
    assert_eq!(fs::read(&bin).expect("read"), b"fresh");
}

#[rstest]
#[case(ReplaceStrategy::Unix)]
#[case(ReplaceStrategy::Windows)]
fn no_staging_file_survives_a_successful_install(#[case] strategy: ReplaceStrategy) {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"old");
    install(strategy, &bin, b"new", VERSION).expect("installed");
    let leftovers: Vec<String> = fs::read_dir(dir.path())
        .expect("readdir")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(STAGING_INFIX))
        .collect();
    assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");
}

#[test]
fn the_unix_strategy_leaves_a_running_binarys_inode_intact() {
    // The task-37 property: a rename swaps the directory entry, so an already-open handle
    // (a running process) keeps reading the old bytes instead of hitting ETXTBSY.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"old binary");
    let held = fs::File::open(&bin).expect("open the 'running' image");
    install(ReplaceStrategy::Unix, &bin, b"new binary", VERSION).expect("installed");
    let mut old = Vec::new();
    {
        use std::io::Read;
        let mut held = held;
        held.read_to_end(&mut old).expect("read the held inode");
    }
    assert_eq!(old, b"old binary", "the held inode must be untouched");
    assert_eq!(fs::read(&bin).expect("read"), b"new binary");
}

#[test]
fn the_unix_strategy_keeps_no_displaced_sibling() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"old");
    let result = install(ReplaceStrategy::Unix, &bin, b"new", VERSION).expect("installed");
    assert_eq!(result.displaced, None);
}

#[test]
fn the_windows_strategy_renames_the_old_binary_aside_under_its_version() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"old binary");
    let result =
        install(ReplaceStrategy::Windows, &bin, b"new binary", VERSION).expect("installed");
    let displaced = result.displaced.expect("windows keeps the old image aside");
    assert!(
        displaced
            .file_name()
            .expect("name")
            .to_string_lossy()
            .contains(VERSION),
        "the displaced name records the version it held: {displaced:?}"
    );
    assert_eq!(fs::read(&bin).expect("read"), b"new binary");
}

#[test]
fn a_leftover_displaced_file_is_swept_by_the_next_update() {
    // On a real Windows run the unlink at the end fails while the image is mapped, so the
    // reaping has to happen on the *following* update rather than this one.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"v1");
    let stale = dir
        .path()
        .join(format!("pixi-sandbox{DISPLACED_INFIX}0.0.1"));
    fs::write(&stale, b"corpse").expect("write");

    let result = install(ReplaceStrategy::Windows, &bin, b"v2", VERSION).expect("installed");
    assert!(
        result.swept.contains(&stale),
        "the stale corpse must be reported as swept: {:?}",
        result.swept
    );
    assert!(!stale.exists(), "the stale corpse must be gone");
}

#[test]
fn the_sweep_leaves_unrelated_neighbours_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"v1");
    let neighbour = dir.path().join("pixi");
    fs::write(&neighbour, b"another tool").expect("write");
    install(ReplaceStrategy::Windows, &bin, b"v2", VERSION).expect("installed");
    assert_eq!(fs::read(&neighbour).expect("read"), b"another tool");
}

#[cfg(unix)]
#[rstest]
#[case(ReplaceStrategy::Unix)]
#[case(ReplaceStrategy::Windows)]
fn the_installed_binary_is_executable(#[case] strategy: ReplaceStrategy) {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    // A non-executable predecessor must not leave the replacement non-executable.
    fs::write(&bin, b"old").expect("write");
    fs::set_permissions(&bin, fs::Permissions::from_mode(0o644)).expect("chmod");
    install(strategy, &bin, b"new", VERSION).expect("installed");
    let mode = fs::metadata(&bin).expect("stat").permissions().mode();
    assert_eq!(mode & 0o777, 0o755, "mode was {mode:o}");
}

#[test]
fn a_failed_windows_swap_puts_the_previous_binary_back() {
    // Drive the swap directly with a staged path that does not exist, so the second
    // rename is guaranteed to fail on every platform. That is the branch that matters:
    // the destination has already been moved aside at this point.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"the previous binary");
    let missing = dir.path().join("never-staged");

    let err = windows_swap(&missing, &bin, VERSION).unwrap_err();
    assert!(
        bin.exists(),
        "the previous binary must be rolled back into place: {err:?}"
    );
    assert_eq!(
        fs::read(&bin).expect("read"),
        b"the previous binary",
        "rollback must restore the original bytes"
    );
    let leftovers: Vec<String> = fs::read_dir(dir.path())
        .expect("readdir")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(DISPLACED_INFIX))
        .collect();
    assert!(
        leftovers.is_empty(),
        "a rolled-back swap must leave no displaced sibling: {leftovers:?}"
    );
}

#[rstest]
#[case(ReplaceStrategy::Unix)]
#[case(ReplaceStrategy::Windows)]
fn a_staging_failure_leaves_the_destination_untouched(#[case] strategy: ReplaceStrategy) {
    // Occupy the staging path with a directory: the write cannot succeed, and the
    // destination must not have been touched by the time it fails.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = existing(dir.path(), b"the previous binary");
    fs::create_dir(staging_path(&bin)).expect("block the staging path");

    let err = install(strategy, &bin, b"new", VERSION).unwrap_err();
    assert_eq!(
        fs::read(&bin).expect("read"),
        b"the previous binary",
        "a staging failure must not disturb the destination: {err:?}"
    );
}

#[test]
fn the_current_strategy_matches_the_host() {
    let expected = if cfg!(windows) {
        ReplaceStrategy::Windows
    } else {
        ReplaceStrategy::Unix
    };
    assert_eq!(ReplaceStrategy::current(), expected);
}
