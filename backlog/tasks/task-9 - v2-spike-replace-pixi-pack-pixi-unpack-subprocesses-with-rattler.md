---
id: TASK-9
title: 'v2 spike: replace pixi-pack/pixi-unpack subprocesses with rattler'
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
labels:
  - core
  - spike
dependencies: []
references:
  - .knowledge/design.md
  - .knowledge/decisions.md
priority: low
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Design §1 notes that re-implementing the pack format on the rattler library is possible but explicitly not v1 work; it would remove the pixi-pack and pixi-unpack subprocesses and their embedded binaries from the transport. Track as a measured spike: produce a decision-level writeup before any code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A spike note in .knowledge/ measures the trade-off: binary size and blob count saved versus implementation risk and pixi-pack format coupling
- [ ] #2 A go/no-go decision is recorded as a new decision ID in .knowledge/decisions.md before implementation starts
<!-- AC:END -->
