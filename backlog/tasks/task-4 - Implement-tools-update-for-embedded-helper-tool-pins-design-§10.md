---
id: TASK-4
title: Implement tools update for embedded helper-tool pins (design §10)
status: Done
assignee:
  - '@me'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 14:21'
labels:
  - cli
  - tools
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/tools.rs
  - crates/pixi-sandbox-core/assets/tools.lock.json
priority: medium
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
tools.rs returns not_yet for tools update. The embedded catalogue in crates/pixi-sandbox-core/assets/tools.lock.json can currently only be bumped by hand-editing reviewed data. The verb should resolve the latest versions of pixi, pixi-pack, pixi-unpack and rattler-index, download and sha256-verify them, and write an updated lock with a reviewable diff.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pixi-sandbox tools update writes an updated tools lock fetched from the same source the embedded pins were compiled from
- [x] #2 Downloaded artifacts are sha256-verified before the lock is written and a mismatch aborts without touching the lock
- [x] #3 tests/manifest.rs the_embedded_tool_pins_are_valid_and_complete stays green after an update
- [x] #4 The --tools-lock PATH override is honoured for out-of-tree locks
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Read tools.rs not_yet and tools_lock.rs, the embedded catalogue format, and the existing --tools-lock override. Implement tools update: resolve latest, download, sha256-verify, write an updated lock with a reviewable diff. Cover AC#2 mismatch-aborts, AC#3 embedded pins stay valid, AC#4 out-of-tree override.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Verified as already implemented and closed the record. tools update lives in crates/pixi-sandbox/src/commands/tools/update.rs (899 lines) and is wired through ToolsCommand::Update in tools.rs; no not_yet call site remains anywhere in crates/*/src.

CLI surface: --tools-lock PATH updates in place (AC#4), no flag prints the candidate catalogue for review as a diff (AC#1), --check reports whether anything is newer without downloading or writing, --tool NAME limits the run.

Evidence, all offline against the restored sandbox toolchain: 14 commands::tools::update unit tests green, including a_new_release_rewrites_the_version_and_every_hash, a_disagreement_with_the_upstream_manifest_aborts and a_failed_write_leaves_the_previous_lock_byte_for_byte (AC#2), plus pixi-sandbox-core tests/manifest.rs the_embedded_tool_pins_are_valid_and_complete (AC#3) — 9 passed, 0 failed.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
tools update was already implemented end to end (crates/pixi-sandbox/src/commands/tools/update.rs, wired via ToolsCommand::Update); this closes the record. Resolves latest releases from the same GitHub source the embedded pins were compiled from, sha256-verifies every downloaded asset against the upstream checksum manifest before writing, and either prints a reviewable candidate catalogue or updates --tools-lock PATH in place. All four acceptance criteria verified offline: 14 update unit tests and the embedded-pin manifest test are green.
<!-- SECTION:FINAL_SUMMARY:END -->
