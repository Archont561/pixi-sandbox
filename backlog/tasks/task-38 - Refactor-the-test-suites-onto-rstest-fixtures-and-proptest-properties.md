---
id: TASK-38
title: Refactor the test suites onto rstest fixtures and proptest properties
status: In Progress
assignee: []
created_date: '2026-10-01 17:49'
updated_date: '2026-10-02 10:13'
labels:
  - testing
  - tooling
  - refactor
milestone: m-0
dependencies: []
references:
  - crates/pixi-sandbox/tests/restore_script.rs
  - crates/pixi-sandbox/tests/user_tools.rs
  - crates/pixi-sandbox/tests/e2e.rs
  - crates/pixi-sandbox/tests/cli.rs
  - crates/pixi-sandbox-core/tests/shard.rs
  - Cargo.toml
documentation:
  - CONTEXT.md
priority: medium
type: chore
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The suite is 226 tests across ~4,300 lines in 11 files, and its scaffolding is copy-paste. `copy_tree` is defined four times (`cli.rs`, `e2e.rs`, `user_tools.rs`, `restore_script.rs`), `fixture_transport` five times, `bin()` three, `make_executable` two, `host_platform` two, and the git helpers (`run_git`/`git`/`commit`) three. Every new test file re-derives the same scaffolding, which is exactly how `restore_script.rs` arrived with a fourth `copy_tree`: the cost of a shared helper is currently higher than the cost of another copy, so the suite grows sideways.

Two libraries fix the two halves of that.

**rstest gives pytest-style fixtures.** A `#[fixture]` is a function other fixtures and tests take as a parameter, so `isolated_home`, `fixture_transport`, `transport_branch(fixture_transport, isolated_home)` and `restored_project(transport_branch)` compose once and a test asks for the one it needs. Setup that is currently repeated per test — tempdir, `HOME`/`TMPDIR` isolation (D10 at user level), a git repo carrying the transport, the `#!` shim standing in for the bundled binary — becomes declarative, and a test body shrinks to the thing it actually asserts. `#[case]` parametrisation covers the other half: the suite has 22 hand-rolled `for … in [...]` tables (ref-unsafe branch names, platform mappings, forbidden fixture words, config error shapes), and a loop that fails names the loop, not the case. One named test per case means the failure line tells you which input broke.

**proptest is already a dependency and drives exactly one test** (`split_then_join_is_identity`). The invariants that are currently sampled by example are the ones worth generating: branch-name ref safety, manifest validate/round-trip, config → publish plan, `files.json` canonicalisation, and the relocation rule that only valid UTF-8 without NUL is rewritten. These are Hypothesis-shaped properties — "for any input, this holds" — and the repository already pays for the dependency.

One constraint comes first: **rstest is not in the vendored tree**, so this cannot start inside the airlock. The dev-dependency has to be added, locked and vendored on the connected side, and a transport repacked, before a restored host can compile the suite. See the lockfile-bot design in `CONTEXT.md` § Session scratchpad.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 rstest is a workspace dev-dependency that reached the airlock the supported way: Cargo.lock and the vendored tree refreshed on the connected side, a transport packed and published with it, and 'cargo check --offline' plus 'pixi run test' green on a restored host with no network
- [ ] #2 One shared test-support module owns every helper that is currently duplicated (copy_tree, fixture_transport, demo_project, bin/isolated_bin, make_executable, host_platform, the git helpers, transport_repo), each defined exactly once and exposed as an rstest #[fixture]; no test file redefines one
- [ ] #3 A check fails the build when a helper name is defined in more than one test file, so the duplication cannot grow back (the way a fourth copy_tree arrived unnoticed)
- [ ] #4 Fixtures isolate by construction: every fixture that runs a restore yields a tempdir HOME and TMPDIR, and D10 still holds - tests target tests/fixtures, never this repository, with scripts/restore.sh as the only reviewed exception
- [ ] #5 Every hand-rolled table loop in the suite (22 today) becomes #[case] parametrisation: one named test per case, each reporting its own input on failure, with no assertion lost
- [ ] #6 proptest covers at least these invariants, beyond the existing split/join identity: branch-name ref safety agreeing with scripts/restore.sh, manifest validate/round-trip, sandbox config to publish plan, files.json canonicalisation, and the relocation rule that only valid UTF-8 without NUL is rewritten
- [ ] #7 Property failures are reproducible: every proptest has its proptest-regressions file committed and not ignored, case counts are bounded so the whole suite stays under 20 seconds, and no test depends on execution order or wall-clock
- [ ] #8 pixi run lint and pixi run test are green, the test count has risen rather than fallen, and the baseline in .agents/skills/session/SKILL.md plus the testing sections of AGENTS.md and README.md state the new conventions
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Connected side first: add `rstest` as a workspace dev-dependency, refresh `Cargo.lock` and the vendored tree, and pack/publish a transport that carries it — nothing below compiles in the airlock until that lands.
2. Create the shared support module (`crates/pixi-sandbox/tests/support/mod.rs`, or a `pixi-sandbox-testkit` dev-only crate if `pixi-sandbox-core`'s tests want it too) and move one helper at a time, deleting each duplicate as its single definition appears. One commit per helper keeps a regression bisectable.
3. Express the moved helpers as `#[fixture]`s and compose them: `isolated_home` → `transport_branch` → `restored_project`. Convert the heaviest files first (`restore_script.rs`, `user_tools.rs`, `e2e.rs`), since they carry the most setup per test.
4. Convert every `for … in [...]` table to `#[case]`s, keeping the assertion text so a failure still explains itself.
5. Add the property tests, each with a bounded case count and its `proptest-regressions/` file committed; check the suite's wall-clock after each one.
6. Add the guard that keeps it true: a check (in `tests/fixtures.rs` or `xtask check-repository`) that fails when a helper name is defined in more than one test file.
7. Update the baselines and conventions: test count in `.agents/skills/session/SKILL.md`, the testing section in `AGENTS.md` and `README.md`, and a short note in the docs on how to write a new test (ask for a fixture; parametrise with `#[case]`; prefer a property where the invariant is universal).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Declared rstest 0.26.1 across the workspace and used the connected relock lane to refresh Cargo.lock. This dependency-only landing must publish a transport containing the new vendor crates before the offline fixture refactor can compile; implementation continues after a fresh restore.

2026-10-02 dependency proof: PR #56 merged as ffd2016; post-merge ci, docs, and publish sandbox runs were green. After force-fetching the rotated sandbox branch, a fresh restore reported 169 vendored crates (up from 155) and Cargo.lock digest 7fbc917aafa0. `pixi run --frozen -- cargo check --workspace --offline` and `pixi run --frozen test` both passed; suite baseline remains 285 passed / 1 skipped.

2026-10-02 support slice: moved the duplicated `bin`, fixture paths, copy, user-isolation, executable, host-platform, temporary-git, and transport-repository setup into `tests/support/mod.rs`. The module exports rstest fixtures (including parameterised helper fixtures); the former test files import it instead of redefining setup. `fixtures.rs` now asserts every shared helper has exactly one definition and a fixture entrypoint. The package test suite passes: 139 passed / 1 skipped.

2026-10-02 property/guard slice: added bounded, seed-file-backed properties for Bash/Rust branch safety, manifest JSON validation/round-trip, publish-target uniqueness/ref safety, files-manifest prefix canonicalisation, and NUL-free UTF-8 relocation. The branch predicate is now shared semantically between `sandbox_config` and `scripts/restore.sh`; a generated test checks their agreement. Added the production-module coverage guard and documented fixture/property conventions in the support module, AGENTS.md, and README.md. The frozen full suite is 322 passed / 1 skipped; frozen llvm-cov reports 80.25% region coverage and 83.72% line coverage.

2026-10-02 rstest slice: converted the platform→tool pin table, runner mapping and invalid override table, unsafe branch names, Git mock/shell table, and CLI top-level verb table into independently reported rstest cases. The remaining fixture assertion-table loops still need the same treatment before the task can close.
<!-- SECTION:NOTES:END -->
