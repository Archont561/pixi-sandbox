//! Repo-level consistency lints: the claims this repository makes about itself that no
//! ordinary cargo test is allowed to check.
//!
//! Why an xtask and not a `#[test]`: D10 — tests target fixtures, never this repository, and
//! `tests/fixtures.rs::no_test_targets_the_repository_root` enforces it. Every checker here
//! therefore takes an explicit root; only `main.rs` ever passes the real checkout, and
//! `tests/repo_checks*.rs` drive each policy against synthetic repositories in tempdirs.
//!
//! One file per numbered check (task-56), so each policy and its helpers sit together instead
//! of sharing one 1,000+ line module. [`CHECKS`] is the only thing that wires them into [`check_repository`]; `support` holds the handful of helpers more than one
//! check needs (`crate::util::lines_without_opt_out` is the other shared primitive, and it
//! already lived outside this subsystem before the split).
//!
//! The checks, in order (numbering preserved from the retired shell lint):
//!  1. task-6 — no reference under `crates/` to the deleted pre-Rust implementation.
//!  2. task-7 — the README Platforms badge, `pixi.toml` and `.pixi-sandbox.toml` tell one
//!     platform story, and the Windows gap is documented rather than advertised.
//!  3. task-2 — version references cannot drift (`release_refs::scan`), and the conda package
//!     manifest (the one file whose format demands a restated literal) equals Cargo.toml.
//!  4. every third-party `uses:` is a full commit SHA with a trailing release label, so a
//!     moved tag cannot change what CI runs.
//!  5. connected-host installation uses the canonical prefix.dev package channel in the
//!     README, installation guide, and generated publishing workflow (task-32).
//!  6. no workflow pins a literal `vX.Y.Z` release tag, so no proof silently keeps running
//!     against the previous release after a cut.
//!  7. retired with the composite Action surfaces in TASK-29.
//!  8. the surviving shell script stays on the Bash 3.2 surface: the darwin runners execute
//!     it with macOS's /bin/bash 3.2, and v0.3.6's release died 127 on both darwin legs
//!     over one Bash-4 builtin (run 36865921206) — the regression that moved everything else
//!     into this xtask.
//!  9. task-36 — every workflow `run:` is a single command line: a step is `uses:`, one
//!     `pixi run <task>` line, or a one-line host bootstrap. A `run: |` block with more than
//!     one command is embedded shell, which belongs in a pixi task (or an xtask subcommand
//!     when it carries logic) — the rule exists so the workflows cannot re-grow the ~280
//!     lines of per-dialect shell task-36 removed.
//! 10. task-39 — `.github/workflows/relock.yml` is the committed render of the generator
//!     `pixi-sandbox init` hands consumers, so the lane this repository runs is provably the
//!     lane it ships; `xtask render-relock` is the only way to change it.
//! 11. every `permissions:` scope is one GitHub accepts. An invented scope is not a weaker
//!     permission, it is a parse error that takes the whole workflow file down: a one-line
//!     `workflows: write` left `auto-release.yml` unrunnable, every run ending in 0s with no
//!     jobs and no diagnostic outside the Actions tab (run 37501052919).
//! 12. task-84 — `AGENTS.md` describes a tree that exists: no repo-map row names a path that
//!     has moved, and the promoted-module list is exactly the `pub mod` set of
//!     `crates/pixi-sandbox/src/lib.rs`. The document is what an agent reads as instruction,
//!     and D10 forbids the one thing that would otherwise catch the drift — a test that
//!     reads this repository.

pub mod agents_md;
mod badges;
mod bash32;
mod channel_drift;
mod conda_manifest;
mod mutable_refs;
mod release_tags;
mod relock;
mod stale_refs;
mod support;
pub mod workflow_permissions;
pub mod workflow_shape;

use anyhow::{Result, bail};
use std::path::Path;

pub struct Failure {
    pub headline: String,
    pub details: Vec<String>,
    pub hint: Option<String>,
}

impl Failure {
    fn new(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            details: Vec::new(),
            hint: None,
        }
    }
    fn with(headline: impl Into<String>, details: Vec<String>, hint: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            details,
            hint: Some(hint.into()),
        }
    }
}

type Check = fn(&Path, &mut Vec<Failure>) -> Result<()>;

/// One entry per numbered check still in force (7 retired in TASK-29), in the order the module
/// doc comment above describes them. [`check_repository`] runs this table start to finish and
/// collects every failure in one pass, so a red run names every problem at once instead of
/// stopping at the first one.
const CHECKS: &[Check] = &[
    |root, failures| {
        // Infallible by construction: its reads skip what they cannot open.
        stale_refs::stale_implementation_references(root, failures);
        Ok(())
    }, // 1
    badges::platform_claims,                    // 2
    conda_manifest::version_references,         // 3
    mutable_refs::action_pins,                  // 4
    channel_drift::canonical_channel_install,   // 5
    release_tags::workflow_literal_tags,        // 6
    bash32::bash32_surface,                     // 8
    workflow_shape::workflow_shape,             // 9
    relock::generated_relock_is_current,        // 10
    workflow_permissions::workflow_permissions, // 11
    agents_md::agents_md_matches_tree,          // 12
];

/// Run every check, returning all failures in one pass so a red run names every problem.
///
/// # Errors
///
/// Fails only when a check cannot run at all, such as an unreadable file. A check that finds a problem is returned as a [`Failure`] instead.
pub fn check_repository(root: &Path) -> Result<Vec<Failure>> {
    let mut failures = Vec::new();
    for check in CHECKS {
        check(root, &mut failures)?;
    }
    Ok(failures)
}

/// `xtask check-repository`: GitHub-annotated adapter over [`check_repository`].
///
/// # Errors
///
/// Fails when [`check_repository`] cannot run, or when any check reports a failure. Each failure is printed as a GitHub annotation first.
pub fn run(root: &Path) -> Result<()> {
    let failures = check_repository(root)?;
    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("::error::{}", failure.headline);
            for detail in &failure.details {
                eprintln!("  {detail}");
            }
            if let Some(hint) = &failure.hint {
                eprintln!("  {hint}");
            }
        }
        bail!("repo consistency: {} check(s) failed", failures.len());
    }
    eprintln!(
        "repo consistency: crates/ is free of prototype references; platform and version claims agree; action pins are immutable; connected-host docs and generated workflows use the canonical package channel; no workflow pins a literal release tag; the surviving shell script stays on the Bash 3.2 surface of the macOS runners; every workflow run: is a single command line; the committed relock workflow is the generator's current render; every permissions: scope is one GitHub accepts; AGENTS.md's repo map resolves and its promoted-module list matches lib.rs" // stale-ref-allowed
    );
    Ok(())
}
