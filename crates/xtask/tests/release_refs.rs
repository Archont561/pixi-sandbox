//! `release_refs` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use std::fs;
use std::path::Path;
use xtask::release_refs::{project_root_ident, rewrite, scan};

const ROOT_IDENT: &str = "Archont561/pixi-sandbox";

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::write(
        dir.path().join("Cargo.toml"),
        "[workspace.package]\nversion = \"2.0.0\"\n",
    )
    .expect("cargo manifest");
    dir
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    fs::write(path, text).expect("file");
}

#[test]
fn the_project_identity_is_the_explicit_upstream_constant() {
    let dir = fixture();
    assert_eq!(project_root_ident(dir.path()).expect("ident"), ROOT_IDENT);
}

#[test]
fn scan_reports_our_stale_pin_and_ignores_third_party_lines() {
    let dir = fixture();
    write(
        dir.path(),
        "README.md",
        "uses: Archont561/pixi-sandbox/setup@v1.0.0\nuses: actions/checkout@v7.0.1\n",
    );
    let findings = scan(dir.path()).expect("scan");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("pins v1.0.0, manifests declare v2.0.0"));
}

#[test]
fn the_opt_out_marker_silences_a_deliberate_old_reference() {
    let dir = fixture();
    write(
        dir.path(),
        "README.md",
        "<!-- stale-ref-allowed -->\nupgrading from Archont561/pixi-sandbox@v1.0.0\n",
    );
    assert!(scan(dir.path()).expect("scan").is_empty());
}

#[test]
fn docs_content_may_carry_no_literal_tag_of_ours_at_all() {
    let dir = fixture();
    write(
        dir.path(),
        "docs/src/content/install.mdx",
        "run PIXI_SANDBOX_VERSION=v2.0.0 install\n",
    );
    let findings = scan(dir.path()).expect("scan");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert!(findings[0].contains("write v__VERSION__"), "{findings:?}");
}

#[test]
fn historical_directories_are_outside_both_modes() {
    let dir = fixture();
    write(
        dir.path(),
        "backlog/tasks/old.md",
        "shipped Archont561/pixi-sandbox@v0.1.0\n",
    );
    write(
        dir.path(),
        "CHANGELOG.md",
        "Archont561/pixi-sandbox@v0.1.0\n",
    );
    assert!(scan(dir.path()).expect("scan").is_empty());
}

#[test]
fn rewrite_moves_our_lines_leaves_third_parties_and_keeps_a_missing_trailing_newline() {
    let dir = fixture();
    // Deliberately no trailing newline: a rewrite must not invent one.
    write(
        dir.path(),
        "README.md",
        "uses: Archont561/pixi-sandbox/setup@v1.0.0\nuses: actions/checkout@v7.0.1 # v7.0.1",
    );
    let touched = rewrite(dir.path(), "v2.0.0").expect("rewrite");
    assert_eq!(touched, 1);
    let text = fs::read_to_string(dir.path().join("README.md")).expect("readback");
    assert_eq!(
        text,
        "uses: Archont561/pixi-sandbox/setup@v2.0.0\nuses: actions/checkout@v7.0.1 # v7.0.1"
    );
    // Idempotent: a second run touches nothing.
    assert_eq!(rewrite(dir.path(), "v2.0.0").expect("rewrite again"), 0);
}
