//! `version.rs` — reading the workspace version (moved from the module's inline `#[cfg(test)]`,
//! TASK-83: names kept, bodies verbatim).

use std::fs;
use xtask::version::workspace_version;

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
