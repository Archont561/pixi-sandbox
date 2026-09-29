---
id: TASK-6
title: Fix the stale not_yet error message referencing the deleted Python prototype
status: Done
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 07:26'
labels:
  - cli
  - cleanup
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/mod.rs
priority: medium
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The not_yet helper in commands/mod.rs tells the user that the Python prototype in .knowledge/research/ implements the verb end-to-end and is the reference for this port. That prototype was deleted in 0.2.0 (see the CHANGELOG entry: remove all Python script references) and .knowledge/research/ now holds only EVIDENCE.md and REPRODUCE-TRANSCRIPT.md. The message misleads an airlock operator who has no network to check.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The not_yet message points only at .knowledge/design.md sections and no longer mentions any Python prototype
- [x] #2 A grep across crates/ finds no Python-prototype references left
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
TDD: red CLI test in tests/cli.rs asserting a deferred verb (publish --keep, tools update) names .knowledge/design.md and mentions no Python prototype; green by rewriting not_yet in commands/mod.rs. AC2 via a new pixi lint task that greps crates/ (cargo tests stay fixture-only, D10).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Red: new tests/cli.rs test a_deferred_verb_cites_only_the_design_section drove both deferred verbs (publish --keep, tools update) through the CLI and asserted the stderr names .knowledge/design.md with its section and contains no python/prototype/.knowledge/research. It failed against the old wording.

Green: not_yet() in commands/mod.rs now cites only the design section. Module doc of commands/mod.rs, the two Linkage comments in core/src/verify.rs, the script fixture in core/tests/verify.rs, the actions.rs header and tests/fixtures/README.md lost their prototype references.

AC2 is executable as pixi run lint-repo-consistency (scripts/lint-repo-consistency.sh), not a cargo test: D10 forbids tests that target this repository. Opt-out marker: stale-ref-allowed on or above the line, used once by the test that must spell the forbidden words out.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Deferred verbs now cite .knowledge/design.md and nothing else; crates/ is free of references to the Python prototype deleted in 0.2.0, enforced by the new lint-repo-consistency task in the lint gate, CI and the pre-commit hook.
<!-- SECTION:FINAL_SUMMARY:END -->
