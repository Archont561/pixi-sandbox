//! The binding oracle for the BDD layer (task-87).
//!
//! The scenarios themselves need no runner. `#[scenario]` from `rstest-bdd-macros` parses the
//! feature file, resolves every sentence against the step registry and emits the rstest test;
//! `#[given]`/`#[when]`/`#[then]` register the steps, expanding to the same `rstest_bdd::step!`
//! this layer used to write by hand, so matching, specificity, world storage and step outcomes
//! all stay the published crate's.
//!
//! What the macros do *not* check is whether every scenario a feature file declares still has a
//! binding. With `#[scenario]` the binding *is* the test, so one lost in a refactor takes a
//! scenario out of the suite without failing anything: the count drops by one and nothing in the
//! tree notices. Each `tests/bdd_<feature>.rs` root therefore carries one
//! [`assert_every_scenario_is_bound`] test beside its bindings.
//!
//! The duplicate-registry guard this module used to hold retired with the runner, because the
//! ground it walked is the published crate's now — twice over. Two patterns matching one
//! sentence is a compile error at the `#[scenario]` site, and a duplicate registration that
//! slips past macro-expansion order (a step declared *below* the bindings) panics in
//! rstest-bdd's own registry naming the file and line, which fails every scenario in the binary.
//! A hand-rolled check could only have reported the same thing later and less precisely.
#![allow(dead_code)] // compiled into every integration-test binary; only the bdd_* ones use it

use std::path::Path;

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

/// The index of the next non-whitespace character at or after `index`.
fn skip_space(bytes: &[char], mut index: usize) -> usize {
    while index < bytes.len() && bytes[index].is_whitespace() {
        index += 1;
    }
    index
}
