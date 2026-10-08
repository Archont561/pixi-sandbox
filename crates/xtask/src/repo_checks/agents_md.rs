//! Check 12: `AGENTS.md` describes a tree that exists — the repo map names no path that has
//! moved, and the promoted-module list is the one `lib.rs` declares.
//!
//! Why this is a check and not a test: the file is documentation, and D10 forbids a test from
//! reading this repository at all (`tests/fixtures.rs::no_test_targets_the_repository_root`),
//! so the drift an agent reads as instruction had nothing holding it to the tree. Both halves
//! of it were stale when this check was written (TASK-84): the repo map named
//! `src/release.rs`, which had become `src/release/` with two files, and the promoted-module
//! list named four of the six `pub mod`s in `lib.rs`.

use super::Failure;
use super::support::rel;
use anyhow::Result;
use std::path::Path;

/// Trees a repo-map path may name that are produced by a build or a restore, never checked in.
/// `.pixi/` is the restored prefix: it exists in an airlock checkout and not in a fresh clone,
/// so whether it resolves says nothing about the documentation.
const GENERATED_TREES: [&str; 5] = [".pixi", "dist", "target", "node_modules", "out"];

/// The `lib.rs` a promoted module is promoted *to*.
#[doc(hidden)] // test boundary: the file the promoted-module list must match (tests/repo_checks_agents_md.rs)
pub const LIB: &str = "crates/pixi-sandbox/src/lib.rs";

pub(super) fn agents_md_matches_tree(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let path = root.join("AGENTS.md");
    if !path.is_file() {
        // A repository without the file makes no claim to hold it to.
        return Ok(());
    }
    let text = crate::util::read(&path)?;
    let shown = rel(root, &path);

    if let Some(map) = section(&text, "## Repo map") {
        let mut missing = Vec::new();
        for token in backticked(map) {
            if let Some(candidate) = repo_map_path(root, token) {
                if !root.join(&candidate).exists() {
                    missing.push(format!("{shown}: `{token}` does not resolve"));
                }
            }
        }
        missing.sort();
        if !missing.is_empty() {
            failures.push(Failure::with(
                "the AGENTS.md repo map names a path that is not in the tree:",
                missing,
                "point the row at the path as it is now (a module that grew into a directory is the usual cause), or drop the row",
            ));
        }
    }

    let lib = root.join(LIB);
    if !lib.is_file() {
        return Ok(());
    }
    // Both borrow from bindings that outlive the vectors of &str: `pub_mods` and `backticked`
    // return slices into the source and the bullet text, never owned copies.
    let lib_source = crate::util::read(&lib)?;
    let declared = pub_mods(&lib_source);
    let conventions: Vec<String> = match section(&text, "### Test conventions") {
        Some(section) => bullets(section),
        None => Vec::new(),
    };
    let documented: Vec<&str> = conventions
        .iter()
        .filter(|bullet| bullet.contains("promoted"))
        .flat_map(|bullet| backticked(bullet).filter(|token| is_identifier(token)))
        .collect();

    let undocumented: Vec<String> = declared
        .iter()
        .filter(|module| !documented.contains(module))
        .map(|module| format!("{LIB} exports `{module}`, which AGENTS.md never names"))
        .collect();
    if !undocumented.is_empty() {
        failures.push(Failure::with(
            "AGENTS.md's promoted-module list is missing a module lib.rs exports:",
            undocumented,
            "add it to the list in Test conventions — an agent reads that list to decide what it can test",
        ));
    }

    let stale: Vec<String> = documented
        .iter()
        .filter(|module| !declared.contains(module))
        .map(|module| format!("AGENTS.md names `{module}`, which {LIB} does not export"))
        .collect();
    if !stale.is_empty() {
        failures.push(Failure::with(
            "AGENTS.md's promoted-module list names a module lib.rs does not export:",
            stale,
            "remove it, or promote the module if it is meant to be reachable from tests/",
        ));
    }
    Ok(())
}

/// The markdown section under `heading`, up to the next heading of any level.
fn section<'a>(text: &'a str, heading: &str) -> Option<&'a str> {
    let start = text.find(heading)?;
    let rest = &text[start + heading.len()..];
    Some(&rest[..rest.find("\n#").unwrap_or(rest.len())])
}

/// Every backticked span: markdown code spans never nest, so every other backtick-delimited
/// run is a code span rather than the text around one.
fn backticked(text: &str) -> impl Iterator<Item = &str> {
    text.split('`').skip(1).step_by(2)
}

/// A repo-map token worth resolving, or `None` when it is not a root-anchored source path.
///
/// The repo map is prose about the tree, and most of its code spans are not paths at all —
/// command names, trait names, subcommand names. Of those that are paths, several are written
/// relative to something the sentence supplies rather than to the repository root
/// (`cli.rs`, `tests/manifest.rs`, `xtask/src/release_assets.rs`), because naming them from the
/// root every time would bury the point of the row. So a token is only checked when its first
/// segment exists at the root: that is the difference between "this claims where a thing is"
/// and "this mentions a file".
fn repo_map_path(root: &Path, token: &str) -> Option<String> {
    if !token.contains('/') || token.contains('@') || token.contains('<') {
        // No directory: a bare filename. Or a package name (`@biomejs/biome`), or a
        // placeholder (`conda-<platform>/`), which names a shape rather than a file.
        return None;
    }
    let head = token.split('/').next().unwrap_or_default();
    if GENERATED_TREES.contains(&head) || !root.join(head).exists() {
        return None;
    }
    Some(
        token
            .trim_end_matches('/')
            .trim_end_matches("/*")
            .to_string(),
    )
}

/// One markdown list item, continuation lines folded in — the promoted-module list is a single
/// bullet that wraps, so a line-oriented scan would read only its first line.
fn bullets(section: &str) -> Vec<String> {
    let mut items = Vec::new();
    let mut current: Option<String> = None;
    for line in section.lines() {
        if line.starts_with("- ") {
            if let Some(previous) = current.replace(line.to_string()) {
                items.push(previous);
            }
        } else if line.starts_with("  ") && !line.trim().is_empty() {
            if let Some(current) = current.as_mut() {
                current.push(' ');
                current.push_str(line.trim());
            }
        } else if let Some(previous) = current.take() {
            items.push(previous);
        }
    }
    if let Some(last) = current {
        items.push(last);
    }
    items
}

fn is_identifier(token: &str) -> bool {
    !token.is_empty()
        && token.starts_with(|c: char| c.is_ascii_lowercase())
        && token
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Every `pub mod` in `lib.rs` — the modules `tests/` can reach.
fn pub_mods(source: &str) -> Vec<&str> {
    source
        .lines()
        .filter_map(|line| line.strip_prefix("pub mod ")?.strip_suffix(';'))
        .map(str::trim)
        .collect()
}
