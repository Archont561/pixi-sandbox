---
id: TASK-106
title: Add a printable operator checklist to the airlock restore page
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - restore
  - checklist
  - airlock
dependencies: []
references:
  - docs/src/content/docs/restore.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The restore page is the strongest operator asset in the set. Add a compact, printable no-network checklist at the top — pre-flight verification, transfer, restore, and post-restore proof — followed by the detailed explanation. It must work printed and with JavaScript disabled, and link to the symptom entries from the troubleshooting task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A compact checklist sits at the top of the restore page covering verify → transfer → restore → prove offline.
- [ ] #2 The checklist is usable printed and with JS disabled.
- [ ] #3 Each step links to its detailed section and to the relevant troubleshooting entry.
<!-- AC:END -->
