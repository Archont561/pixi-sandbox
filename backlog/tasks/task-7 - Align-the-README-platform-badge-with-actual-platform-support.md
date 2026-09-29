---
id: TASK-7
title: Align the README platform badge with actual platform support
status: Done
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 07:26'
labels:
  - docs
dependencies: []
references:
  - README.md
  - pixi.toml
priority: low
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The README badge advertises linux-64, osx-arm64 and win-64, but win-64 was removed from the pixi.toml workspace platforms because bun has no win-64 conda-forge build, and decision D11 lists Windows as not yet proven. Either drop win-64 from the badge or document the Windows gap explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Platforms badge matches the platforms pixi.toml declares and the bundles .pixi-sandbox.toml publishes
- [x] #2 Windows is either absent from the badge or explicitly marked unproven with the bun blocker noted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
TDD at the lint seam: scripts/lint-repo-consistency.sh parses the badge, pixi.toml [workspace] platforms and .pixi-sandbox.toml bundles, fails on disagreement, then the README is fixed until it passes.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Green: the badge is linux--64 | osx--arm64 and a Platform Support table states linux-64 proven and published, osx-arm64 declared but airlock-unproven (D11, task-1), linux-aarch64/osx-64 release binaries only, win-64 unsupported with the conda-forge bun blocker and D11 on one line.

Proved both ways: re-adding win-64 to the badge fails the check, removing it passes. Wired into pixi run lint, ci.yml and a lefthook pre-commit job.

Red first: the checker reported the badge advertising three platforms against the two pixi.toml declares, the win-64 claim, and the missing Windows-gap line.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Platforms badge matches pixi.toml exactly, every platform .pixi-sandbox.toml publishes is one of them, and Windows is documented as unsupported with the bun blocker and D11 rather than advertised. pixi run lint-repo-consistency keeps the three files in agreement.
<!-- SECTION:FINAL_SUMMARY:END -->
