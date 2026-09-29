---
id: TASK-8
title: Fail pack --cargo-vendor early on duplicate crate sources (design §11)
status: Done
assignee:
  - '@me'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 10:06'
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
- [x] #1 pack --cargo-vendor detects the duplicate-source condition and fails with an explanation of the remedy before anything is published
- [x] #2 A fixture-based test reproduces the condition (crates.io plus a git dependency on the same crate) and asserts the pack-time error
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Read design.md 11 for the exact upstream failure, read pack.rs vendor path, reproduce the duplicate-source condition in tests/fixtures, fail at pack time with a remedy, add a fixture-based test.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
pack --cargo-vendor now reads Cargo.lock before creating anything and refuses a crate+version reachable from two sources, naming the crate, both sources and the two remedies (make the versions differ, or drop one of the dependencies). Path/workspace members have no source and are correctly not treated as a collision.

Placed in run() rather than in the vendor step: a test caught that the later position left a half-built transport behind, so the retry failed on the stale-transport guard instead of showing the real error. The lockfile read needs no network, so the refusal is immediate.

Fixture tests/fixtures/duplicate-source-project carries a lockfile generated from a real crates.io itoa 1.0.15 plus a file:// git repo with the same version (git URL frozen, nothing fetches it). Both directions tested: duplicate rejected with a remedy, and demo-project (multiple crates, repeated versions, a path member) accepted. 100 tests pass, lint clean.
<!-- SECTION:FINAL_SUMMARY:END -->
