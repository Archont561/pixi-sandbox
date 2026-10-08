---
id: TASK-86
title: Add BDD features with rstest-bdd for pixi-sandbox behaviour
status: To Do
assignee: []
created_date: '2026-10-08 23:07'
labels:
  - testing
  - bdd
dependencies: []
priority: medium
type: enhancement
ordinal: 86000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The suite proves behaviour with descriptive test names (rstest fixtures, proptest properties) but carries no executable specification: `a_failed_restore_is_reported_once_and_never_retried` reads as one assertion chain, not as the contract in the words a product discussion would use. A Gherkin feature states the behaviour first — scenario by scenario, in Given/When/Then — and the suite binds those sentences to the same fixtures and assertions the house already trusts.

rstest-bdd (`leynos/rstest-bdd`) is the binding layer: `.feature` files are parsed at compile time (via the `gherkin` crate), steps are ordinary Rust functions annotated `#[given]`/`#[when]`/`#[then]`, and `#[scenario]`/`scenarios!` generate one `#[rstest]` test per scenario — so every scenario appears in the ordinary `cargo test` count and failures name the scenario, with rstest fixture injection available underneath. It is a dev-only, compile-time dependency: nothing changes in the shipped product.

Intended behaviour: a BDD layer at `crates/pixi-sandbox/tests/features/` that states the product's observable behaviour in Gherkin — exit statuses, files and bytes on disk, report lines an operator reads — with step definitions under `tests/` driving the public API through the fixture builders `tests/support` already exposes. First slice, three features: restore verification (a verified restore writes the project; a failed verification writes nothing and keeps its evidence), transport integrity (a packed transport round-trips through verification; a tampered byte is refused before anything is written), and the user-tools policy (register, skip, and the honest-reporting paths). The layer complements the suite; it does not replace it.

Seam to agree before tests are written: per feature, the scenario list and which fixture builder each step binds to, so a step body stays glue and the assertions stay where the house puts them. Scenarios state outcomes, never internals.

One constraint came first and is now satisfied: a new crate cannot be fetched inside the airlock (egress allows github.com, not crates.io), so rstest-bdd landed the way task-38 landed rstest — declared at the workspace root (`rstest-bdd = "0.6.0"`) and as a dev-dependency of `pixi-sandbox`, with Cargo.lock refreshed by the connected relock workflow (2026-10-09). The crate's MSRV (1.88) sits below the workspace toolchain (1.98.1); the workspace's declared library MSRV 1.85 is unaffected because the crate is dev-only. A restored host compiles the suite only after the post-merge transport carries the new vendor crates (task-38's precedent: fresh restore, then `pixi run --frozen test`).

Non-goals: no replacement or rewriting of existing tests; no product behaviour change; no runtime dependency; no network; no feature-coverage requirement for every module — behaviour slices only.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 rstest-bdd is wired as a dev-only dependency and every Gherkin scenario runs as a plain cargo test test, visible in the suite count.
- [ ] #2 At least three .feature files under crates/pixi-sandbox/tests/features/ state observable behaviour in Gherkin: restore verification, transport integrity, and the user-tools policy.
- [ ] #3 Step definitions live under tests/ and drive the public API through the existing fixture builders; no step code lands in src/, and no scenario points at this repository or a real HOME.
- [ ] #4 No existing test is deleted or weakened: the suite count does not drop and the descriptive tests keep the coverage they have.
- [ ] #5 The test, lint and check-repository gates stay green, and AGENTS.md Test conventions describe where feature files and step definitions live.
<!-- AC:END -->
