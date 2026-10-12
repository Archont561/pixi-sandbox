---
id: TASK-101
title: Add machine and network context chips to command blocks
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - ux
  - safety
dependencies: []
references:
  - docs/src/components/
  - docs/src/content/docs/restore.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Running a connected-host command on an airlock is a high-cost mistake, and today the machine/network context is prose. Add compact context chips above command blocks across guides and reference: Connected host, Airlock, CI, Network required, Offline only, Read-only verification. Chips must be semantic — text plus icon, never color alone — so screen readers and print get the same signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A shared chip component renders the six audited states with accessible labels.
- [ ] #2 Command blocks in the guides and reference pages that run under a specific machine/network condition carry the matching chip.
- [ ] #3 The airlock restore page marks every command that must never touch the network.
<!-- AC:END -->
