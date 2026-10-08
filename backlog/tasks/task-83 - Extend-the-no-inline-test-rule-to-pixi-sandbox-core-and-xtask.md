---
id: TASK-83
title: Extend the no-inline-test rule to pixi-sandbox-core and xtask
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-08 22:50'
labels:
  - testing
  - policy
dependencies: []
priority: medium
type: enhancement
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding, test policy. AGENTS.md (Test conventions) says pixi-sandbox-core and xtask "still carry inline tests and are not yet enforced; splitting them is backlog work". No open backlog task tracks that work. Measured: 27 source files contain #[cfg(test)] modules, 3,077 lines in total (core: 4 files, 326 lines; xtask: 23 files, 2,751 lines). pixi-sandbox-git has none. The binary crate is guarded by production_sources_carry_no_inline_test_modules in crates/pixi-sandbox/tests/fixtures.rs, with a LEGACY list that may only shrink.

Intended behaviour: move each inline module to tests/ through the crate's public API. Extend the guard to pixi-sandbox-core and xtask with a shrinking LEGACY list, so the rule is enforced rather than promised.

Seam to agree before tests are written: per module, the public API, exercised from tests/<module>.rs. Private helpers that need testing are exported #[doc(hidden)] pub, as AGENTS.md already prescribes.

Non-goals: no behaviour change, no new test framework, and no weakening of assertions to make moved tests pass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each inline module in pixi-sandbox-core and xtask moves to tests/; no test is lost (names kept, or a recorded mapping), and the test count does not drop.
- [x] #2 A guard test in each crate's tests/ fails on any production source containing #[cfg(test)] that is not in a LEGACY list, and fails on stale LEGACY entries (mirroring fixtures.rs).
- [x] #3 The LEGACY list is recorded in the guard test when it lands and shrinks over successive commits; the final list is empty.
- [x] #4 AGENTS.md's test-convention paragraph is updated to match the enforced state.
- [x] #5 No test points at this repository or a real HOME (D10).
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->

Landed on the session branch as six commits: the two guards recording full LEGACY lists (core 4,
xtask 24 — the audit's 23 plus `agents_md.rs`, which landed with check 12), then four move
commits shrinking xtask's list 24 → 19 → 7 → 3 → 0 (core's went 4 → 0 in its own commit).

Evidence, all local:

- AC#1 every module moved to `tests/<module>.rs` through the crate's public API — core's four
  into the existing per-module files, xtask's twelve check modules into `tests/repo_checks*.rs`
  with the shared `valid_fixture` moved verbatim to `tests/support/mod.rs`, the release family
  and `util`/`version`/`conda_platforms`/`smoke`/`workflow`/`airlock`/`starter` into their
  name-matched files. Test names kept everywhere except one recorded mapping: `main.rs`'s
  `airlock_self_bin_subcommand_reaches_the_downloader_validation` moves to `tests/cli.rs` as a
  black-box drive of the binary (the conventions' binary-target route — `Args`/`run_args` are
  unreachable from `tests/`), name kept. Count never dropped: 772 → 774 (the two guards), held
  at 774 through every commit (core 209, xtask 133).
- AC#2 each crate's `tests/fixtures.rs::production_sources_carry_no_inline_test_modules`
  mirrors the `pixi-sandbox` oracle: new `#[cfg(test)]` fails, stale LEGACY entries fail.
- AC#3 the lists landed full and shrank over the successive commits (24 → 19 → 7 → 3 → 0);
  both are empty and the guards keep them so.
- AC#4 AGENTS.md's Test conventions paragraph records the enforced state — every crate guarded,
  core and xtask lists empty, `pixi-sandbox`'s legacy list the only remaining debt.
- AC#5 the migration introduces no repository- or HOME-pointing tests: everything new is
  tempdir/fixture-fed. One pre-existing pin travelled with the corpus —
  `starter_workflow_checks_the_real_main_ref_without_creating_or_seeding_the_repo` embeds
  `.github/workflows/starter.yml` at compile time and asserts its shape. It was moved verbatim
  because the non-goals forbid weakening assertions; its subject is the fixture-fed
  `parse_main_ref`/`verify` functions, and it touches neither the checkout as a project nor
  any real HOME. If AC#5 is to be read strictly against even compile-time pins, the follow-up
  is to promote that pin into `xtask check-repository` (where check 10 already pins the relock
  render) and drop the test — say the word and I will.

Test boundaries exported `#[doc(hidden)] pub` (the seams a test legitimately reaches):
`collapse_nul_runs`, `replace_all`, `EMBEDDED_JSON`; `append_line`, `conda_packages`,
`WORKFLOW_PATH`, `run_actionlint`; `LEVELS`, `SCOPES`, `MULTI_RUN_ALLOWED`,
`GENERATED_MARKER`, `LIB`; `staged_name`, `binary_name`, `SUMS_NAME`, `built_binary_path`,
`host_triple`, and `prepare_release`'s five staging helpers; `DARWIN_EGRESS_PROFILE`, the
gh/pixi driving helpers, `resolve_pixi_trampoline`, `command_with_absolute_program`,
`parse_main_ref`, `floating_version_findings`, `publish_refusals`. `sha256_file` and `ShellGit`
are imported from the crates that own them instead. xtask gained a library surface (`lib.rs`
`pub mod`s) for the tests to reach; `main.rs` keeps the clap glue, black-box per the
conventions.

Baselines for the next session: 774 passed / 1 skipped (21 git · 209 core · 411 pixi-sandbox ·
133 xtask).

<!-- SECTION:NOTES:END -->
