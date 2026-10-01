---
id: TASK-25
title: Move JavaScript tooling into Bun web environment
status: To Do
assignee: []
created_date: '2026-10-01 09:49'
labels:
  - bun
  - tooling
  - windows
milestone: m-0
dependencies: []
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: medium
type: chore
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move npm-compatible development tools such as Biome and Astro to package.json/bun.lock, keep Bun out of the default Windows-compatible Pixi environment, and document the Node.js plus prefix-local Bun fallback for Windows if supported.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Default Pixi environment has no Bun-only Conda dependency
- [ ] #2 Web tasks install from the committed Bun lockfile
- [ ] #3 Windows CI does not require Bun in the default environment
- [ ] #4 Bun fallback is tested or explicitly documented as unsupported
<!-- AC:END -->
