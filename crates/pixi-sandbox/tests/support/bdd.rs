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
//!
//! This module also holds the one guard the macro bindings need and the runner never did:
//! [`assert_every_scenario_is_bound`], because with `#[scenario]` the binding *is* the test, so
//! a binding lost in a refactor takes a scenario out of the suite without failing anything.
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

/// The binding oracle `#[scenario]` needs and the hand-rolled runner could not: every scenario
/// a feature file declares must still carry a `#[scenario]` binding in the test root that owns
/// it.
///
/// `#[scenario]` *is* what turns a Gherkin scenario into a test, so a binding lost in a
/// refactor removes the scenario from the suite without failing anything — the count just drops
/// by one and nothing in the tree notices. This reads the test root's own source and compares
/// the scenario names the feature declares against the `name = "…"` selectors bound there,
/// naming every scenario that has no binding.
///
/// Source text rather than the compiled test list, on purpose: a platform-gated binding (the
/// user-tools pre-policy scenario) is absent from the binary on a foreign host while still
/// bound in the source, and must not be reported as missing. Selectors are read as `name =`,
/// the house form — an `index = N` selector says nothing readable about which scenario it binds
/// once a feature file is reordered, and this guard would not recognise it.
///
/// # Panics
/// When the feature file or the test root does not read, the feature does not parse as Gherkin,
/// or at least one scenario has no binding — the message names the feature, the test root and
/// every unbound scenario.
pub fn assert_every_scenario_is_bound(feature: &str, test_root: &str) {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let feature_path = crate_dir.join(feature);
    let source = std::fs::read_to_string(&feature_path)
        .unwrap_or_else(|error| panic!("{} does not read: {error}", feature_path.display()));
    let parsed =
        gherkin::Feature::parse(&source, gherkin::GherkinEnv::default()).unwrap_or_else(|error| {
            panic!(
                "{} does not parse as Gherkin: {error}",
                feature_path.display()
            )
        });
    let root_path = crate_dir.join(test_root);
    let root_source = std::fs::read_to_string(&root_path)
        .unwrap_or_else(|error| panic!("{} does not read: {error}", root_path.display()));

    let bound = scenario_selectors(&root_source);
    let unbound: Vec<&str> = parsed
        .scenarios
        .iter()
        .map(|scenario| scenario.name.as_str())
        .filter(|name| !bound.iter().any(|selector| selector == name))
        .collect();
    assert!(
        unbound.is_empty(),
        "{feature}: {} scenario(s) carry no #[scenario] binding in {test_root}: {}. A scenario \
         without a binding is a scenario the suite silently stopped running.",
        unbound.len(),
        unbound
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
}

/// Every `name = "…"` selector in a test root's source, whitespace between the three tokens
/// allowed but not inside the literal — so a `#[scenario]` attribute rustfmt broke across lines
/// reads the same as one it left alone.
fn scenario_selectors(source: &str) -> Vec<String> {
    let bytes: Vec<char> = source.chars().collect();
    let mut found = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if !matches_token(&bytes, index, "name") {
            index += 1;
            continue;
        }
        let mut cursor = index + "name".chars().count();
        cursor = skip_space(&bytes, cursor);
        if bytes.get(cursor) != Some(&'=') {
            index += 1;
            continue;
        }
        cursor = skip_space(&bytes, cursor + 1);
        if bytes.get(cursor) != Some(&'"') {
            index += 1;
            continue;
        }
        cursor += 1;
        let start = cursor;
        while cursor < bytes.len() && bytes[cursor] != '"' {
            cursor += 1;
        }
        found.push(bytes[start..cursor].iter().collect());
        index = cursor + 1;
    }
    found
}

/// Whether `token` starts at `index` and is not part of a longer identifier.
fn matches_token(bytes: &[char], index: usize, token: &str) -> bool {
    let length = token.chars().count();
    if index + length > bytes.len()
        || bytes[index..index + length].iter().collect::<String>() != token
    {
        return false;
    }
    let is_identifier_edge = |character: &char| character.is_alphanumeric() || *character == '_';
    let before_is_identifier = index > 0 && is_identifier_edge(&bytes[index - 1]);
    let after_is_identifier = bytes.get(index + length).is_some_and(is_identifier_edge);
    !before_is_identifier && !after_is_identifier
}

fn skip_space(bytes: &[char], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_whitespace() {
        index += 1;
    }
    index
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
