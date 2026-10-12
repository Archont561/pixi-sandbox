---
id: TASK-103
title: Split the configuration reference into overview and policy
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:01'
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
- [x] #1 Two pages exist per the audit split; minimal setup, bundle policy, CI policy, budgets, and host requirements are independently deep-linkable.
- [x] #2 Every key and table from the current page survives the split with examples intact.
- [x] #3 Both pages follow the reference template: "Use this when…", minimum config, tables, safety implications, failure modes, related guides.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: reference/configuration.mdx split into reference/configuration/overview.mdx and policy.mdx and deleted; sidebar Reference group updated. Overview carries location, schema, top-level fields, minimal config, validation (with the plan commands and chips), and the manifest contract; policy carries the complete tables for [[bundle]], [budgets] (with remedies), runners, [workflow]] (incl. the push_paths derivation), [host_requirements] (resolution rule, validation errors, status), plus the full and self-hosted examples. Cross-page links re-pointed: /reference/configuration/ -> overview, #host_requirements--host-level-declarations -> policy page fragment (from security/host-requirements.mdx x2). Import-depth lesson from task-102 applied up front (../../../../components). Build 27 pages zero warnings; biome 13 files green; link-check ledger is now exactly one pending link (/compatibility/, task-104).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The configuration reference is two pages: overview (schema, top-level fields, minimal config, validation) and Bundle/CI/host policy (every policy table with defaults and remedies) — independently deep-linkable, no key or table lost.
<!-- SECTION:FINAL_SUMMARY:END -->
