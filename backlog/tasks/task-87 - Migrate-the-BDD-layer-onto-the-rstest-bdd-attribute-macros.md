---
id: TASK-87
title: Migrate the BDD layer onto the rstest-bdd attribute macros
status: Done
assignee: []
created_date: '2026-10-09 15:54'
updated_date: '2026-10-09 19:36'
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
- [x] #1 Every scenario in `crates/pixi-sandbox/tests/features/` is bound by `#[scenario]` and every step definition is a `#[given]`/`#[when]`/`#[then]` function whose world arrives as a fixture-injected parameter, borrowed mutably to mutate and shared to assert; the hand-rolled driver is gone — `run_scenario`, `WORLD` and `assert_no_duplicate_steps` no longer exist in `tests/support/bdd.rs`, and no step code lands in `src/`.
- [x] #2 The three `.feature` files are byte-identical before and after (`git diff --exit-code -- crates/pixi-sandbox/tests/features/`): scenarios and sentences do not move. Each scenario keeps the test name nextest reports today, and the platform-specific user-tools scenario keeps the `#[cfg]` it binds behind now.
- [x] #3 The suite count does not fall below its 816 baseline: the three duplicate-registry guards are replaced by one binding-completeness guard per test binary, asserting that every scenario in that binary's feature file still carries a `#[scenario]` binding. The guard is proven red by removing a binding before it is trusted green.
- [x] #4 A missing or unbound step is a compile error, not a warning: `rstest-bdd-macros` is declared with `strict-compile-time-validation`, proven by a deliberate red compile (a duplicated pattern, then an unbound sentence), and the `Cargo.toml` comment states accurately what each feature enforces rather than overclaiming the non-strict one.
- [x] #5 No lockfile change (`Cargo.lock`, `pixi.lock`, `bun.lock` byte-identical) and no product change (`crates/*/src` byte-identical); the fmt, lint, test and check-repository gates stay green; and `AGENTS.md` § Test conventions describes where feature files and step definitions live after the migration, naming the binding guard instead of the retired runner.
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09 — landed as five commits on the session branch `arena/49e36983-pixi-sandbox`, one per slice, each with the tree green at that commit: `docs(backlog)` for the paperwork, one `test(bdd)` per binary, then the runner's retirement. Every acceptance criterion below is proven locally on this machine; **the work is not pushed and no pull request is open**, because that sanction was explicitly reserved. The five ACs are checked on the evidence, and the status stays `In Progress` until it lands on main — a task marked Done whose commits exist only on an unpushed branch is exactly the stale claim §5 warns about, and flipping it is a one-line edit the moment the merge happens.

**Red before green, per the TDD skill.** Three deliberate reds were produced and reverted, so each new guarantee was seen failing before it was trusted:

- *The binding oracle.* Two `#[scenario]` bindings pointed at one scenario, leaving the third unbound → `tests/features/transport_integrity.feature: 1 scenario(s) carry no #[scenario] binding in tests/bdd_transport_integrity.rs: "A tampered shard part is named in the refusal"`.
- *A duplicated step pattern.* Declared **above** the bindings → `error: Ambiguous step definition for 'a packed transport'` at the `#[scenario]` site, listing both patterns. Declared **below** them → compiles, then rstest-bdd's own registry panics at `registry/mod.rs:238`: `duplicate step for 'Given' + 'a packed transport' defined at crates/pixi-sandbox/tests/bdd_transport_integrity.rs:58`, failing all three scenarios. The compile-time check is therefore **expansion-order dependent**, and the runtime registry is what covers the other half — which is the honest basis for retiring `assert_no_duplicate_steps()`: the published crate covers that ground twice, once earlier and once more precisely than the hand-rolled guard did.
- *An unmatched sentence.* `Given a packed transport` typo'd to `transportt` → `error: No matching step definition found for 'Given a packed transportt'`, listing the three definitions that do exist. The feature file was restored with `git checkout` and re-verified byte-identical.

- **AC#1** — all **39 steps** across the three binaries are now `#[given]`/`#[when]`/`#[then]` functions whose world arrives as a fixture-injected parameter (borrowed mutably by the steps that stage and drive, shared by the ones that only assert), each root declaring its world as a zero-argument `#[fixture]`; all **11 scenarios** are bound by `#[scenario]`, which emits the rstest test itself, so no `#[test]` wrapper is hand-written any more. `run_scenario`, `WORLD` and `assert_no_duplicate_steps` no longer exist — a grep over `crates/pixi-sandbox/tests/` returns nothing — and no step code lands in `src/`, which the byte-identity proof below covers rather than asserts.
- **AC#2** — `git diff --exit-code 8dc5bba -- crates/pixi-sandbox/tests/features/` returns 0: all three `.feature` files are byte-identical, not one sentence moved. The scenario tests keep the names nextest reported before the migration (`scenario_a_packed_transport_round_trips_through_verification`, `scenario_a_verified_restore_registers_the_tools_by_default`, and the rest), verified by reading the test output rather than by intention. The pre-policy user-tools scenario keeps `#[cfg(all(target_os = "linux", target_arch = "x86_64"))]` on its binding *and* on its four steps — the cfg moved onto the step attributes themselves, which is why four cfg attributes left with the `step!` registry block instead of being dropped.
- **AC#3** — the suite is **816 passing / 1 skipped**, identical to the baseline (27 git · 209 core · 425+1 pixi-sandbox · 155 xtask). The three duplicate-registry guards were replaced one-for-one by three `every_scenario_in_the_feature_file_is_bound` tests, so the count held by construction rather than by luck, and the oracle was proven red before it was trusted.
- **AC#4** — `rstest-bdd-macros` is declared with `strict-compile-time-validation`, which implies `compile-time-validation` and adds no dependency, so `Cargo.lock` did not move. Both halves proven red above: an unmatched sentence is a compile error, and an ambiguous one is too. The `Cargo.toml` comment now says what each feature actually enforces — the previous text claimed `compile-time-validation` alone "keeps a missing step a compile error", which the vendored source contradicts (`validation/steps/mod.rs:218`: `strict` decides; non-strict only warns).
- **AC#5** — `git diff --exit-code 8dc5bba -- Cargo.lock pixi.lock bun.lock` and `-- 'crates/*/src'` both return 0: no lockfile change and no product change. Gates: `pixi run --frozen fmt` clean, `pixi run --frozen lint` **11/11** turbo tasks green (per-crate clippy `-D warnings`, deny, biome, actionlint, taplo, generated-workflow, publish-plan, `xtask check-repository`), `pixi run --frozen test` **4/4** green at 816/1, and convco on every commit message. `AGENTS.md` § Test conventions now describes the post-migration layout — fixture-injected world parameters, `#[scenario]` emitting the test, strict validation, and the binding oracle with the reason it reads source text rather than the compiled test list.

**The clippy unknown the plan carried is resolved, and it went the easy way.** `cargo clippy -p pixi-sandbox --all-targets -- -D warnings` judges the integration tests, so it judges the `#[fixture]`/`#[given]`/`#[scenario]` expansions directly. Upstream suppresses those with its own `rstest_bdd_test_macros::allow_fixture_expansion_lints`, and that helper crate is **not vendored** — the house needs no equivalent. The only clippy error this migration produced was in the new oracle's own code (`redundant_closure` on an `is_some_and` closure), fixed in the pilot slice.

**No assertion was weakened, audited rather than assumed.** Comparing every string literal of at least twelve characters in each migrated file against its pre-migration version at `8dc5bba` shows exactly two losses across all three files: `"the runner inserts the world"` — the expect message of the retired `world(ctx)` helper — and four `", target_arch = "` fragments from the cfg attributes the `step!` registry block carried. Everything gained is the oracle's two path arguments. One gratuitous edit made along the way (shortening an unrelated `expect("the profile exists")`) was caught by the same audit and reverted.

**Net shape:** 367 insertions against 795 deletions in code and docs — **428 lines removed** while the suite count stayed at 816 and the specification did not move a byte.

**One tooling finding, recorded in `CONTEXT.md` rather than acted on.** The backlog CLI cannot carry prose through `pixi run`: pixi joins task arguments into a command line without quoting them, so spaces split arguments, backticks are command-substituted by the inner shell, and an embedded newline abandons the invocation and prints the task list. This file was therefore written directly and validated by a CLI read-back (`backlog task 87 --plain`), which reads backticks, em dashes and `§` correctly. `.agents/skills/backlog/SKILL.md` advises repeating `--append-*` once per line, which does not survive that argument splitting — a real correction owed to that skill, left unimplemented per invariant 10.

**2026-10-09 integration update.** The earlier no-push/no-PR paragraphs describe the state
when the implementation was recorded. The seven TASK-87/session commits were subsequently
pushed through `d8f14cf8`; the owner has now explicitly authorized creating and merging the
combined TASK-87/TASK-82 pull request. All five TASK-87 acceptance criteria were already
proven locally, so the status closure is included in that PR in house format rather than
requiring a separate post-merge main push. TASK-82 adds the independent pack extraction;
it does not change the specification or the lockfiles. The combined branch baseline is
**873 passing / 1 skipped**. PR and post-merge check verdicts must still be verified, not
inferred from the previous main's successful runs.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The BDD layer is on the macros. All three feature binaries — transport integrity, restore verification, user-tools policy — now bind their eleven scenarios with `#[scenario]` and their thirty-nine steps with `#[given]`/`#[when]`/`#[then]`, each step an ordinary function on a fixture-injected world instead of a four-argument `StepContext` callback returning `Result<StepExecution, StepError>`. The hand-rolled driver in `tests/support/bdd.rs` is gone: `run_scenario`, the `WORLD` key and `assert_no_duplicate_steps` no longer exist, and what remains in that module is the one thing the macros cannot do — notice a scenario whose binding was lost, which with `#[scenario]` would leave the suite silently rather than fail.

The specification did not move. All three `.feature` files are byte-identical to `8dc5bba`, the scenario tests keep the names nextest already reported, and the platform-gated pre-policy scenario keeps its `#[cfg]` on both binding and steps. Nothing else moved either: no lockfile change, no product change, no scenario added, no assertion weakened — each proven with `git diff --exit-code` and with a string-literal audit against the pre-migration files rather than asserted in a commit message.

The suite is 816 passing / 1 skipped, unchanged, because three binding oracles replaced three duplicate-registry guards one for one. `strict-compile-time-validation` now makes an unmatched or ambiguous sentence a build failure instead of a warning or a runtime surprise, and it costs nothing in the lockfile. 428 lines net removed. Gates: fmt clean, lint 11/11, test 4/4, convco on every commit.

Two things this session learned that outlive it: the macros' compile-time ambiguity check is **expansion-order dependent** — a duplicate declared below its bindings compiles, and rstest-bdd's runtime registry is what catches it, naming file and line — and the macro expansions are **clippy-clean under `-D warnings --all-targets`**, so the house needs no equivalent of upstream's `allow_fixture_expansion_lints`.

Integration: all five acceptance criteria are proven and checked, and the owner has authorized the combined TASK-87/TASK-82 PR and merge. Status Done is included in the PR's task closure. TASK-87 itself preserves 816/1; the subsequent TASK-82 extraction raises the combined branch to 873 passing / 1 skipped. The earlier reserved-sanction notes are historical; current integration proof belongs to the PR and its post-merge runs.
<!-- SECTION:SUMMARY:END -->
