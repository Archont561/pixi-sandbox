---
id: TASK-58
title: 'platform: migrate init.rs, standalone.rs, self_update assets to Platform'
status: Done
assignee: []
created_date: '2026-10-03 09:26'
updated_date: '2026-10-03 09:32'
labels:
  - refactor
  - dry
dependencies:
  - TASK-55
priority: high
ordinal: 59000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Delete the byte-identical (os,arch) to platform matches in commands/init.rs current_platform and standalone.rs platform, and self_update/assets.rs SUPPORTED_HOSTS, in favour of task-55 Platform type. Preserve existing error message text, since tests assert on it. See doc-9 section A1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 commands/init.rs current_platform and standalone.rs platform both delegate to Platform::from_os_arch instead of restating the match
- [x] #2 self_update/assets.rs SUPPORTED_HOSTS is derived from Platform::ALL instead of a hand-written table
- [x] #3 Existing error message text and test assertions on it are unchanged
- [x] #4 pixi run --frozen test passes with count >= 516/1
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
init.rs, standalone.rs and self_update/assets.rs now derive from Platform instead of hand-duplicating the (os,arch)->platform and platform->asset-name matches. Behavior and public signatures unchanged; all existing tests pass unmodified. No new dependency.
<!-- SECTION:FINAL_SUMMARY:END -->
