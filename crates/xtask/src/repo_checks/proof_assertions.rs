//! Check 13 (doc-13 §1.2): the consumer proof may only assert strings the generated publisher
//! actually contains.
//!
//! The proof downloads the binary a release is about to publish, lets it generate a consumer
//! publisher, and then checks that publisher for the properties it claims to prove. Those
//! assertions live in this repository and the publisher they inspect is rendered by Rust in
//! another crate, so nothing but this check keeps the two in step — and the drift is silent in
//! the worst direction: the release publishes, the proof goes red afterwards, and the red run
//! looks like a consumer defect.
//!
//! That is exactly what happened at v0.6.1. TASK-76 moved the release download into
//! `pixi-sandbox fetch-release`, and `generated_workflow.rs` gained a test asserting the render
//! contains **no** `releases/download` literal, while `consumer-proof.yml` kept grepping for one
//! (run 38130260877, step "Render and regenerate consumer workflow", exit 1). The generator and
//! its assertion contradicted each other for a full release.
//!
//! **The assertions have moved, and this check moved with them.** Doc-13's slice 5 relocated the
//! greps into `release.yml`, which this check did not read — so from that point until TASK-89 it
//! scanned no literals at all and passed vacuously, while its fixtures stayed green because
//! `tests/support/mod.rs` injected a grep of its own. TASK-89 then ported the recipe into
//! `xtask`, and the two literals became [`PUBLISHER_ASSERTIONS`] in `consumer_proof.rs`. Both
//! homes are scanned: the module, and any `grep -F` a future edit adds back to the workflow.
//!
//! Scanning is not enough on its own, because the failure mode that actually shipped was not a
//! *wrong* assertion but *no* assertion — an empty list satisfies every filter. A module that
//! declares no `PUBLISHER_ASSERTIONS` is therefore a finding in its own right.

use super::Failure;
use anyhow::Result;
use regex::Regex;
use std::path::Path;

const PROOF_WORKFLOW: &str = ".github/workflows/consumer-proof.yml";
/// Where the ported assertions live (TASK-89): the proof's recipe, as tested Rust.
const PROOF_MODULE: &str = "crates/xtask/src/consumer_proof.rs";
const GENERATOR: &str = "crates/pixi-sandbox/src/generated/github_workflow.rs";
/// The const array [`PROOF_MODULE`] declares its assertions in.
const ASSERTIONS_CONST: &str = "PUBLISHER_ASSERTIONS";

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

/// Every string literal in the proof module's `PUBLISHER_ASSERTIONS` array.
///
/// Written against the declaration rather than the whole file so a string that merely mentions
/// a publisher property elsewhere in the module is not mistaken for an assertion of it.
fn module_literals(module: &str) -> Vec<String> {
    let declaration = Regex::new(&format!(r"{ASSERTIONS_CONST}[^=]*=\s*\[([^\]]*)\]"))
        .expect("the assertion pattern is a literal");
    let literal = Regex::new(r#""([^"]*)""#).expect("the literal pattern is a literal");
    let Some(found) = declaration.captures(module) else {
        return Vec::new();
    };
    literal
        .captures_iter(&found[1])
        .map(|each| each[1].to_string())
        .collect()
}

/// Fail when the consumer proof asserts a string the generator's own source no longer contains,
/// or when the ported proof has stopped asserting anything at all.
pub(super) fn proof_assertions_match_the_generator(
    root: &Path,
    failures: &mut Vec<Failure>,
) -> Result<()> {
    let proof_path = root.join(PROOF_WORKFLOW);
    let module_path = root.join(PROOF_MODULE);
    // A repository with neither home has nothing to keep in step; skipping is not optimism here
    // because the check is about a file pair, and one half is legitimately optional.
    if !proof_path.exists() && !module_path.exists() {
        return Ok(());
    }
    let generator = crate::util::read(&root.join(GENERATOR))?;

    let mut asserted = Vec::new();
    if module_path.exists() {
        let module = crate::util::read(&module_path)?;
        let literals = module_literals(&module);
        if literals.is_empty() {
            // Not a stale assertion but no assertion: the shape this check passed vacuously
            // over between doc-13's slice 5 and TASK-89. An empty list satisfies every filter,
            // so "nothing to compare" has to be a finding rather than a clean result.
            failures.push(Failure::with(
                "the consumer proof asserts nothing about the generated publisher",
                vec![format!("{PROOF_MODULE} declares no {ASSERTIONS_CONST}")],
                format!(
                    "the proof must assert what it proves. Declare the properties in {ASSERTIONS_CONST} so this check can hold them against {GENERATOR}; a proof that checks nothing passes every release and proves none of them"
                ),
            ));
        }
        asserted.extend(literals);
    }
    if proof_path.exists() {
        asserted.extend(asserted_literals(&crate::util::read(&proof_path)?));
    }

    let stale: Vec<String> = asserted
        .into_iter()
        .filter(|literal| !generator.contains(literal.as_str()))
        .collect();
    if !stale.is_empty() {
        failures.push(Failure::with(
            "consumer proof asserts strings the generated publisher does not contain (doc-13 §1.2)",
            stale
                .iter()
                .map(|literal| format!("the consumer proof asserts the publisher contains {literal:?}"))
                .collect(),
            format!(
                "assert only what {GENERATOR} renders. If the publisher changed on purpose, update the proof's assertion in the same commit; an assertion that outlives its string fails every release after the one that removed it"
            ),
        ));
    }
    Ok(())
}
