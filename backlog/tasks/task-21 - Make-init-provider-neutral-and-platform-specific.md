---
id: TASK-21
title: Make init provider-neutral and platform-specific
status: To Do
assignee: []
created_date: '2026-10-01 09:49'
labels:
  - cli
  - init
  - platform
milestone: m-0
dependencies: []
references:
  - .knowledge/v1-evolution-plan.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace init github with pure init. Add --github-workflow-path, --script-path, and --config overrides; prefer pixi-sandbox.toml with dotfile fallback; generate only the current platform launcher; make generated files safely regenerable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pixi-sandbox init works without a provider subcommand
- [ ] #2 Unix generates only the selected POSIX launcher and Windows only the PowerShell launcher
- [ ] #3 Both config filenames and explicit --config work
- [ ] #4 CLI tests cover force and path overrides
<!-- AC:END -->
