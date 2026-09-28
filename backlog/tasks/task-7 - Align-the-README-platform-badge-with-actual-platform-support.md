---
id: TASK-7
title: Align the README platform badge with actual platform support
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
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
- [ ] #1 The Platforms badge matches the platforms pixi.toml declares and the bundles .pixi-sandbox.toml publishes
- [ ] #2 Windows is either absent from the badge or explicitly marked unproven with the bun blocker noted
<!-- AC:END -->
