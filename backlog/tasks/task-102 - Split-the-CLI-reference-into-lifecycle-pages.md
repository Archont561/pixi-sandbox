---
id: TASK-102
title: Split the CLI reference into lifecycle pages
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - reference
  - cli
dependencies:
  - TASK-96
references:
  - docs/src/content/docs/reference/cli.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 102000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The single 23k-character CLI reference covers global usage, diagnostics, pack, doctor, publish, restore, unpack, init, plan, tools, and self-update — efficient for maintainers, hard to scan, link, and review. Split it along lifecycle boundaries into five pages: (1) CLI overview and shared diagnostics; (2) Create and verify a transport (init, plan, pack, doctor, sizes/budgets); (3) Publish and rotate transport branches (publish, branch naming, retention, CI workflow behavior); (4) Restore and inspect a transport (restore, unpack, offline checks, operator failure recovery); (5) Tools and self-update (embedded tools, lock/pins, self-update). Each page opens with "Use this when…", the minimum command, detail tables, safety implications, failure modes, and related guides. Do not copy Pixi one-page-per-command approach.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Five reference pages exist per the audit split, each starting with "Use this when…" and ending with failure modes and related guides.
- [ ] #2 No command, flag, or diagnostic from the current page is lost; deep-linkable headings survive the split.
- [ ] #3 The pages live under the Reference group of the new IA and are reachable from the lifecycle guides.
<!-- AC:END -->
