---
id: TASK-8
title: Fail pack --cargo-vendor early on duplicate crate sources (design §11)
status: In Progress
assignee:
  - '@me'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 09:55'
labels:
  - pack
  - vendor
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/pack.rs
  - .knowledge/design.md
priority: low
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
cargo vendor hard-fails when the same crate and version is reachable from two sources such as crates.io and a git dependency (known upstream issue, design §11). Cargo gives no useful error when a crate is missing at restore time, so the failure must surface at pack time with a clear remedy instead of being discovered on the airlock.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pack --cargo-vendor detects the duplicate-source condition and fails with an explanation of the remedy before anything is published
- [ ] #2 A fixture-based test reproduces the condition (crates.io plus a git dependency on the same crate) and asserts the pack-time error
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Read design.md 11 for the exact upstream failure, read pack.rs vendor path, reproduce the duplicate-source condition in tests/fixtures, fail at pack time with a remedy, add a fixture-based test.
<!-- SECTION:PLAN:END -->
