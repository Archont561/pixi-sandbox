---
id: TASK-59
title: 'platform: migrate xtask airlock, release_assets, conda_platforms to Platform'
status: Done
assignee: []
created_date: '2026-10-03 09:26'
updated_date: '2026-10-03 09:34'
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
- [x] #1 xtask airlock.rs static_asset_for_platform delegates to Platform instead of its own match
- [x] #2 pixi run --frozen test passes with count >= 516/1
- [x] #3 pixi run --frozen lint stays clean
- [x] #4 release_assets.rs staged_name/binary_name are proven to agree with Platform::asset_name for every Platform::ALL member via a cross-check test (kept generic over arbitrary triples by design, not migrated onto Platform itself)
- [x] #5 conda_platforms.rs SUPPORTED_PLATFORMS is derived from Platform::ALL instead of its own literal list
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
xtask airlock.rs, conda_platforms.rs now derive from Platform. release_assets.rs stays generic over arbitrary target triples by design, with a new cross-check test proving agreement with Platform for the 5 known platforms. pixi run --frozen test: 546 passed / 1 skipped.
<!-- SECTION:FINAL_SUMMARY:END -->
