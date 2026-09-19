use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_golden() {
    let mut cmd = Command::cargo_bin("pixi-sandbox").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pack, publish and reconstruct"));
}

#[test]
fn test_pack_help() {
    let mut cmd = Command::cargo_bin("pixi-sandbox").unwrap();
    cmd.args(["pack", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pack pixi environments"));
}

#[test]
fn test_reconstruct_help() {
    let mut cmd = Command::cargo_bin("pixi-sandbox").unwrap();
    cmd.args(["reconstruct", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Reconstruct"));
}

#[test]
fn test_inventory_help() {
    let mut cmd = Command::cargo_bin("pixi-sandbox").unwrap();
    cmd.args(["inventory", "--help"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("inventory"));
}

#[test]
fn test_version() {
    let mut cmd = Command::cargo_bin("pixi-sandbox").unwrap();
    cmd.arg("--version");
    cmd.assert().success();
}
