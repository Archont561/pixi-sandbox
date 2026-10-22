//! CI build graph contracts through the public automation CLI, using isolated fixtures.
use std::{fs, path::Path, process::Command};

fn copy_tree(source: &Path, target: &Path) {
    fs::create_dir_all(target).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let destination = target.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            fs::copy(entry.path(), destination).unwrap();
        }
    }
}

fn invoke(change: Option<(&str, &str, &str)>) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ci-builds"),
        root.path(),
    );
    if let Some((path, original, replacement)) = change {
        let path = root.path().join(path);
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains(original));
        fs::write(path, text.replace(original, replacement)).unwrap();
    }
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .arg("--root")
        .arg(root.path())
        .arg("check-ci-builds")
        .env("HOME", root.path())
        .env("USERPROFILE", root.path())
        .output()
        .unwrap()
}

#[test]
fn ci_runs_one_workspace_suite_and_serializes_cargo_lints() {
    let output = invoke(None);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_optional_or_duplicate_test_gates_are_refused() {
    for (original, weakened) in [
        ("run: pixi run coverage", "run: pixi run test"),
        ("run: pixi run test-doc", "run: pixi run test"),
        ("run: pixi run test-payload-roundtrip", "run: echo skipped"),
        (
            "run: pixi run coverage",
            "continue-on-error: true\n        run: pixi run coverage",
        ),
        ("steps.changes.outputs.code != 'false'", "false"),
    ] {
        let output = invoke(Some((".github/workflows/ci.yml", original, weakened)));
        assert!(
            !output.status.success(),
            "accepted weakened test gate: {original}"
        );
    }
}

#[test]
fn mismatched_toolchain_or_lost_build_flags_are_refused() {
    for (path, original, weakened) in [
        (
            ".github/workflows/ci.yml",
            "cmd-format: pixi run --frozen -- {0}",
            "cmd-format: '{0}'",
        ),
        (
            ".github/workflows/ci.yml",
            "hashFiles('pixi.lock')",
            "hashFiles('unrelated')",
        ),
        (
            ".github/workflows/ci.yml",
            "CARGO_PROFILE_TEST_DEBUG: \"0\"",
            "CARGO_PROFILE_TEST_DEBUG: \"2\"",
        ),
        ("turbo.json", "\"CARGO_INCREMENTAL\"", "\"UNRELATED\""),
        ("turbo.json", "\"CARGO_PROFILE_*\"", "\"UNRELATED\""),
    ] {
        let output = invoke(Some((path, original, weakened)));
        assert!(
            !output.status.success(),
            "accepted mismatched configuration: {original}"
        );
    }
}

#[test]
fn competing_cargo_lints_release_planning_and_filtered_suites_are_refused() {
    for (path, original, weakened) in [
        ("turbo.json", "\"@repo/rust#lint\"", "\"unrelated#lint\""),
        ("turbo.json", "\"//#lint:generated\"", "\"//#unrelated\""),
        ("turbo.json", "\"//#lint:plan\"", "\"//#unrelated\""),
        (
            "package.json",
            "-p pixi-sandbox -- plan",
            "-p pixi-sandbox --release -- plan",
        ),
        (
            "crates/package.json",
            "clippy --workspace --all-targets",
            "clippy -p pixi-sandbox",
        ),
        (
            "crates/package.json",
            "nextest --workspace",
            "nextest -p pixi-sandbox",
        ),
        ("pixi.toml", "cargo test --doc --workspace", "bun run test"),
    ] {
        let output = invoke(Some((path, original, weakened)));
        assert!(
            !output.status.success(),
            "accepted weakened graph: {original}"
        );
    }
}

#[test]
fn generated_reports_must_not_invalidate_source_lint_caches() {
    let output = invoke(Some(("turbo.json", "!crates/lcov.info", "!unrelated")));
    assert!(
        !output.status.success(),
        "coverage output must not invalidate lint caches"
    );
}
