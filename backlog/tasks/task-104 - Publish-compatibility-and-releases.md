---
id: TASK-104
title: Publish compatibility and releases
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:04'
labels:
  - docs
  - releases
  - compatibility
dependencies: []
references:
  - docs/astro.config.mjs
  - docs/src/content/docs/installation.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Readers cannot currently discover the support policy: supported pixi-sandbox versions, compatible Pixi versions, Rust/Cargo/OS support, generated-workflow compatibility, retired interfaces, and security fixes. Add a "Compatibility and releases" page carrying the current release and documentation version, a support matrix (OS, platform, Pixi, Git, Rust where relevant), an upgrade checklist, the generated-workflow upgrade policy, a migration table for retired composite Actions/reusable publisher paths, and a link to the GitHub releases feed — plus a compact version/release badge in the header or footer. A full version switcher is explicitly out of scope until multiple docs versions are maintained.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The page names the current release and the documentation version, with a support matrix covering OS, platform, Pixi, Git, and Rust requirements.
- [x] #2 It carries an upgrade checklist and the generated-workflow upgrade policy.
- [x] #3 A compact version/release badge appears in the header or footer and links to the releases feed.
- [x] #4 No version switcher is introduced.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: compatibility.mdx published under Overview and wired into the sidebar; footer override registered via starlight components.Footer. The custom Footer reuses Starlight's virtual EditLink/LastUpdated/Pagination and adds the version line (tag icon + 'Documented release: vX.Y.Z · Compatibility and releases · Release notes'); the resolver moved from astro.config.mjs into docs/src/version.mjs so component and content resolve the same value — verified in the built HTML (v0.6.2 on every page). Page content: documented release with links to GitHub releases and CHANGELOG; support matrix (five Pixi platforms, five release assets, connected-host and airlock requirements, Rust 1.88 MSRV, schema 1, generated-workflow marker); six-step upgrade checklist; the upgrade-job policy (weekly + dispatch, PR-only, pins never float, optional token fallback); retired-interface mapping table pointing at the migration guide; versioning policy stating no switcher yet. __VERSION__ substitution verified in the matrix. Biome caught the two now-unused node:fs/node:path imports in astro.config.mjs after the resolver move — removed. Build 28 pages zero warnings; biome 15 files clean; link-check fully green (the /compatibility/ ledger entry is now live).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Compatibility and releases page live with the support matrix, upgrade checklist, upgrade policy, and retired-interface mapping; every page footer carries the documented release with links to the compatibility page and release notes; no version switcher introduced.
<!-- SECTION:FINAL_SUMMARY:END -->
