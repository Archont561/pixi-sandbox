//! `prepare_release` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use xtask::prepare_release::{
    refresh_generated_files, reversion_line, stamp_internal_pins, stamp_version,
    write_touched_report,
};

fn git(args: &[&str], cwd: &Path) {
    let output = StdCommand::new("git")
        .args(args)
        .env("GIT_AUTHOR_NAME", "fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(&["init", "-q", "-b", "main"], dir.path());
    fs::write(dir.path().join("CHANGELOG.md"), "# changelog\n").expect("write");
    fs::write(dir.path().join("Cargo.toml"), "version = \"0.4.2\"\n").expect("write");
    git(&["add", "."], dir.path());
    git(&["commit", "-q", "-m", "initial"], dir.path());
    dir
}

#[test]
fn only_the_column_zero_version_line_is_stamped() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("pixi.toml");
    fs::write(
        &path,
        "version = \"1.0.0\"\n[deps]\n  version = \"9.9.9\"\npkg = { version = \"3.3.3\" }\n",
    )
    .expect("manifest");
    stamp_version(&path, "2.0.0").expect("stamp");
    let text = fs::read_to_string(&path).expect("readback");
    assert_eq!(
        text,
        "version = \"2.0.0\"\n[deps]\n  version = \"9.9.9\"\npkg = { version = \"3.3.3\" }\n"
    );
}

#[test]
fn a_manifest_without_a_top_level_version_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("x.toml");
    fs::write(&path, "[package]\nname = \"x\"\n").expect("manifest");
    assert!(stamp_version(&path, "2.0.0").is_err());
}

#[test]
fn the_suffix_after_the_closing_quote_survives() {
    assert_eq!(
        reversion_line("version = \"1.0.0\" # keep me\n", "2.0.0"),
        Some("version = \"2.0.0\" # keep me\n".to_string())
    );
    assert_eq!(reversion_line("  version = \"1.0.0\"\n", "2.0.0"), None);
}

#[test]
fn generated_files_follow_the_version_just_stamped_into_the_workspace() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manifest = dir.path().join("Cargo.toml");
    fs::write(
        &manifest,
        "[workspace]\n\n[workspace.package]\nversion = \"1.2.3\"\n",
    )
    .expect("workspace manifest");

    stamp_version(&manifest, "1.2.4").expect("stamp release version");
    refresh_generated_files(dir.path()).expect("refresh generated files");

    let relock = fs::read_to_string(dir.path().join(xtask::workflow::RELOCK_PATH))
        .expect("rendered relock workflow");
    assert!(
        relock.contains("# pixi-sandbox-version: 1.2.4"),
        "the generated file must carry the new release, not the xtask binary's old version:\n{relock}"
    );
}

#[test]
fn internal_path_pins_move_and_external_requirements_do_not() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("Cargo.toml");
    fs::write(
        &path,
        "[dependencies]\nanyhow = { version = \"1\" }\npixi-sandbox = { path = \"../pixi-sandbox\", version = \"1.0.0\" }\npixi-sandbox-core = { version = \"1.0.0\", path = \"../core\" }\n",
    )
    .expect("manifest");
    assert!(stamp_internal_pins(&path, "2.0.0").expect("stamp"));
    let text = fs::read_to_string(&path).expect("readback");
    assert!(text.contains("anyhow = { version = \"1\" }"), "{text}");
    assert!(
        text.contains("pixi-sandbox = { path = \"../pixi-sandbox\", version = \"2.0.0\" }"),
        "{text}"
    );
    assert!(
        text.contains("pixi-sandbox-core = { version = \"2.0.0\", path = \"../core\" }"),
        "{text}"
    );
    // Idempotent: a second pass reports no change.
    assert!(!stamp_internal_pins(&path, "2.0.0").expect("stamp again"));
}

/// The touched-file report is the release workflow's `git add` list, so its filtering —
/// tracked files only, staged or not — is exactly what commit-release stages. Exercised
/// against a real git in a tempdir (D10: never this checkout); the fixture builder runs
/// git itself because an oracle must not share code with the thing it judges.
#[test]
fn modified_and_staged_tracked_files_are_listed_and_untracked_are_not() {
    let dir = repo();
    let root = dir.path();
    fs::write(root.join("CHANGELOG.md"), "new entry\n").expect("write");
    fs::write(root.join("Cargo.toml"), "version = \"0.4.3\"\n").expect("write");
    git(&["add", "Cargo.toml"], root);
    fs::write(root.join("notes.txt"), "scratch\n").expect("write");

    let report = root.join("touched.txt");
    write_touched_report(root, &report).expect("report");
    assert_eq!(
        fs::read_to_string(&report).expect("read"),
        "CHANGELOG.md\nCargo.toml\n",
        "tracked changes are listed sorted; the untracked file is absent"
    );
}

#[test]
fn a_clean_tree_writes_an_empty_report() {
    let dir = repo();
    let report = dir.path().join("touched.txt");
    write_touched_report(dir.path(), &report).expect("report");
    assert_eq!(
        fs::read_to_string(&report).expect("read"),
        "",
        "nothing changed, so nothing is staged by the release"
    );
}

#[test]
fn a_non_repository_names_the_status_context() {
    let dir = tempfile::tempdir().expect("tempdir");
    let error = write_touched_report(dir.path(), &dir.path().join("touched.txt"))
        .expect_err("status outside a repository must fail");
    assert!(
        error
            .to_string()
            .contains("running git status for the touched-file report"),
        "got: {error}"
    );
}
