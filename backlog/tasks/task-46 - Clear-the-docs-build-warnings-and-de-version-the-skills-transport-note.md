---
id: TASK-46
title: Clear the docs build warnings and de-version the skill's transport note
status: Done
assignee:
  - '@agent'
created_date: '2026-10-02 17:30'
updated_date: '2026-10-02 17:50'
labels:
  - docs
  - maintenance
dependencies: []
references:
  - docs/src/content.config.ts
  - docs/astro.config.mjs
  - .agents/skills/session/SKILL.md
priority: low
type: chore
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Astro docs build exits 0 but prints three recurring warnings that bury real signal:
`[vite] MODULE_LEVEL_DIRECTIVE` for `"use astro:head-inject"` on every MDX page, `[content]
The collection "i18n" does not exist or is empty`, and `[content] Entry docs → 404 was not
found` with no `/404.html` emitted. Resolve each at its root (or suppress with a targeted,
commented filter when the warning is upstream noise the build already handles), and rewrite
the session skill's "The published branch carries 0.4.0" paragraph so it states how to read
the published manifest instead of a version that rots every release.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `pixi run --frozen bun --filter=pixi-sandbox-docs run build` completes with zero warnings
- [x] #2 the lint gates that cover docs (biome, lint-docs) stay green
- [x] #3 the session skill names no pixi-sandbox version for the published transport; it says how to check what the transport carries
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Spike against the installed sources in node_modules first: identify what emits each warning
(Starlight's content-layer collections, the head-propagation directive Astro's MDX pipeline
emits, and whatever references the `404` docs entry), then fix or narrowly suppress with a
comment naming the upstream cause. Re-run the build until the warning count is zero.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: All three warnings traced to their emitters in the installed tree and fixed at the root. (1) `use astro:head-inject`: Astro's own `content/vite-plugin-content-assets.js` prepends the directive to every propagated-assets virtual module; by the time Rollup bundles them the semantics are applied, so the MODULE_LEVEL_DIRECTIVE warning is upstream noise — filtered with a targeted `vite.build.rollupOptions.onwarn` that passes everything else through. (2) Empty `i18n` collection: Starlight's runtime calls `getCollection("i18n")` on every page; the collection is now declared (`i18nLoader`/`i18nSchema`) and carries one real override, `page.editLink: "Edit this page on GitHub"`, so the file's purpose is self-evident. (3) Missing 404: Starlight resolves its 404 route by the reserved docs slug `404` (`getEntry("docs", "404")`); `docs/src/content/docs/404.mdx` now provides the page, in house hero style. That page surfaced the second half of the same upstream wart: Starlight's `[...slug]` catch-all lists every docs entry, so its own `404` route and the catch-all both claim `/404` by design — silenced with `prerenderConflictBehavior: "ignore"`, which is effectively narrow here because Starlight owns the only two routes this site has; the dedicated route wins and renders `404.mdx` into `/404.html` either way (verified in `docs/dist/404.html`). Build output: zero warnings, 19 pages, override text present in the emitted HTML. The session skill's transport paragraph now tells the reader to read the published manifest instead of naming a version that rots. Gates: fmt, lint (biome 11 files, check-repository), suite 405/1.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The docs build is warning-free with every warning fixed at its root or narrowly filtered with the upstream cause documented: the rollup head-inject filter, the declared i18n collection with a real edit-link override, the custom `404.mdx` page plus the conflict-behavior setting its reserved slug requires, and a version-agnostic transport note in the session skill. Suite 405/1.
<!-- SECTION:SUMMARY:END -->
