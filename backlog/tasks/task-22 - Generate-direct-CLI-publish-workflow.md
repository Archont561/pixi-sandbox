---
id: TASK-22
title: Generate direct CLI publish workflow
status: Done
assignee:
  - '@agent'
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 17:00'
labels:
  - workflow
  - ci
milestone: m-0
dependencies:
  - TASK-21
  - TASK-27
references:
  - .knowledge/publish-automation.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Regenerate a workflow that installs pixi-sandbox from the @archont561/pixi-sandbox prefix.dev channel and invokes plan, pack, doctor, and publish directly, without Archont561 composite Actions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generated workflow has no Archont561 action references
- [x] #2 Workflow uses the configured config path
- [x] #3 Native bundle/platform matrix and doctor verification remain intact
- [x] #4 Generated workflow passes actionlint
<!-- AC:END -->
