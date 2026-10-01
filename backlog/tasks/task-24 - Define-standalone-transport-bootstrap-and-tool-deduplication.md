---
id: TASK-24
title: Define standalone transport bootstrap and tool deduplication
status: To Do
assignee: []
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 09:49'
labels:
  - transport
  - restore
  - spike
milestone: m-0
dependencies:
  - TASK-21
references:
  - .knowledge/decisions.md
  - .knowledge/rattler-spike.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: spike
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Measure the proposal to make pixi-sandbox the standalone transport/restore orchestrator. Decide how pixi-pack and pixi-unpack are replaced or retained, version the transport schema, and ensure pixi-sandbox is not shipped twice as both bootstrap tool and environment dependency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A measured binary size and restore-disk comparison exists
- [ ] #2 A proposed schema migration or compatibility policy is recorded
- [ ] #3 Duplicate pixi-sandbox packaging behavior is specified
- [ ] #4 An accepted or rejected decision is recorded through the Backlog decision API
<!-- AC:END -->
