//! Check 6: no workflow pins a literal `vX.Y.Z` release tag, so no proof silently keeps
//! running against the previous release after a cut.

use super::Failure;
use super::support::{rel, workflow_files};
use crate::util::{lines_without_opt_out, version_tag_re};
use anyhow::Result;
use regex::Regex;
use std::path::Path;

/// A workflow that names a specific `vX.Y.Z` of ours is stale the moment a release lands,
/// and it fails silently: the job keeps running against old binaries and still reports green.
/// Scoped to a `version:` value, so an example in a description is not a false positive.
pub(super) fn workflow_literal_tags(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let version_key = Regex::new(r"(^|[\t ])version:[\t ]").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if !version_key.is_match(line) || line.contains("description:") {
                continue;
            }
            for tag in version_tag_re().find_iter(line) {
                findings.push(format!(
                    "{}:{number}: pins the literal {}",
                    rel(root, &file),
                    tag.as_str()
                ));
            }
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a workflow pins a literal release tag, so it will silently prove a stale release after the next cut:",
            findings,
            "read the tag from the checked-out Cargo.toml, or take it from a repository variable",
        ));
    }
    Ok(())
}
