//! Check 4: every third-party `uses:` is a full commit SHA with a trailing release label, so a
//! moved tag cannot change what CI runs.

use super::Failure;
use super::support::{rel, workflow_files};
use crate::util::lines_without_opt_out;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

/// Every third-party `uses:` is a full commit SHA plus a trailing release label, so a
/// compromised or force-moved tag cannot change what CI runs. actionlint validates workflow
/// syntax and inputs but says nothing about what a `uses:` resolves to, so this is the only
/// thing standing between a mutable tag and the release pipeline.
///
/// to pin. A deliberate exception opts out with the marker on the line.
pub(super) fn action_pins(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let label = Regex::new(r"#[\t ]*v[0-9]").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some(uses_at) = line.find("uses:") else {
                continue;
            };
            let spec = line[uses_at + "uses:".len()..]
                .trim_start()
                .split(|c: char| c.is_whitespace() || c == '#')
                .next()
                .unwrap_or("");
            if spec.starts_with("./") || spec.starts_with("../") || spec.starts_with("docker://") {
                continue;
            }
            let Some((_, reference)) = spec.split_once('@') else {
                continue;
            };
            let after_spec = &line[line.find(spec).map_or(line.len(), |at| at + spec.len())..];
            let why = if reference.len() != 40
                || !reference
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                Some(format!(
                    "pins {reference}, a mutable ref, not a 40-character commit SHA"
                ))
            } else if !label.is_match(after_spec) {
                Some("pins a SHA with no trailing release label, so the next person cannot tell what it is".to_string())
            } else {
                None
            };
            if let Some(why) = why {
                findings.push(format!("{}:{number}:{why}", rel(root, &file)));
            }
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a third-party action is not pinned to a full commit SHA (see .knowledge/publish-automation.md):",
            findings,
            "resolve the tag with 'gh api repos/OWNER/REPO/git/ref/tags/TAG', dereference it if it is an annotated tag, and pin it as owner/repo@<sha> # vX.Y.Z",
        ));
    }
    Ok(())
}
