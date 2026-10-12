---
id: TASK-102
title: Split the CLI reference into lifecycle pages
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 08:59'
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
- [x] #1 Five reference pages exist per the audit split, each starting with "Use this when…" and ending with failure modes and related guides.
- [x] #2 No command, flag, or diagnostic from the current page is lost; deep-linkable headings survive the split.
- [x] #3 The pages live under the Reference group of the new IA and are reachable from the lifecycle guides.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: reference/cli.mdx split into reference/cli/{overview,create-verify,publish,restore,tools}.mdx and deleted. Each page opens with 'Use this when...', carries the machine chips, and closes with safety implications, failure modes, and related guides; every flag, table, and diagnostic from the source page survives (verified by re-reading the old page against the five new ones during the split). Sidebar Reference group now lists the five CLI pages. Cross-references re-pointed: reference/cli/#doctor -> reference/cli/create-verify/#doctor (configuration.mdx, security/host-requirements.mdx), generic 'CLI reference' links -> reference/cli/overview/. Two mechanical lessons: pages under reference/cli/ need the ../../../../components import depth, and the split leaves no index at reference/cli/ so bare /reference/cli/ links must name a page. Added a frontmatter guard (fm-check.py, scratch) after the third YAML colon failure — all 30 pages pass. Build 26 pages zero warnings; biome 13 files green; link-check ledger reduced to task-103/104 targets (configuration/policy x6, compatibility x1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The CLI reference is five lifecycle pages (overview/diagnostics, create-verify, publish, restore, tools) with 'Use this when' intros, chips, safety implications, failure modes, and related guides; no flag or diagnostic lost, deep-linkable headings preserved.
<!-- SECTION:FINAL_SUMMARY:END -->
