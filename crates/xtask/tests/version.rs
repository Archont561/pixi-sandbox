//! `version.rs` — reading the workspace version (moved from the module's inline `#[cfg(test)]`,
//! TASK-83: names kept, bodies verbatim).

use std::fs;
use xtask::version::{milestone_tag_version, workspace_version};

#[test]
fn the_workspace_package_version_is_read_not_the_first_version_line() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace.dependencies]\nserde = { version = \"1\" }\n\n[workspace.package]\nversion = \"9.8.7\"\n",
    )
    .expect("manifest");
    assert_eq!(workspace_version(dir.path()).expect("version"), "9.8.7");
}

#[test]
fn a_manifest_without_the_table_is_an_error_naming_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").expect("manifest");
    let err = workspace_version(dir.path()).expect_err("must fail");
    assert!(format!("{err:#}").contains("[workspace.package]"));
}

#[test]
fn a_milestone_tag_resolves_to_its_release_selector() {
    assert_eq!(
        milestone_tag_version("milestone-v0.9.0").expect("canonical tag"),
        "v0.9.0"
    );
    assert_eq!(
        milestone_tag_version("milestone-v10.20.30").expect("two digits"),
        "v10.20.30"
    );
}

#[test]
fn anything_but_a_strict_milestone_tag_is_refused() {
    for bad in [
        "v0.9.0",
        "milestone-0.9.0",
        "milestone-v0.9",
        "milestone-v0.9.0-rc.1",
        "milestone-v",
        "milestone-v0.9.0/extra",
        "m-1",
        "",
    ] {
        assert!(
            milestone_tag_version(bad).is_err(),
            "{bad:?} must not start the release lane"
        );
    }
}
