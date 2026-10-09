//! The executable half of the BDD layer (task-86): binds the Gherkin scenarios in
//! `tests/features/` to step definitions registered in the rstest-bdd registry.
//!
//! Steps register themselves with rstest-bdd's own `step!` macro, so matching (the
//! `rstest-bdd-patterns` engine), the step registry, `StepContext` world storage and the
//! `StepExecution` outcome type are all the published crate's. What lives here is only the
//! scenario driver the `#[scenario]` attribute macro would otherwise generate: parse the
//! feature with the `gherkin` crate, resolve every sentence in the registry, execute it
//! against an owned world cell. One `#[test]` per scenario calls [`run_scenario`], so every
//! scenario appears in the ordinary test count and failures name the feature, the scenario
//! and the step.
//!
//! The attribute macros themselves (`#[given]`, `#[scenario]`, …) ship in a separate
//! `rstest-bdd-macros` crate that the airlocked vendor set cannot carry; see `CONTEXT.md`
//! § Session scratchpad, 2026-10-09 second session. If a connected session ever vendors it,
//! this runner is the piece `#[scenario]` replaces — the feature files stay.
#![allow(dead_code)] // compiled into every integration-test binary; only the bdd_* ones use it

use rstest_bdd::{StepContext, StepExecution, StepKeyword, StepText, find_step_with_mode};
use std::any::Any;
use std::path::Path;

/// The fixture name every BDD world is stored under in its `StepContext`.
pub const WORLD: &str = "world";

/// Run one named scenario from a `.feature` file against the steps registered in this test
/// binary. `feature` is relative to the crate root (`tests/features/…`). The world starts as
/// `W::default()`; steps borrow it mutably through the context.
///
/// # Panics
/// When the feature file is missing or unparsable, the scenario name is absent, a sentence
/// has no registered step definition, or a step reports failure — each panic names the
/// feature, scenario and step it died on.
pub fn run_scenario<W: Any + Default>(feature: &str, scenario_name: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(feature);
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{} does not read: {error}", path.display()));
    let parsed = gherkin::Feature::parse(&source, gherkin::GherkinEnv::default())
        .unwrap_or_else(|error| panic!("{} does not parse as Gherkin: {error}", path.display()));
    let scenario = parsed
        .scenarios
        .iter()
        .find(|scenario| scenario.name == scenario_name)
        .unwrap_or_else(|| {
            panic!(
                "{feature} has no scenario named {scenario_name:?}; it carries: {}",
                parsed
                    .scenarios
                    .iter()
                    .map(|scenario| scenario.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        });

    let cell = StepContext::owned_cell(W::default());
    let mut ctx = StepContext::default();
    ctx.insert_owned::<W>(WORLD, &cell);

    let mut effective = StepKeyword::Given;
    for (index, step) in scenario.steps.iter().enumerate() {
        let keyword = match step.keyword.trim() {
            "Given" => StepKeyword::Given,
            "When" => StepKeyword::When,
            "Then" => StepKeyword::Then,
            // `And`, `But` and `*` inherit the keyword of the step they extend.
            "And" | "But" | "*" => effective,
            other => panic!(
                "{feature}, scenario {scenario_name:?}: unknown gherkin keyword {other:?} \
                 (step {index})"
            ),
        };
        effective = keyword;

        let definition = find_step_with_mode(keyword, StepText::from(step.value.as_str()))
            .unwrap_or_else(|| {
                panic!(
                    "{feature}, scenario {scenario_name:?}: no step definition for \
                         {keyword:?} \"{}\" (step {index})",
                    step.value
                )
            });
        let outcome = (definition.run)(&mut ctx, &step.value, None, None).unwrap_or_else(|error| {
            panic!(
                "{feature}, scenario {scenario_name:?}: step {index} \
                     ({keyword:?} \"{}\") failed: {error}",
                step.value
            )
        });
        assert!(
            matches!(outcome, StepExecution::Continue { .. }),
            "{feature}, scenario {scenario_name:?}: step {index} skipped — this layer has \
             no pending steps"
        );
    }
}

/// The registry guard the macro layer enforces at compile time: two steps in the same test
/// binary must not claim the same keyword and pattern.
///
/// # Panics
/// Lists every duplicated (keyword, pattern) pair with both source locations.
pub fn assert_no_duplicate_steps() {
    let duplicates = rstest_bdd::duplicate_steps();
    assert!(
        duplicates.is_empty(),
        "duplicate step definitions in this binary: {}",
        duplicates
            .iter()
            .map(|group| {
                let locations = group
                    .iter()
                    .map(|step| format!("{}:{}", step.file, step.line))
                    .collect::<Vec<_>>()
                    .join(" and ");
                format!(
                    "{:?} \"{}\" at {locations}",
                    group[0].keyword,
                    group[0].pattern.as_str()
                )
            })
            .collect::<Vec<_>>()
            .join("; ")
    );
}
