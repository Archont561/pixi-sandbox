---
id: TASK-108
title: Give the split reference pages a wider content mode
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:24'
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
- [x] #1 Reference pages render with a wider content column at 1280–1440px; wide tables and TOML blocks no longer wrap excessively.
- [x] #2 The right-side ToC and the mobile layout are unchanged.
- [x] #3 The layout change is scoped to reference pages, not guides.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: implemented as a Starlight component override rather than per-page frontmatter, so every current and future reference page inherits it with zero repetition. New docs/src/components/ContentPanel.astro replicates Starlight's default markup plus its `@layer starlight.core` styles (padding, container max-width, `> :global(* + *)` spacing, the 72rem margin-inline rule) and adds a `reference-wide` class when `Astro.locals.starlightRoute.entry.id` starts with `reference/`. Registered as `components.ContentPanel` in docs/astro.config.mjs. AC#1: `.reference-wide { --sl-content-width: 56rem }` — unlayered, so it beats Starlight's `@layer starlight.base :root` default (45rem); at 1280px the content column goes 720→850px and at 1440px 720→896px, so wide TOML blocks, CLI flag tables, and the budget/reference tables stop wrapping. AC#2: the property is set on `.content-panel` — a *descendant* level — precisely because the right-side ToC column (`.right-sidebar-container`) and the header grid read `--sl-content-width` from :root and live outside the panel, so their geometry is bit-identical; `max-width` only caps, so the mobile single-column layout below 72rem is unchanged. AC#3: verified in the built HTML — reference/cli/publish and reference/configuration/policy carry the class (content + footer panels), guides/ci-publishing, index, and restore carry zero occurrences, and the ToC aside is present on all of them. Verified the emitted CSS: the copied defaults sit inside `@layer starlight.core` and `.reference-wide:where(.astro-*)` is outside every layer. Build 28 pages; biome 16 files clean (the new .astro included); link-check unchanged (1313 links, only the known favicon defect, task-110). The `[WARN] Could not render /404` line in build output is the pre-existing catch-all/slug conflict notice, not introduced here.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Reference pages now render with a wider content column (56rem vs 45rem) through a route-scoped ContentPanel override; the right-side ToC, the header grid, and the mobile layout keep their exact current geometry, and guides and all other pages are untouched.
<!-- SECTION:FINAL_SUMMARY:END -->
