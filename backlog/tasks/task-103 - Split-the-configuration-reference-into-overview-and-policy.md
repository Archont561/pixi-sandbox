---
id: TASK-103
title: Split the configuration reference into overview and policy
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - reference
  - configuration
dependencies:
  - TASK-96
references:
  - docs/src/content/docs/reference/configuration.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The 15k-character configuration reference mixes schema, budgets, runners, bundle shape, workflow policy, host requirements, validation, and examples. Split into "Configuration overview" (file selection, schema/version, minimal config) and "Bundle, CI, and host policy" ([[bundle]], budgets/runners/workflow, host requirements), using the same page template as the CLI split.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two pages exist per the audit split; minimal setup, bundle policy, CI policy, budgets, and host requirements are independently deep-linkable.
- [ ] #2 Every key and table from the current page survives the split with examples intact.
- [ ] #3 Both pages follow the reference template: "Use this when…", minimum config, tables, safety implications, failure modes, related guides.
<!-- AC:END -->
