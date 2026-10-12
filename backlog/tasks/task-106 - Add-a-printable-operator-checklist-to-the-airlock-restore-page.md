---
id: TASK-106
title: Add a printable operator checklist to the airlock restore page
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:15'
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
- [x] #1 A compact checklist sits at the top of the restore page covering verify → transfer → restore → prove offline.
- [x] #2 The checklist is usable printed and with JS disabled.
- [x] #3 Each step links to its detailed section and to the relevant troubleshooting entry.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: restore.mdx gained an "Operator checklist" box directly under the intro and above section 1 (it also surfaces in the page TOC). AC#1: seven tick-boxes across four phases — Transfer (bundle both refs on the connected host), Verify (doctor --branch-location --verify, read-only, hard stop on mismatch), Restore (./restore.sh or pixi-sandbox restore --output-path . --force, then open a new shell for the registered user tools), Prove offline-clean (pixi install --frozen --offline must be a no-op; pixi run --frozen -- cargo build --offline; run the workload, with a missing GUI/font/display server framed as a [host_requirements] finding), plus a stop rule (nothing may reach the network). AC#3: every step links to its detailed section (§1–§4 in-page anchors, verified present as heading ids in the built HTML) and to the matching troubleshooting symptom entries (1, 2, 3, 4, 5, 6, 8). AC#2: the box is static markup — CSS-drawn checkboxes (border boxes, not glyphs, so they print in any font), string-style HTML consistent with the page's existing blocks, zero client JS; the container carries break-inside: avoid so it does not split across printed pages, and Starlight's print stylesheet (verified in dist/_astro/print.*.css) hides chrome, sets content width to 100%, and pre-wraps code blocks. Build 28 pages, zero warnings; frontmatter check clean; link-check 1309 links with the only failures the known favicon defect (task-110).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The airlock restore page now opens with a compact printable checklist covering transfer → verify → restore → prove-offline, each step linked to its detailed section and its troubleshooting symptom, built as static no-JS markup that prints with the page.
<!-- SECTION:FINAL_SUMMARY:END -->
