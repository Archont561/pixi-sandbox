//! Shared plumbing for the repository-automation commands.
//!
//! Everything here is pure or takes explicit paths: D10 forbids a test from inspecting this
//! checkout, so every function must be drivable from a tempdir fixture, and only `main.rs`
//! ever hands over the real repository root at runtime.

use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

/// The documented opt-out marker: a line that must name a forbidden thing to forbid it
/// (or legitimately cite an older release) carries this on the line or the line above.
pub const OPT_OUT: &str = "stale-ref-allowed";

/// 1-based line numbers paired with lines, skipping any line that carries the opt-out marker
/// on itself or on the line directly above (rustfmt owns where a trailing comment lands).
pub fn lines_without_opt_out(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut prev_opted = false;
    text.lines().enumerate().filter_map(move |(idx, line)| {
        let line_opted = line.contains(OPT_OUT);
        let skip = line_opted || prev_opted;
        prev_opted = line_opted;
        if skip { None } else { Some((idx + 1, line)) }
    })
}

/// A literal `vX.Y.Z` release tag, the token both the reference scan and the workflow-tag
/// check hunt for.
pub fn version_tag_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"v[0-9]+\.[0-9]+\.[0-9]+").expect("static regex"))
}

/// Strict semver core: `X.Y.Z` with nothing else. Versions flow into git tags, download URLs
/// and file contents, so anything looser is rejected at the door.
pub fn is_strict_semver(version: &str) -> bool {
    let mut parts = version.split('.');
    let three = [parts.next(), parts.next(), parts.next()];
    parts.next().is_none()
        && three
            .iter()
            .all(|p| p.is_some_and(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit())))
}

pub fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Write through a sibling temp file and an atomic rename, so a failure part-way never leaves
/// a half-written file behind (the release preparation edits a dozen files in sequence).
pub fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let tmp = tempfile::NamedTempFile::new_in(parent)
        .with_context(|| format!("creating a temp file beside {}", path.display()))?;
    fs::write(tmp.path(), contents)
        .with_context(|| format!("writing a temp copy of {}", path.display()))?;
    tmp.persist(path)
        .with_context(|| format!("replacing {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_out_skips_the_line_and_the_line_below_the_marker() {
        let text = "one\nstale-ref-allowed\ntwo\nthree";
        let kept: Vec<_> = lines_without_opt_out(text).collect();
        assert_eq!(kept, vec![(1, "one"), (4, "three")]);
    }

    #[test]
    fn strict_semver_rejects_prefixes_suffixes_and_missing_parts() {
        assert!(is_strict_semver("0.3.6"));
        assert!(is_strict_semver("10.20.30"));
        for bad in ["v0.3.6", "0.3", "0.3.6.1", "0.3.6-rc1", "", "a.b.c"] {
            assert!(!is_strict_semver(bad), "{bad} must be rejected");
        }
    }

    #[test]
    fn version_tags_are_found_mid_line() {
        let caps: Vec<_> = version_tag_re()
            .find_iter("uses x@v1.2.3 and v10.0.1")
            .map(|m| m.as_str())
            .collect();
        assert_eq!(caps, vec!["v1.2.3", "v10.0.1"]);
    }
}
