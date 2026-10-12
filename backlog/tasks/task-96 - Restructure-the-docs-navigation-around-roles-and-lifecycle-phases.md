---
id: TASK-96
title: Restructure the docs navigation around roles and lifecycle phases
status: To Do
assignee: []
created_date: '2026-10-12 07:55'
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
- [ ] #1 Sidebar in docs/astro.config.mjs implements the eight-group IA from doc-18, and every existing docs page is re-homed into it with its content intact.
- [ ] #2 New "How it works" page carries one lifecycle diagram, what exists on each machine, what crosses the air gap, what is verified, and which files are generated or committed.
- [ ] #3 New "Choose your role" page links each of the four roles to its tailored first page in one click.
- [ ] #4 pixi run --frozen bun --filter=pixi-sandbox-docs run build stays warning-free and the biome lint-docs gate stays green.
- [ ] #5 Search, ToC, edit-page link, and previous/next navigation still work on every moved page; no internal link is left pointing at a moved path.
<!-- AC:END -->
