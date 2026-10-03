---
id: TASK-60
title: >-
  platform: migrate sandbox_config runner_for and the generated workflow
  template to Platform
status: Done
assignee: []
created_date: '2026-10-03 09:27'
updated_date: '2026-10-03 09:36'
labels:
  - refactor
  - dry
dependencies:
  - TASK-55
priority: medium
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace sandbox_config.rs runner_for hand-written match with Platform gh_runner, and have generated/github_workflow.rs render_github_workflow source its platform to asset-name mapping from Platform instead of re-declaring the five literals. Re-render .github/workflows/publish-sandbox.yml afterward so it does not drift from its template. See doc-9 section A1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sandbox_config.rs runner_for delegates to Platform::gh_runner
- [x] #2 pixi run --frozen test and pixi run --frozen lint pass
- [x] #3 A test proves every rendered asset name in github_workflow.rs template agrees with Platform::asset_name for each Platform::ALL member (the embedded shell case/switch stays literal text by design, since it runs on a plain GitHub runner before any pixi-sandbox binary exists to call into)
<!-- AC:END -->
