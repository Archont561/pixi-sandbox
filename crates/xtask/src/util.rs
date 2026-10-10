//! Shared plumbing for the repository-automation commands.
//!
//! Everything here is pure or takes explicit paths: D10 forbids a test from inspecting this
//! checkout, so every function must be drivable from a tempdir fixture, and only `main.rs`
//! ever hands over the real repository root at runtime.

use anyhow::{Context, Result};
use regex::Regex;
use std::fs::{self, OpenOptions};
use std::io::Write;
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
#[must_use]
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

/// Append one line to a workflow file, creating it if needed. Broken out because the file
/// mechanics are what a test can drive without mutating process-global environment (the
/// `$GITHUB_*` selection around it is two obvious lines each).
#[doc(hidden)] // test boundary: the one-line append rule (tests/util.rs)
pub fn append_line(target: &Path, line: &str) {
    let Ok(mut file) = OpenOptions::new().create(true).append(true).open(target) else {
        eprintln!("warning: cannot append to {}", target.display());
        return;
    };
    let _ = writeln!(file, "{line}");
}

/// Append `key=value` to `$GITHUB_OUTPUT`, so a workflow consumes the value as
/// `steps.<id>.outputs.<key>` (task-36 AC#4). No-op when the variable is absent: the command
/// keeps its stdout form, which is what a local run reads.
pub fn github_output(key: &str, value: &str) {
    if let Some(target) = std::env::var_os("GITHUB_OUTPUT") {
        append_line(Path::new(&target), &format!("{key}={value}"));
    }
}

/// Append markdown to `$GITHUB_STEP_SUMMARY`; prints to stdout when absent, so the same
/// invocation reports the same thing locally.
pub fn github_summary(markdown: &str) {
    match std::env::var_os("GITHUB_STEP_SUMMARY") {
        Some(target) => append_line(Path::new(&target), markdown),
        None => println!("{markdown}"),
    }
}
