---
id: TASK-6
title: Fix the stale not_yet error message referencing the deleted Python prototype
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
labels:
  - cli
  - cleanup
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/mod.rs
priority: medium
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The not_yet helper in commands/mod.rs tells the user that the Python prototype in .knowledge/research/ implements the verb end-to-end and is the reference for this port. That prototype was deleted in 0.2.0 (see the CHANGELOG entry: remove all Python script references) and .knowledge/research/ now holds only EVIDENCE.md and REPRODUCE-TRANSCRIPT.md. The message misleads an airlock operator who has no network to check.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The not_yet message points only at .knowledge/design.md sections and no longer mentions any Python prototype
- [ ] #2 A grep across crates/ finds no Python-prototype references left
<!-- AC:END -->
