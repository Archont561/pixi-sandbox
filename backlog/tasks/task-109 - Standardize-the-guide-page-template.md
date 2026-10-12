---
id: TASK-109
title: Standardize the guide page template
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:36'
labels:
  - docs
  - templates
  - guides
dependencies: []
references:
  - docs/src/content/docs/guides/
  - docs/src/content/docs/restore.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: low
type: docs
ordinal: 109000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Standardize every guide on one template: it opens with the outcome, audience, prerequisites, machine context, and maturity, and closes with verification, failure modes, and next steps. Apply it across the guides in the new IA so each stage of the transport lifecycle links to the next safe action and its prerequisite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A documented template exists and every guide page follows it (outcome, audience, prerequisites, machine context, maturity; verify, failures, next steps).
- [x] #2 Each lifecycle stage links to the next safe action and its prerequisite.
- [x] #3 The restore page keeps its operator checklist inside the template.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: AC#1 — new docs/src/components/GuideMeta.astro renders the standard opening box (Outcome / Audience / Prerequisites / Machine context / Maturity) as a compact, border-and-text, no-JS, print-friendly table (break-inside: avoid, single-column under 30rem); values accept inline JSX so <code> and links work. The template itself is documented in project/repository.mdx under "Editing this documentation" → "The guide template", which also names restore.mdx as the reference implementation and states the closing rule (Verification → Failure modes → Next steps, in that order, under those exact headings). Applied to all six guides (cargo-vendoring, ci-publishing, configure-bundles, migration, sizes-and-budgets, workflow-policy) plus the two operator pages (restore.mdx, first-offline-bundle.mdx): each now opens with the meta box and closes with the three standard sections; the old per-page "See also" lists were folded into "Next steps", and "Proving it is offline" / "You have proved" were renamed to the standard "## Verification" heading. AC#2 — every Next steps names the next safe action and its prerequisite: configure-bundles → restore (branch carried across the gap); sizes-and-budgets → publish (verified, within-budget transport); cargo-vendoring → restore (published branch); workflow-policy → ci-publishing (generated workflow committed); ci-publishing → restore (published branch in a bundle); migration → ci-publishing (workflow + config committed); first-offline-bundle → the three deeper guides each with their prerequisite; restore → run the workload, or the symptom entry when something failed. AC#3 — restore.mdx keeps its printable operator checklist, now sitting directly under the meta box and still above section 1 (verified in the built HTML), and gained the three closing sections; its Failure modes table maps each symptom to its troubleshooting entry (1, 2, 3, 4, 5, 6, 8) plus the --log-file evidence pointer. Verified in dist: all eight pages carry guide-meta + id="verification"/"failure-modes"/"next-steps". Build 28 pages, zero warnings; frontmatter check clean; link-check 1337 links with only the known favicon failures (task-110); biome 17 files clean (GuideMeta.astro included).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Every guide now follows one documented template — a GuideMeta opening box (outcome, audience, prerequisites, machine context, maturity) and closing Verification / Failure modes / Next steps sections — with each lifecycle stage linking to the next safe action and its prerequisite, and the restore page keeping its printable operator checklist inside the template.
<!-- SECTION:FINAL_SUMMARY:END -->
