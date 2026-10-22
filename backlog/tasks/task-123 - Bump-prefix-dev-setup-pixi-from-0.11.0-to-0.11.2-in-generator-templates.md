---
id: TASK-123
title: Bump prefix-dev/setup-pixi from 0.11.0 to 0.11.2 in generator templates
status: Done
assignee: []
created_date: '2026-10-22 07:37'
updated_date: '2026-10-22 07:49'
labels:
  - ci
  - deps
  - workflow
dependencies: []
priority: medium
type: chore
ordinal: 123000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Dependabot PR #177 bumps setup-pixi from 0.11.0 to 0.11.2 but directly edits committed workflows, which fails check-repository check 10 (generated file must equal render).

Correct fix: bump the pin in generator templates (crates/pixi-sandbox/src/generated/github_workflow.rs and relock_workflow.rs) and in all non-generated workflows (.github/workflows/*.yml), then re-render committed workflows via xtask render-relock and init rendering, update golden fixtures.

PR #177 failed ci lint because of this. The cargo and bun Dependabot PRs (#176, #178) passed and can be merged separately.

Reference: https://github.com/Archont561/pixi-sandbox/pull/177
Reference: https://github.com/prefix-dev/setup-pixi/releases/tag/v0.11.2
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Update setup-pixi pin from 9dabb60412d3d2d967a8d682f7ce01317af40514 v0.11.0 to v0.11.2 SHA in generator templates
- [ ] #2 Update same pin in all .github/workflows/*.yml non-generated files
- [ ] #3 Re-render relock.yml via xtask render-relock and verify byte-equality
- [ ] #4 Update golden fixtures for generated workflows
- [ ] #5 Verify lint, check-repository, test, and actionlint pass
<!-- AC:END -->
