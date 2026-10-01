---
id: TASK-26
title: Evaluate Turbo for multi-package JavaScript orchestration
status: To Do
assignee: []
created_date: '2026-10-01 09:49'
updated_date: '2026-10-01 09:49'
labels:
  - bun
  - turbo
  - tooling
milestone: m-0
dependencies:
  - TASK-25
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: low
type: spike
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Do not add Turbo merely for the current single docs workspace. Re-evaluate after multiple JS packages exist; if adopted, make Bun own dependencies, Turbo own the JS task graph, and Pixi provide the environment facade.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adoption threshold and task graph are documented
- [ ] #2 Turbo is a Bun dev dependency, not a Conda runtime dependency
- [ ] #3 Cache inputs/outputs and CI behavior are specified
<!-- AC:END -->
