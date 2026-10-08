//! `repo_checks::agents_md` (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

mod support;

use std::fs;
use support::{headlines, valid_fixture};

/// A repo map plus a Test conventions paragraph, the two places this check reads. `paths`
/// and `modules` are the claims under test; everything else is scaffolding the other
/// checks need.
fn agents_md(paths: &[&str], modules: &[&str]) -> String {
    let rows: String = paths
        .iter()
        .map(|path| format!("| `{path}` | a row |\n"))
        .collect();
    format!(
        "## Repo map\n\n| path | notes |\n| --- | --- |\n{rows}\n\
         ### Test conventions\n\n\
         - A module worth testing is **promoted to `lib.rs` as `pub mod`** and tested \
         through its public API — the shape {} already have.\n\
         - Something else entirely, with `assert_cmd` in it.\n",
        modules
            .iter()
            .map(|m| format!("`{m}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn fixture(paths: &[&str], modules: &[&str], lib_mods: &[&str]) -> tempfile::TempDir {
    let dir = valid_fixture();
    fs::write(dir.path().join("AGENTS.md"), agents_md(paths, modules)).expect("AGENTS.md");
    let lib: String = lib_mods.iter().map(|m| format!("pub mod {m};\n")).collect();
    fs::write(dir.path().join(xtask::repo_checks::agents_md::LIB), lib).expect("lib.rs");
    dir
}

#[test]
fn a_repo_map_path_that_does_not_resolve_fires_check_12() {
    let dir = fixture(
        &["crates/pixi-sandbox/src/release.rs"],
        &["generated"],
        &["generated"],
    );
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("names a path that is not in the tree")),
        "{found:?}"
    );
}

#[test]
fn a_module_lib_rs_exports_but_the_docs_omit_fires_check_12() {
    let dir = fixture(
        &["scripts/restore.sh"],
        &["generated"],
        &["generated", "host_probe"],
    );
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("missing a module lib.rs exports")),
        "{found:?}"
    );
}

#[test]
fn a_module_the_docs_name_but_lib_rs_dropped_fires_check_12() {
    let dir = fixture(
        &["scripts/restore.sh"],
        &["generated", "retired"],
        &["generated"],
    );
    let found = headlines(dir.path());
    assert!(
        found
            .iter()
            .any(|h| h.contains("names a module lib.rs does not export")),
        "{found:?}"
    );
}

#[test]
fn an_accurate_agents_md_is_clean() {
    let dir = fixture(
        &["crates/pixi-sandbox/src/lib.rs", "scripts/restore.sh"],
        &["generated", "host_probe"],
        &["generated", "host_probe"],
    );
    let found: Vec<String> = headlines(dir.path())
        .into_iter()
        .filter(|h| h.contains("AGENTS.md") || h.contains("lib.rs"))
        .collect();
    assert!(found.is_empty(), "{found:?}");
}

/// The false positives that would make this check cry wolf every time someone documents a
/// relative path: a bare filename, a path anchored at a crate rather than the root, a
/// build tree, and a placeholder are all prose, not claims about the root.
#[test]
fn paths_that_are_not_root_anchored_are_left_alone() {
    let dir = fixture(
        &[
            "cli.rs",
            "tests/manifest.rs",
            "xtask/src/release_assets.rs",
            "generated/relock_workflow.rs",
            ".pixi/sandbox-env.sh",
            "dist/conda/conda-<platform>/",
            "@biomejs/biome",
        ],
        &["generated"],
        &["generated"],
    );
    let found = headlines(dir.path());
    assert!(
        !found.iter().any(|h| h.contains("AGENTS.md")),
        "prose about a relative path is not a claim about the root: {found:?}"
    );
}

/// A directory row ends in `/` or `/*`; both mean the directory, not a file inside it.
#[test]
fn a_directory_row_resolves_to_the_directory() {
    let dir = fixture(
        &["crates/pixi-sandbox/src/", "crates/pixi-sandbox/src/*"],
        &["generated"],
        &["generated"],
    );
    let found = headlines(dir.path());
    assert!(!found.iter().any(|h| h.contains("AGENTS.md")), "{found:?}");
}

/// A repository with no AGENTS.md makes no claim to hold it to, and one whose Test
/// conventions section is missing must not fail the promoted-module half either.
#[test]
fn a_repository_without_the_documented_sections_is_clean() {
    let dir = valid_fixture();
    let found: Vec<String> = headlines(dir.path())
        .into_iter()
        .filter(|h| h.contains("AGENTS.md") || h.contains("lib.rs"))
        .collect();
    assert!(found.is_empty(), "{found:?}");

    let dir = valid_fixture();
    fs::write(
        dir.path().join("AGENTS.md"),
        "# Agent notes\n\nNothing here.\n",
    )
    .unwrap();
    assert!(
        headlines(dir.path())
            .iter()
            .all(|h| !h.contains("AGENTS.md"))
    );
}
