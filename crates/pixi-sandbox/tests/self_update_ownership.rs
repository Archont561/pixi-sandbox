//! Unit tests for the ownership refusal ladder (`src/self_update/ownership.rs`).

use pixi_sandbox::self_update::ownership::*;
use std::path::Path;

use std::fs;

fn touch(path: &Path, body: &str) {
    fs::create_dir_all(path.parent().expect("has a parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

#[test]
fn a_plain_binary_in_a_scratch_directory_is_standalone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    touch(&bin, "binary bytes");
    let owner = classify(&bin);
    assert_eq!(owner, Ownership::Standalone);
    assert!(owner.is_standalone());
    assert!(owner.refusal(&bin).is_none());
}

#[test]
fn a_destination_that_does_not_exist_yet_is_standalone() {
    // The CI-managed case: `--dest $RUNNER_TEMP/pixi-sandbox` before anything is downloaded.
    let dir = tempfile::tempdir().expect("tempdir");
    assert_eq!(
        classify(&dir.path().join("runner-scratch/pixi-sandbox")),
        Ownership::Standalone
    );
}

#[test]
fn a_managed_launcher_is_refused_and_points_at_the_transport() {
    let dir = tempfile::tempdir().expect("tempdir");
    let launcher = dir.path().join("pixi-sandbox");
    touch(
        &launcher,
        &format!(
            "#!/bin/sh\n# {}\nexec /x\n",
            pixi_sandbox::user_tools::MANAGED_MARKER
        ),
    );
    let owner = classify(&launcher);
    assert_eq!(owner, Ownership::ManagedLauncher);
    let refusal = owner.refusal(&launcher).expect("refused");
    assert!(refusal.contains("managed launcher"), "{refusal}");
    assert!(refusal.contains("--dest"), "{refusal}");
}

#[test]
fn a_large_binary_containing_the_marker_bytes_is_not_mistaken_for_a_launcher() {
    // The marker string can legitimately appear inside the compiled binary's own data.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    let mut body = pixi_sandbox::user_tools::MANAGED_MARKER.as_bytes().to_vec();
    body.resize(128 * 1024, b'\0');
    fs::write(&bin, &body).expect("write");
    assert_eq!(classify(&bin), Ownership::Standalone);
}

#[test]
fn a_pixi_global_trampoline_is_refused_with_the_pixi_remedy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("bin/pixi-sandbox");
    touch(&bin, "trampoline");
    touch(
        &dir.path()
            .join("bin/trampoline_configuration/pixi-sandbox.json"),
        "{}",
    );
    let owner = classify(&bin);
    assert_eq!(owner, Ownership::GlobalTrampoline);
    let refusal = owner.refusal(&bin).expect("refused");
    assert!(
        refusal.contains("pixi global update pixi-sandbox"),
        "{refusal}"
    );
}

#[test]
fn a_binary_inside_a_conda_prefix_is_refused_and_the_prefix_is_named() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join(".pixi/envs/default");
    fs::create_dir_all(prefix.join(CONDA_META)).expect("mkdir");
    let bin = prefix.join("bin/pixi-sandbox");
    touch(&bin, "binary");
    let owner = classify(&bin);
    assert_eq!(
        owner,
        Ownership::CondaPrefix {
            prefix: prefix.clone()
        }
    );
    let refusal = owner.refusal(&bin).expect("refused");
    assert!(refusal.contains(&prefix.display().to_string()), "{refusal}");
    assert!(refusal.contains("pixi update pixi-sandbox"), "{refusal}");
}

#[test]
fn a_restored_transport_tool_is_refused_and_points_at_a_repack() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join(".pixi/tools/linux-64/pixi-sandbox");
    touch(&bin, "binary");
    let owner = classify(&bin);
    assert_eq!(owner, Ownership::TransportTool);
    let refusal = owner.refusal(&bin).expect("refused");
    assert!(refusal.contains("manifest"), "{refusal}");
    assert!(refusal.contains("repack"), "{refusal}");
}

#[test]
fn the_windows_transport_spelling_is_recognised_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join(".pixi/tools/win-64/pixi-sandbox.exe");
    touch(&bin, "binary");
    assert_eq!(classify(&bin), Ownership::TransportTool);
}

#[test]
fn a_pixi_project_directory_that_is_not_a_tools_dir_stays_standalone() {
    // `.pixi/` alone must not condemn a path — only the tools/<platform>/ shape does.
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join(".pixi/pixi-sandbox");
    touch(&bin, "binary");
    assert_eq!(classify(&bin), Ownership::Standalone);
}

#[test]
fn the_ladder_prefers_the_launcher_verdict_over_the_prefix_one() {
    // A managed launcher that happens to sit inside a conda prefix must report the
    // launcher remedy: it is the more specific, more actionable of the two.
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join("env");
    fs::create_dir_all(prefix.join(CONDA_META)).expect("mkdir");
    let launcher = prefix.join("bin/pixi-sandbox");
    touch(
        &launcher,
        &format!(
            "#!/bin/sh\n# {}\n",
            pixi_sandbox::user_tools::MANAGED_MARKER
        ),
    );
    assert_eq!(classify(&launcher), Ownership::ManagedLauncher);
}
