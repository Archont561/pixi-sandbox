---
id: TASK-96
title: Restructure the docs navigation around roles and lifecycle phases
status: Done
assignee: []
created_date: '2026-10-12 07:55'
updated_date: '2026-10-12 08:39'
labels:
  - docs
  - navigation
  - information-architecture
dependencies: []
references:
  - docs/astro.config.mjs
  - docs/src/content/docs/
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The 13-page docs set is organized by artifact (Start here / Guides / Reference / Design), so the four audiences — project owner, CI maintainer, airlock operator, security/platform reviewer — must infer which pages apply to them. Rebuild the Starlight sidebar into the eight-group IA from the audit plan (doc-18): Overview, Start here, For project owners, For CI maintainers, For airlock operators, Security and operations, Reference, Project.

Re-home every existing page into that IA, add the "How it works" lifecycle page and the "Choose your role" page, and move design/repository material into Project. Preserve the branded hero and transport diagram: this task changes navigation and adds two foundational pages, not the visual identity.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Sidebar in docs/astro.config.mjs implements the eight-group IA from doc-18, and every existing docs page is re-homed into it with its content intact.
- [x] #2 New "How it works" page carries one lifecycle diagram, what exists on each machine, what crosses the air gap, what is verified, and which files are generated or committed.
- [x] #3 New "Choose your role" page links each of the four roles to its tailored first page in one click.
- [x] #4 pixi run --frozen bun --filter=pixi-sandbox-docs run build stays warning-free and the biome lint-docs gate stays green.
- [x] #5 Search, ToC, edit-page link, and previous/next navigation still work on every moved page; no internal link is left pointing at a moved path.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: Sidebar rebuilt into the eight-group IA in docs/astro.config.mjs; every existing page re-homed with git mv (quickstart -> first-offline-bundle, using-in-your-project -> configure-bundles, actions -> migration, host-requirements -> security/, design+repository -> project/). reference/actions.mdx deleted: it was a verbatim duplicate of guides/actions.mdx and the IA merges both into the migration page (task-105 expands it). New pages: how-it-works (six-stage lifecycle, per-machine table, branch contents, verification table, init-owned files), choose-your-role (four role paths with the mistake to avoid each), plus the IA-required extractions cargo-vendoring, sizes-and-budgets, workflow-policy, security/verification-guarantees, security/diagnostics. configure-bundles and ci-publishing trimmed where their content moved out; every internal link re-pointed by sed sweep verified with a scratch link checker over docs/dist. Build: 20 pages (was 14), zero warnings; biome 11 files green. Forward links to pages that land with tasks 99/100/102/103 are deliberate and tracked by the checker as the running ledger (9 pending: troubleshooting x5, reference/configuration/policy x3, reference/cli/publish x1); the pre-existing missing favicon.svg (14 links, every page) is recorded for the task-110 link gate. Groups gain their remaining audit-listed pages (glossary, compatibility, troubleshooting, the five CLI pages, the two config pages) as those tasks land, each wiring its own sidebar entry, so every commit keeps a working link-clean site.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Docs navigation restructured around roles and lifecycle phases: eight-group IA, all 13 existing pages re-homed, seven new pages (how-it-works, choose-your-role, cargo-vendoring, sizes-and-budgets, workflow-policy, verification-guarantees, diagnostics), duplicate reference/actions page removed, and every internal link re-pointed. Build 20 pages zero warnings, biome green.
<!-- SECTION:FINAL_SUMMARY:END -->
