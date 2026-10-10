---
id: TASK-77
title: Raise the Rust lint gate to pedantic and clear the audited source debt
status: In Progress
assignee: []
created_date: '2026-10-06 20:24'
labels:
  - refactor
  - rust
  - tooling
  - cleanup
  - testing
dependencies: []
references:
  - Cargo.toml
  - crates/pixi-sandbox-core/src/platform.rs
  - crates/pixi-sandbox/src/commands/doctor.rs
  - crates/pixi-sandbox/src/commands/restore.rs
  - crates/pixi-sandbox/src/commands/tools/update.rs
  - crates/pixi-sandbox-core/src/transport_budget.rs
documentation:
  - >-
    backlog/docs/plans/source-pedantic-lint-audit/doc-12 -
    Source-Audit-Pedantic-Lint-Debt-and-the-Long-Function-Remainder.md
  - >-
    backlog/docs/plans/repo-wide-refactor-dry-kiss-solid/doc-9 -
    Repo-wide-Refactor-Plan-DRY-KISS-SOLID-and-Fixture-Property-Based-Tests.md
priority: medium
type: chore
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
doc-9 promised a one-off clippy::pedantic pass as a cheap follow-up in a Workstream 4 that was never written, so it was never run. doc-12 runs it and records the result: 469 unique findings (435 under src/) from `cargo clippy --workspace --all-targets -- -W clippy::pedantic -W clippy::nursery`, none of which the current gate can see, because there is no `[workspace.lints]` table and no per-crate `[lints]` table and the four crate lint scripts run clippy at the default level behind `-D warnings`. The tree grew 3425 source lines in the three days after doc-9 measured it, all of it under that blind gate.

The findings are unusually tractable. 261 of the 469 are MachineApplicable, so `cargo clippy --fix` writes them and review is reading a diff. Cognitive complexity is zero workspace-wide while too_many_lines is 13, which says the long functions here are long sequences rather than tangles, and Extract Method is therefore low-risk. Duplicate 8-line blocks number 18 across 20159 lines, so doc-9 DRY work held. The debt has shifted into the two library crates: pixi-sandbox-core and pixi-sandbox-git run about one finding per 19 lines, while xtask, which doc-9 called its biggest gap, is now the cleanest crate by a factor of eight.

Five of the thirteen long functions are the part of doc-9 A3 that was never executed. Task-57 did only pack::run, and pack::run is the one entry that has dropped off the list, so clippy independently confirms the five that were left behind. The first and second largest functions in the workspace are the two workflow renderers and they belong to task-76, not here.

Encode the policy first so the gate holds the line for free, then sweep, then finish the structural remainder. See doc-12 for the per-lint, per-crate and per-file tables and for the false positives that must not be churned.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A reviewed clippy policy lives in a `[workspace.lints.clippy]` table that all four crates inherit, every deliberate allow carries a reason comment, and the choice is recorded in .knowledge/decisions.md as a new D-number rather than left implicit.
- [x] #2 The policy allows clippy::literal_string_with_formatting_args with a reason, because all 10 hits are intentional: placeholder templates such as `.replace("{version}", ...)` and shell parameter expansions such as `${PREFIX:-sandbox}` asserted inside Rust string literals, per doc-12 Finding 6.
- [ ] #3 The 261 MachineApplicable findings are applied in reviewed per-crate commits with no behaviour change, and pixi run --frozen lint passes on all four crates at the raised bar.
- [ ] #4 Documentation debt the policy demands is paid rather than silently downgraded: 58 missing_errors_doc, 28 too_long_first_doc_paragraph, 12 missing_panics_doc and 16 doc_markdown are each either written or allowed with a recorded reason.
- [x] #5 The five long functions doc-9 A3 listed and never executed are extracted, one function per commit: doctor::print_human at 179 lines, restore::register_user_tools at 121, doctor::run at 111, restore::run at 110, and tools::update::refresh at 101.
- [ ] #6 xtask main::run_args at 101 lines and the 137-line narrative test in tests/e2e.rs stay as they are, allowed in place with a reason, because splitting a subcommand dispatcher or a single offline end-to-end proof makes both worse.
- [x] #7 The two workflow renderers are left to task-76: this task does not restructure render_github_workflow at 476 lines or render_relock_workflow at 347, and where the raised gate would fail on them it adds a function-scoped allow naming task-76 as the owner instead of reshaping the templates.
- [x] #8 The three cast sites in transport-size arithmetic are each read and resolved: shard.rs:122, transport_budget.rs:232 and pack.rs:523 are either corrected with a test pinning the bound, or annotated with why the cast cannot truncate, since a silent truncation there yields a wrong budget verdict rather than a crash.
- [ ] #9 The 413-line inline test module in commands/tools/update.rs, 54 percent of that file against a 22 percent workspace rate, moves to a tests/ sibling matching the crate self_update_*.rs convention, with no test lost.
- [ ] #10 pixi run --frozen test reports at least 654 passing before and after every commit in this task, no test added or changed points at this checkout or a real HOME per D10, no new crate dependency is added because the vendored tree cannot fetch one without a relock cycle, and no rendered workflow, manifest schema, transport format or git-access path changes.
<!-- AC:END -->

<!-- SECTION:NOTES:BEGIN -->
## Implementation Notes

2026-10-10, on `arena/00dbb795-pixi-sandbox` (base 0a036ba2). Environment note: the
turn-end patchset rollback wiped the local commit chain mid-session; the history was
rebuilt from the surviving tree with the same per-crate/per-function slicing (the
git crate's sweep fixes are folded into the TASK-76 commit — its only src files were
touched by both tasks). Suite: 924/1 at start → 928/1 now (35 git / 212 core /
525 pixi-sandbox / 156 xtask).

Pedantic-clean with per-crate `clippy -D warnings` green: pixi-sandbox-core (22
`# Errors` sections, the audited judgement items, the AC#8 casts resolved with
pinning tests, two extractions), pixi-sandbox-git (24 `# Errors`, 10 `# Panics`,
`#[must_use]` builders, format_collect, two test judgement items) and pixi-sandbox
(42 doc sections, all mechanical findings, the pack casts with a pinning test, eight
extractions one per commit, four scoped allows with reasons). Policy D20 plus the
`literal_string_with_formatting_args` allow with its reason. AC#1, #2, #5, #7 and
#8 are met.

Remaining (next session): xtask's 48 findings (38 missing_errors_doc, 4
format_collect, unnecessary_wraps stale_refs.rs:20, missing_panics_doc util.rs:32,
similar_names tests/support/mod.rs:88, case_sensitive tests/release_assets.rs:69,
too_many_lines on workflow.rs:68 lint_generated_workflow → extract and main.rs:302
run_args → allow in place with a reason per AC#6); the AC#9 migration of the
413-line inline test module in commands/tools/update.rs to tests/tools_update.rs
(logic promoted through lib.rs; only this one of the LEGACY inline-test list
migrates); final gates (fmt, full lint at zero warnings on all four crates, full
tests, check-repository, lint-generated-workflow, convco over the range);
task-file closure. The lint gate is red on xtask until that lands; the checkpoint
PR was merged with this state documented.
<!-- SECTION:NOTES:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
