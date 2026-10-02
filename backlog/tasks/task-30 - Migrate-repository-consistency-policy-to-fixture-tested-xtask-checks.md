---
id: TASK-30
title: Migrate repository consistency policy to fixture-tested xtask checks
status: Done
assignee:
  - '@agent'
created_date: '2026-10-01 14:10'
updated_date: '2026-10-02 17:30'
labels:
  - rust
  - tooling
  - lint
  - testing
milestone: m-0
dependencies:
  - TASK-27
  - TASK-29
  - TASK-31
references:
  - scripts/lint-repo-consistency.sh
  - .knowledge/decisions.md
  - README.md
  - .pixi-sandbox.toml
  - pixi.toml
priority: medium
type: enhancement
ordinal: 33000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace `lint-repo-consistency.sh` with composable Rust policy checks exercised against synthetic repository fixtures. Preserve D10: ordinary tests must never inspect this checkout; only the xtask command receives the actual repository root at runtime. Cover the platform story, stale references, action pin policy, generated-file drift, and version/reference policy through structured diagnostics rather than shell pipelines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every policy checker accepts an explicit root or parsed input and returns structured diagnostics without assuming the current working directory
- [x] #2 Fixture repositories prove each policy fails independently and a valid fixture passes; tests never target the actual pixi-sandbox checkout
- [x] #3 Platform badge/workspace/publish-plan agreement and the documented Windows gap are parsed robustly rather than through order-sensitive shell pipelines
- [x] #4 Third-party action SHA policy, stale prototype references with the documented opt-out, generated direct-workflow drift, and release-version rules retain equivalent or stronger coverage; retired composite-Action drift policy is not carried forward
- [x] #5 `xtask check-repository <root>` reports all failures in one run, `pixi run lint-repo-consistency` uses it, and D10's no-repository-root test remains green
- [x] #6 `scripts/lint-repo-consistency.sh` is removed after parity is demonstrated
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
After TASK-29 removes composite-Action policy, inventory the remaining numbered checks and give each one a pure function plus one valid and one invalid fixture. Reuse the generated-workflow and release modules rather than duplicating their predicates. Add an aggregation layer that collects every diagnostic, switch the Pixi lint task to it, compare failures against deliberately damaged temporary repositories, and remove the shell implementation last.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as crates/xtask/src/repo_checks.rs: eight composable checks, every one taking an explicit root and returning structured Failure{headline,details,hint} diagnostics collected in a single pass. D10 holds: only main.rs passes the real root at runtime; the test suite builds a minimal valid fixture repository in a tempdir and proves it passes, then breaks one policy per test (stale reference + opt-out, badge/workspace disagreement, undeclared published platform, conda-manifest version mismatch, stale documented release reference, mutable action ref and unlabelled SHA, missing template/committed installer, workflow literal tag, drifted/relative generated action, Bash-4 construct in a surviving script). Platform claims are parsed with the toml crate (all bundles' platforms, not the first match of an order-sensitive pipeline). `xtask check-repository` reports all failures in one run and `pixi run lint-repo-consistency` uses it; scripts/lint-repo-consistency.sh is removed after parity was demonstrated against the live tree. Deviation from AC#4's second clause, deliberately: TASK-29 has not retired the composite actions yet, so the check-7 drift policy (shared with the render-action-shims xtask) IS carried forward — dropping it now would un-guard published action paths; remove it with TASK-29. A new check 8 (introduced by the v0.3.6 release failure, run 36865921206) holds the surviving shell bootstrap — restore.sh, which must run where no toolchain can be assumed — to the Bash 3.2 surface of the macOS runners.

2026-10-02: AC#4 closed on the v0.4.2 tree. TASK-29 (landed the same day, after this task closed) removed the composite-Action surfaces and repo-consistency check 7 with them: `crates/xtask/src/repo_checks.rs` documents slot 7 as "retired with the composite Action surfaces in TASK-29", and no composite surface remains in the tree (no root `action.yml`, no `.github/actions/`, no `setup/`/`publish/` trees; `scripts/` holds `restore.sh` only) — the temporary carried-forward exception recorded above is gone, so there is no obsolete policy left to remove. The four families AC#4 names are fixture-tested and green (suite 403 passed / 1 skipped, baseline re-run 2026-10-02): action-SHA policy is check 4 (`a_mutable_tag_or_an_unlabelled_sha_fires_check_4`), stale prototype references with the opt-out are check 1 (`a_stale_reference_under_crates_fires_check_1_and_the_marker_silences_it`), generated direct-workflow drift is the golden-file freeze `generated_workflow_matches_the_reviewed_golden_file` plus the structural contract tests in `crates/pixi-sandbox/tests/generated_workflow.rs`, actionlint over the exact renderer (`xtask lint-generated-workflow`), check 5 over the generator source, and check 9's reviewed-exception shape rule over the committed render, and release-version rules are check 3 with `release_refs::scan` plus `xtask check-release-refs`. Checks 9 and 10 postdate this task, so workflow-shape and relock-render coverage is now stronger than what the retired shell lint checked. Equivalence is judged against the current fixture-tested inventory; the pre-migration shell script itself survives only in immutable history.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Repository consistency policy is fixture-tested Rust with structured diagnostics behind `xtask check-repository`; the shell lint is gone. The composite-Action drift check was intentionally retained until TASK-29 retired the actions themselves — TASK-29 has since removed those surfaces and check 7 with them, and AC#4 is closed with every named policy family fixture-tested on the v0.4.2 tree.
<!-- SECTION:SUMMARY:END -->
