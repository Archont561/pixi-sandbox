//! Check 13 (doc-13 §1.2): the consumer proof may only assert strings the generated publisher
//! actually contains.
//!
//! The proof downloads a released binary, lets it generate a consumer publisher, and then greps
//! that publisher for the properties it claims to prove. Those greps live in a workflow and the
//! publisher they inspect is rendered by Rust in another crate, so nothing but this check keeps
//! the two in step — and the drift is silent in the worst direction: the release publishes, the
//! proof goes red afterwards, and the red run looks like a consumer defect.
//!
//! That is exactly what happened at v0.6.1. TASK-76 moved the release download into
//! `pixi-sandbox fetch-release`, and `generated_workflow.rs` gained a test asserting the render
//! contains **no** `releases/download` literal, while `consumer-proof.yml` kept grepping for one
//! (run 38130260877, step "Render and regenerate consumer workflow", exit 1). The generator and
//! its assertion contradicted each other for a full release.

use super::Failure;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

const PROOF_WORKFLOW: &str = ".github/workflows/consumer-proof.yml";
const GENERATOR: &str = "crates/pixi-sandbox/src/generated/github_workflow.rs";

/// Every `grep -F '<literal>'` in the proof that inspects a generated publisher.
///
/// Both quote styles are handled because the workflow's shell is written by hand. Written as an
/// alternation rather than a backreference: the `regex` crate deliberately does not support
/// backreferences, so each quote style captures into its own group.
fn asserted_literals(proof: &str) -> Vec<String> {
    let pattern = Regex::new(r#"grep -F (?:'([^']*)'|"([^"]*)") [^ ]*publish-sandbox\.yml"#)
        .expect("the assertion pattern is a literal");
    pattern
        .captures_iter(proof)
        .filter_map(|found| {
            found
                .get(1)
                .or_else(|| found.get(2))
                .map(|m| m.as_str().to_string())
        })
        .collect()
}

/// Fail when the consumer proof greps a generated publisher for a string the generator's own
/// source no longer contains.
pub(super) fn proof_assertions_match_the_generator(
    root: &Path,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let proof_path = root.join(PROOF_WORKFLOW);
    // A repository without the proof has nothing to keep in step; skipping is not optimism here
    // because the check is about a file pair, and one half is legitimately optional.
    if !proof_path.exists() {
        return Ok(());
    }
    let proof = crate::util::read(&proof_path)?;
    let generator = crate::util::read(&root.join(GENERATOR))?;

    let stale: Vec<String> = asserted_literals(&proof)
        .into_iter()
        .filter(|literal| !generator.contains(literal.as_str()))
        .collect();
    if !stale.is_empty() {
        failures.push(Failure::with(
            "consumer proof asserts strings the generated publisher does not contain (doc-13 §1.2)",
            stale
                .iter()
                .map(|literal| format!("{PROOF_WORKFLOW} greps the publisher for {literal:?}"))
                .collect(),
            format!(
                "assert only what {GENERATOR} renders. If the publisher changed on purpose, update the proof's grep in the same commit; a proof that greps a retired string fails every release after the one that removed it"
            ),
        ));
    }
    Ok(())
}
