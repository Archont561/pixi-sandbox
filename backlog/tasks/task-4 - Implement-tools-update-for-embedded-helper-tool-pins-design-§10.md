---
id: TASK-4
title: Implement tools update for embedded helper-tool pins (design §10)
status: In Progress
assignee:
  - '@me'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 10:23'
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
