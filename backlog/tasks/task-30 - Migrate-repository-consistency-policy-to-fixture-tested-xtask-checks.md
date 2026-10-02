---
id: TASK-30
title: Migrate repository consistency policy to fixture-tested xtask checks
status: Done
assignee:
  - '@agent'
created_date: '2026-10-01 14:10'
updated_date: '2026-10-02 18:10'
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
Implemented as crates/xtask/src/repo_checks.rs: eight composable checks, every one taking an explicit root and returning structured Failure{headline,details,hint} diagnostics collected in a single pass. D10 holds: only main.rs passes the real root at runtime; the test suite builds a minimal valid fixture repository in a tempdir and proves it passes, then breaks one policy per test (stale reference + opt-out, badge/workspace disagreement, undeclared published platform, conda-manifest version mismatch, stale documented release reference, mutable action ref and unlabelled SHA, missing template/committed installer, workflow literal tag, drifted/relative generated action, Bash-4 construct in a surviving script). Platform claims are parsed with the toml crate (all bundles' platforms, not the first match of an order-sensitive pipeline). `xtask check-repository` reports all failures in one run and `pixi run lint-repo-consistency` uses it; scripts/lint-repo-consistency.sh is removed after parity was demonstrated against the live tree. AC#4 is now proven by TASK-29's landed removal of the public composite Actions, reusable publisher, generated Action shims, and repository-consistency check 7. The current fixture suite retains the third-party SHA, stale-reference opt-out, generated-workflow, and release-version checks without carrying the retired composite-Action drift policy. Check 8 (introduced by the v0.3.6 release failure, run 36865921206) still holds the surviving shell bootstrap — restore.sh, which must run where no toolchain can be assumed — to the Bash 3.2 surface of the macOS runners.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Repository consistency policy is fixture-tested Rust with structured diagnostics behind `xtask check-repository`; the shell lint is gone. AC#4 is complete: TASK-29 retired the composite-Action surfaces and the obsolete drift policy, while the remaining action-pin, stale-reference, generated-workflow, and release-version checks retain fixture-backed coverage.
<!-- SECTION:SUMMARY:END -->
