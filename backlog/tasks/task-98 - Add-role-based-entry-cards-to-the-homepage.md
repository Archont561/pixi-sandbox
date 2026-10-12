---
id: TASK-98
title: Add role-based entry cards to the homepage
status: To Do
assignee: []
created_date: '2026-10-12 07:55'
labels:
  - docs
  - homepage
  - roles
dependencies:
  - TASK-96
references:
  - docs/src/content/docs/index.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Immediately after the hero, add four entry cards so each audience can pick a role and reach a tailored first page in one click: project owner → first offline bundle; CI maintainer → CI publishing; airlock operator → restore on the airlock; security/platform reviewer → security and host boundaries. Keep the transport diagram as the lead visual and use text plus icons, never color alone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Four role cards sit directly under the hero, each stating the primary outcome and linking its first page.
- [ ] #2 Card destinations resolve inside the new IA from task-96.
- [ ] #3 The homepage still leads with the Pixi project → orphan branch → airlock diagram and the zero-network language.
- [ ] #4 Role selection is keyboard reachable and is not signaled by color alone.
<!-- AC:END -->
