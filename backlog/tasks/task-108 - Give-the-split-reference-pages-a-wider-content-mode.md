---
id: TASK-108
title: Give the split reference pages a wider content mode
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - reference
  - layout
  - ux
dependencies:
  - TASK-102
references:
  - docs/src/styles/custom.css
  - docs/astro.config.mjs
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: low
type: docs
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Long code/table pages are hard to orient in the three-column desktop layout. Once the references are split, give them a wider content mode so TOML examples, CLI flags, and wide tables stay readable at 1280–1440px without excessive wrapping, preserving the local ToC and leaving mobile untouched.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reference pages render with a wider content column at 1280–1440px; wide tables and TOML blocks no longer wrap excessively.
- [ ] #2 The right-side ToC and the mobile layout are unchanged.
- [ ] #3 The layout change is scoped to reference pages, not guides.
<!-- AC:END -->
