---
id: TASK-55
title: 'platform: introduce a Platform type in pixi-sandbox-core'
status: Done
assignee:
  - '@agent'
created_date: '2026-10-03 09:25'
updated_date: '2026-10-03 09:29'
labels:
  - refactor
  - dry
dependencies: []
priority: high
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a single enum (Platform: Linux64/LinuxAarch64/Osx64/OsxArm64/Win64) in pixi-sandbox-core with from_os_arch, as_str, target_triple, asset_name, gh_runner, FromStr/Display, and an ALL const. No new deps (hand-written, no strum). Pure addition, no call sites changed yet. See backlog/docs/plans/repo-wide-refactor-dry-kiss-solid/doc-9 section A1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Platform type lives in pixi-sandbox-core with from_os_arch/as_str/target_triple/asset_name/gh_runner
- [x] #2 A round-trip property test covers every Platform::ALL member
- [x] #3 pixi run --frozen test passes with count >= 516/1
- [x] #4 No new Cargo dependency was added
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TDD: wrote crates/pixi-sandbox-core/tests/platform.rs first (confirmed red: unresolved import), then implemented crates/pixi-sandbox-core/src/platform.rs (Platform enum, from_os_arch, as_str, target_triple, asset_name, gh_runner, FromStr, Display, ALL) to go green. No call sites migrated yet (task-58/59/60). pixi run --frozen fmt, cargo clippy -p pixi-sandbox-core --all-targets -D warnings, and pixi run --frozen test all pass: 545 passed / 1 skipped (was 516/1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Added Platform enum (pixi-sandbox-core) as the single source of truth for platform id, target triple, release asset name, and GitHub runner label. TDD red-green: test file written first against a module that did not exist, then the minimal implementation. 27 new tests (rstest cases + a bounded proptest over Platform::ALL). No new Cargo dependency. Workspace suite: 545 passed / 1 skipped.
<!-- SECTION:FINAL_SUMMARY:END -->
