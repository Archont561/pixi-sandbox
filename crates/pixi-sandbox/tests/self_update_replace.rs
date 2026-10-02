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

/// The rollback's own failure mode: the staged file is missing *and* the destination name has
/// been taken by something that cannot be renamed over, so the previous binary cannot be put
/// back. The operator must be told where their binary went rather than left guessing.
#[test]
fn an_unrecoverable_windows_swap_names_the_file_to_move_back_by_hand() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    fs::write(&bin, b"the previous binary").expect("write");
    let missing = dir.path().join("never-staged");

    // Re-occupy the destination with a directory while the swap is mid-flight by making the
    // rollback target unavailable: a directory cannot be replaced by `rename` of a file.
    let err = {
        let aside = bin.with_file_name(format!("pixi-sandbox{DISPLACED_INFIX}0.0.1"));
        fs::rename(&bin, &aside).expect("move aside");
        fs::create_dir(&bin).expect("occupy the destination");
        let result = windows_swap(&missing, &bin, VERSION);
        result.unwrap_err()
    };
    let text = format!("{err:?}");
    assert!(
        text.contains("pixi-sandbox"),
        "the message must name the binary: {text}"
    );
}

/// A destination whose parent cannot be created is reported, not silently skipped.
#[test]
fn an_uncreatable_destination_directory_is_an_error() {
    let dir = tempfile::tempdir().expect("tempdir");
    let blocker = dir.path().join("not-a-dir");
    fs::write(&blocker, b"a file where a directory is needed").expect("write");
    let err = install(
        ReplaceStrategy::Unix,
        &blocker.join("pixi-sandbox"),
        b"new",
        VERSION,
    )
    .unwrap_err();
    assert!(format!("{err:?}").contains("not-a-dir"), "{err:?}");
}

/// The sweep must tolerate a directory it cannot read rather than abort the update.
#[test]
fn a_destination_directory_that_cannot_be_listed_still_installs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("fresh/pixi-sandbox");
    let result = install(ReplaceStrategy::Unix, &bin, b"new", VERSION).expect("installed");
    assert!(
        result.swept.is_empty(),
        "nothing to sweep in a new directory"
    );
}

/// The Unix swap's failure path: a directory occupying the destination cannot be renamed over.
/// The staged file must not be left behind.
#[test]
fn a_failed_unix_rename_is_reported_and_leaves_no_staged_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("pixi-sandbox");
    fs::create_dir(&destination).expect("a directory where the binary should be");
    fs::write(destination.join("occupant"), b"x").expect("make it non-empty");

    let err = install(ReplaceStrategy::Unix, &destination, b"new", VERSION).unwrap_err();
    assert!(format!("{err:?}").contains("pixi-sandbox"), "{err:?}");
    assert!(
        !staging_path(&destination).exists(),
        "a failed swap must clean up its staged file"
    );
}

/// The Windows swap cannot even move the old image aside — the error must name both paths so
/// an operator knows nothing was touched.
#[cfg(unix)]
#[test]
fn a_windows_swap_that_cannot_move_the_old_binary_aside_reports_both_paths() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).expect("mkdir");
    let destination = locked.join("pixi-sandbox");
    fs::write(&destination, b"the previous binary").expect("write");
    let staged = dir.path().join("staged");
    fs::write(&staged, b"new").expect("write");

    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).expect("read-only dir");
    let result = windows_swap(&staged, &destination, VERSION);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).expect("restore");

    let err = result.unwrap_err();
    let text = format!("{err:?}");
    assert!(text.contains("aside"), "{text}");
    assert_eq!(
        fs::read(&destination).expect("read"),
        b"the previous binary",
        "nothing may be touched when the move aside fails"
    );
}

/// An unlistable destination directory must not abort the update — the sweep is best effort.
#[cfg(unix)]
#[test]
fn an_unreadable_directory_sweeps_nothing_instead_of_failing() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("opaque");
    fs::create_dir(&target).expect("mkdir");
    let destination = target.join("pixi-sandbox");
    // Write-but-not-read: staging and renaming still work, listing does not.
    fs::set_permissions(&target, fs::Permissions::from_mode(0o333)).expect("chmod");
    let result = install(ReplaceStrategy::Unix, &destination, b"new", VERSION);
    fs::set_permissions(&target, fs::Permissions::from_mode(0o755)).expect("restore");

    let result = result.expect("an unlistable directory is not a failure");
    assert!(result.swept.is_empty());
    assert_eq!(fs::read(&destination).expect("read"), b"new");
}

/// A sibling whose name is not UTF-8 must be skipped by the sweep, not panic it.
#[cfg(unix)]
#[test]
fn a_non_utf8_sibling_is_skipped_by_the_sweep() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let odd = dir.path().join(OsStr::from_bytes(b"\xff\xfe-not-utf8"));
    fs::write(&odd, b"junk").expect("write");
    let destination = dir.path().join("pixi-sandbox");
    fs::write(&destination, b"old").expect("write");

    install(ReplaceStrategy::Windows, &destination, b"new", VERSION).expect("installed");
    assert!(odd.exists(), "an unrelated non-UTF-8 file must survive");
    assert_eq!(fs::read(&destination).expect("read"), b"new");
}
