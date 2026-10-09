---
id: TASK-87
title: Migrate the BDD layer onto the rstest-bdd attribute macros
status: In Progress
assignee: []
created_date: '2026-10-09 15:54'
updated_date: '2026-10-09 15:54'
labels:
  - testing
  - bdd
dependencies:
  - TASK-86
references:
  - crates/pixi-sandbox/tests/support/bdd.rs
  - crates/pixi-sandbox/tests/bdd_transport_integrity.rs
  - crates/pixi-sandbox/tests/bdd_restore_verification.rs
  - crates/pixi-sandbox/tests/bdd_user_tools_policy.rs
  - crates/pixi-sandbox/tests/features
  - AGENTS.md
priority: medium
type: chore
ordinal: 87000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
task-86 bound the Gherkin layer through the vendored runtime half of rstest-bdd only: `.feature` files parsed by the `gherkin` crate, steps registered with rstest-bdd's own `step!` macro, and one thin driver in `tests/support/bdd.rs` doing what the `#[scenario]` attribute macro would otherwise codegen (parse the feature, resolve every sentence in the registry, execute it against an owned world cell). That shape was forced, not chosen — the attribute macros ship in a separate `rstest-bdd-macros` crate which the airlocked vendor set could not carry at the time (`CONTEXT.md` § Session scratchpad, 2026-10-09 second session).

The blocker is gone. `8dc5bba` declared `rstest-bdd-macros 0.6.0` with `compile-time-validation`, the relock bot refreshed `Cargo.lock` through PR #126, and the repacked `sandbox/developer-linux-64` transport vendors the macros crate and its whole tree (`rstest-bdd-harness`, `cap-std`, `camino`, `newt-hype`, `proc-macro-error3`, `convert_case`, `syn 3`), so the layer compiles offline on a restored host. This task takes the swap-in path that entry named: replace the runner with the macros' own bindings and leave the specification untouched.

Intended behaviour: every scenario is bound by `#[scenario(path, name)]` and every step is an ordinary function annotated `#[given]`/`#[when]`/`#[then]` whose world arrives as an rstest fixture parameter — borrowed mutably where the step mutates, shared where it only asserts — instead of a four-argument `StepContext` callback returning a `StepExecution` or a `StepError`. The world becomes a zero-argument `#[fixture]` in the test root, which is the shape `tests/support/mod.rs` already uses for its setup values. The three `.feature` files and their sentences are the specification and do not move: not one word, not one byte. `rstest-bdd`'s own vendored `tests/concurrent_mut_fixtures.rs` is the authoritative offline example of the target shape, including two mutable fixture borrows held at once in one step.

Two guards change job rather than disappear. `#[given]` expands to the same `rstest_bdd::step!` the hand-written registry used (`rstest-bdd-macros/src/codegen/wrapper/emit/mod.rs`), so the runtime registry is unchanged and `duplicate_steps()` would keep working; retiring it is therefore a choice, and it is only honest because of what replaces it. The macros' compile-time validation covers *ambiguity* as a hard error but does so per bound sentence, which is narrower ground than a whole-registry walk, and under `compile-time-validation` alone a *missing* step is a warning rather than an error — the `Cargo.toml` comment claiming otherwise is wrong and gets corrected here. Declaring `strict-compile-time-validation` (which implies the non-strict feature and adds no dependency, so no lockfile movement) makes a missing or unbound step a compile error, and `tests/support/bdd.rs` shrinks from scenario driver to binding oracle: one guard per test binary asserting that every scenario in its feature file still carries a `#[scenario]` binding. That is the failure mode this migration introduces and the old runner could not have — a scenario whose binding is forgotten silently leaves the suite, and nothing else in the tree would notice.

Non-goals: no new scenarios and no new feature files; no change to any `.feature` byte; no `Cargo.lock`, `pixi.lock` or `bun.lock` change; no product behaviour change; no weakening or deletion of any descriptive test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every scenario in `crates/pixi-sandbox/tests/features/` is bound by `#[scenario]` and every step definition is a `#[given]`/`#[when]`/`#[then]` function whose world arrives as a fixture-injected parameter, borrowed mutably to mutate and shared to assert; the hand-rolled driver is gone — `run_scenario`, `WORLD` and `assert_no_duplicate_steps` no longer exist in `tests/support/bdd.rs`, and no step code lands in `src/`.
- [ ] #2 The three `.feature` files are byte-identical before and after (`git diff --exit-code -- crates/pixi-sandbox/tests/features/`): scenarios and sentences do not move. Each scenario keeps the test name nextest reports today, and the platform-specific user-tools scenario keeps the `#[cfg]` it binds behind now.
- [ ] #3 The suite count does not fall below its 816 baseline: the three duplicate-registry guards are replaced by one binding-completeness guard per test binary, asserting that every scenario in that binary's feature file still carries a `#[scenario]` binding. The guard is proven red by removing a binding before it is trusted green.
- [ ] #4 A missing or unbound step is a compile error, not a warning: `rstest-bdd-macros` is declared with `strict-compile-time-validation`, proven by a deliberate red compile (a duplicated pattern, then an unbound sentence), and the `Cargo.toml` comment states accurately what each feature enforces rather than overclaiming the non-strict one.
- [ ] #5 No lockfile change (`Cargo.lock`, `pixi.lock`, `bun.lock` byte-identical) and no product change (`crates/*/src` byte-identical); the fmt, lint, test and check-repository gates stay green; and `AGENTS.md` § Test conventions describes where feature files and step definitions live after the migration, naming the binding guard instead of the retired runner.
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
One slice per test binary, additive before subtractive so the tree is green at every commit, and the pilot slice carries the two unknowns.

1. Backlog paperwork: this file, plus closing task-86 (5/5 ACs, status Done, NOTES and SUMMARY recording the runner deviation and its retirement here). No code.
2. Pilot — migrate `bdd_transport_integrity.rs` (3 scenarios, 10 steps, one `TransportWorld`). Smallest of the three, and the slice that resolves both unknowns empirically rather than in a commit message: (i) `cargo clippy -p pixi-sandbox --all-targets -- -D warnings` judges the `#[fixture]`/`#[given]` expansions directly, and upstream suppresses those with `rstest_bdd_test_macros::allow_fixture_expansion_lints` — a helper crate that is not vendored, so this slice finds out whether the house needs its own narrowly-scoped allow attributes; (ii) the compile-time check really fails on a duplicated pattern, proven by a deliberate red compile that is never committed. Also lands `strict-compile-time-validation`, the corrected `Cargo.toml` comment, and the new binding-completeness guard in `tests/support/bdd.rs` alongside the still-used runner.
3. Migrate `bdd_restore_verification.rs` (4 scenarios, 14 steps, `RestoreWorld`).
4. Migrate `bdd_user_tools_policy.rs` (4 scenarios, 15 steps, `PolicyWorld`) — the largest, and the one whose pre-policy scenario binds behind `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]`; the binding keeps that cfg, which is why the guard scans source text for bindings rather than counting compiled tests.
5. Retire the runner: delete `run_scenario`, `WORLD` and `assert_no_duplicate_steps`, keep `tests/support/bdd.rs` as the binding oracle, and rewrite the `AGENTS.md` § Test conventions paragraph so it states what is rather than what was (invariant 10).
6. Gates, then the `CONTEXT.md` § Session scratchpad entry.

Red before green per the TDD skill: each migration slice first proves the binding it is about to rely on fails when broken (a duplicated pattern must not compile; a removed binding must fail the guard), then lands the migration. Generated test function names stay the ones nextest reports today so the count stays comparable and the diff stays readable. The three immovable claims are proven with `git diff --exit-code`, not asserted: `tests/features/`, the three lockfiles, and `crates/*/src`.
<!-- SECTION:PLAN:END -->
