//! The single definition of "a reference to OUR release", used by both the lint that reports
//! the drift (`check-repository`) and the release preparation that fixes it.
//!
//! Why one module and not a predicate pasted into each caller: the bug this exists to kill is
//! a release commit that bumps the manifests and leaves the ~90 documentation references on
//! the previous tag. The check and the fix must agree on exactly which references are ours,
//! and two hand-maintained copies of the same predicate are two things that drift.
//!
//! What counts as OURS — a line qualifies when it names this project:
//!   * `Archont561/pixi-sandbox[/subpath]@vX.Y.Z`   (a `uses:` ref or a submodule example)
//!   * `releases/download/vX.Y.Z/<asset>`            (the install one-liner and its siblings)
//!   * `version: vX.Y.Z`                             (the setup action's version input)
//!   * `PIXI_SANDBOX_VERSION=vX.Y.Z`                 (the environment override)
//!   * `` `uses: @vX.Y.Z` ``                         (the bare form used in prose)
//!
//! A third-party pin on the same line (`actions/checkout@v7.0.1`, `setup-pixi@v0.10.2`) is
//! someone else's release: this repository is deliberately not the authority on `setup-pixi`'s
//! version, so such a line is skipped entirely rather than rewritten.
//!
//! A reference that must stay on an older release (an upgrade walkthrough, a regression
//! fixture) opts out with `stale-ref-allowed` on the line or the line above, and is left
//! alone by both modes.
//!
//! The docs site (docs/src/content) is NOT in the stamped set: it derives the version at
//! build time. Its pages write `v__VERSION__` and docs/astro.config.mjs substitutes the
//! workspace version, so a release rewrites zero documentation lines there. What `scan`
//! asserts for docs is therefore the inverse claim: an ours-line must carry NO literal tag at
//! all — a literal would be correct today and silently stale after the next cut, which is the
//! exact rot the derivation exists to kill.

use crate::util::{OPT_OUT, version_tag_re};
use anyhow::{Context, Result, bail};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Canonical upstream identity, independent of interfaces that releases may retire.
pub fn project_root_ident(_root: &Path) -> Result<String> {
    Ok("Archont561/pixi-sandbox".to_string())
}

/// Does this line name our project? (See the module comment for the full inventory.)
fn is_ours(line: &str, root_ident: &str) -> bool {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(
            r"releases/download/v[0-9]|version: v[0-9]|PIXI_SANDBOX_VERSION=v[0-9]|`uses: @v[0-9]",
        )
        .expect("static regex")
    });
    line.contains(root_ident) || re.is_match(line)
}

/// The files that carry LITERAL references (the README family): GitHub renders them raw, so
/// there is no build step to derive a version in. These are what `rewrite` stamps at release
/// time. Markdown and MDX only, and never the places that are historical by construction: a
/// changelog and a closed task record are *supposed* to name old releases; `.knowledge/`
/// holds design history; `docs/` derives.
fn ref_files(root: &Path) -> Vec<PathBuf> {
    const EXCLUDED_TOP_DIRS: [&str; 5] = [".git", ".knowledge", "node_modules", "docs", "backlog"];
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root)
        .min_depth(1)
        .into_iter()
        .filter_entry(|entry| {
            // Only top-level exclusions, mirroring the retired `find -path './dir/*'` set: an
            // exclusion buried deeper would silently hide a real reference from both modes.
            if entry.depth() == 1 && entry.file_type().is_dir() {
                let name = entry.file_name().to_string_lossy();
                return !EXCLUDED_TOP_DIRS.contains(&name.as_ref());
            }
            true
        })
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|x| x == "md" || x == "mdx"))
        .filter(|p| p.file_name().is_none_or(|n| n != "CHANGELOG.md"))
        .collect();
    files.sort();
    files
}

/// The files that DERIVE the version (the docs site's content). `scan` checks these for the
/// opposite defect — a literal tag where `v__VERSION__` belongs — and `rewrite` never
/// touches them.
fn docs_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root.join("docs/src/content"))
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(walkdir::DirEntry::into_path)
        .filter(|p| p.extension().is_some_and(|x| x == "md" || x == "mdx"))
        .collect();
    files.sort();
    files
}

fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Report every reference of ours that disagrees with the declared version (README family)
/// and every literal tag of ours in the derived docs. Empty means consistent.
pub fn scan(root: &Path) -> Result<Vec<String>> {
    let current = format!("v{}", crate::version::workspace_version(root)?);
    let root_ident = project_root_ident(root)?;
    let mut findings = Vec::new();

    for file in ref_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in crate::util::lines_without_opt_out(&text) {
            if !is_ours(line, &root_ident) {
                continue;
            }
            for tag in version_tag_re().find_iter(line) {
                if tag.as_str() != current {
                    findings.push(format!(
                        "{}:{}: pins {}, manifests declare {}",
                        display_rel(root, &file),
                        number,
                        tag.as_str(),
                        current
                    ));
                }
            }
        }
    }

    for file in docs_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in crate::util::lines_without_opt_out(&text) {
            if !is_ours(line, &root_ident) {
                continue;
            }
            for tag in version_tag_re().find_iter(line) {
                findings.push(format!(
                    "{}:{}: pins the literal {} — the docs derive the version; write v__VERSION__",
                    display_rel(root, &file),
                    number,
                    tag.as_str()
                ));
            }
        }
    }

    Ok(findings)
}

/// Move every reference to OUR release in the README family to `new_tag`; returns how many
/// files changed. Destination-only, deliberately: by the time this runs the tree already
/// declares the NEW version (the manifests are stamped first), so an "old tag" guard argument
/// cannot be checked against anything and is not taken. Whatever the docs said, they now say
/// the declared release — which is the same thing `scan` asserts afterwards.
///
/// Rewrites are byte-faithful outside the approved lines: content is carried through
/// `split_inclusive`, so line endings and a missing trailing newline survive untouched (the
/// retired shell pipeline once truncated 1978 lines of documentation by getting this wrong).
pub fn rewrite(root: &Path, new_tag: &str) -> Result<usize> {
    if new_tag.is_empty() {
        bail!("rewrite needs a destination tag");
    }
    let root_ident = project_root_ident(root).context("refusing to guess what to rewrite")?;
    crate::version::workspace_version(root)
        .context("Cargo.toml has no parsable workspace version")?;

    let mut touched = 0usize;
    for file in ref_files(root) {
        let text = crate::util::read(&file)?;
        let mut prev_opted = false;
        let rewritten: String = text
            .split_inclusive('\n')
            .map(|line| {
                let line_opted = line.contains(OPT_OUT);
                let qualifies = !line_opted && !prev_opted && is_ours(line, &root_ident);
                prev_opted = line_opted;
                if qualifies {
                    version_tag_re().replace_all(line, new_tag).into_owned()
                } else {
                    line.to_string()
                }
            })
            .collect();
        if rewritten != text {
            crate::util::write_atomic(&file, &rewritten)?;
            touched += 1;
        }
    }
    eprintln!("repin: {touched} file(s) moved to {new_tag}");
    Ok(touched)
}
