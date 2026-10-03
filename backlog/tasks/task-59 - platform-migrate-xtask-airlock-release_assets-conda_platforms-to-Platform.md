---
id: TASK-59
title: 'platform: migrate xtask airlock, release_assets, conda_platforms to Platform'
status: To Do
assignee: []
created_date: '2026-10-03 09:26'
updated_date: '2026-10-03 09:27'
labels:
  - refactor
  - dry
dependencies:
  - TASK-55
priority: high
ordinal: 60000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace xtask/src/airlock.rs static_asset_for_platform, release_assets.rs target-triple derivation, and conda_platforms.rs SUPPORTED_PLATFORMS with calls into task-55 Platform type. xtask already depends on pixi-sandbox-core. See doc-9 section A1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 xtask airlock.rs static_asset_for_platform delegates to Platform instead of its own match
- [ ] #2 xtask release_assets.rs target-triple derivation and conda_platforms.rs SUPPORTED_PLATFORMS source from Platform::ALL
- [ ] #3 pixi run --frozen test passes with count >= 516/1
- [ ] #4 pixi run --frozen lint stays clean
<!-- AC:END -->
