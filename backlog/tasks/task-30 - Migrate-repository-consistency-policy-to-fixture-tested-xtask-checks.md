---
id: TASK-30
title: Migrate repository consistency policy to fixture-tested xtask checks
status: To Do
assignee: []
created_date: '2026-10-01 14:10'
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
- [ ] #1 Every policy checker accepts an explicit root or parsed input and returns structured diagnostics without assuming the current working directory
- [ ] #2 Fixture repositories prove each policy fails independently and a valid fixture passes; tests never target the actual pixi-sandbox checkout
- [ ] #3 Platform badge/workspace/publish-plan agreement and the documented Windows gap are parsed robustly rather than through order-sensitive shell pipelines
- [ ] #4 Third-party action SHA policy, stale prototype references with the documented opt-out, generated direct-workflow drift, and release-version rules retain equivalent or stronger coverage; retired composite-Action drift policy is not carried forward
- [ ] #5 `xtask check-repository <root>` reports all failures in one run, `pixi run lint-repo-consistency` uses it, and D10's no-repository-root test remains green
- [ ] #6 `scripts/lint-repo-consistency.sh` is removed after parity is demonstrated
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
After TASK-29 removes composite-Action policy, inventory the remaining numbered checks and give each one a pure function plus one valid and one invalid fixture. Reuse the generated-workflow and release modules rather than duplicating their predicates. Add an aggregation layer that collects every diagnostic, switch the Pixi lint task to it, compare failures against deliberately damaged temporary repositories, and remove the shell implementation last.
<!-- SECTION:PLAN:END -->
