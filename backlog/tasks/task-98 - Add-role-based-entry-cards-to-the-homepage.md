---
id: TASK-98
title: Add role-based entry cards to the homepage
status: Done
assignee: []
created_date: '2026-10-12 07:55'
updated_date: '2026-10-12 08:43'
labels:
  - docs
  - homepage
  - roles
dependencies:
  - TASK-96
references:
  - docs/src/content/docs/index.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 98000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Immediately after the hero, add four entry cards so each audience can pick a role and reach a tailored first page in one click: project owner → first offline bundle; CI maintainer → CI publishing; airlock operator → restore on the airlock; security/platform reviewer → security and host boundaries. Keep the transport diagram as the lead visual and use text plus icons, never color alone.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Four role cards sit directly under the hero, each stating the primary outcome and linking its first page.
- [x] #2 Card destinations resolve inside the new IA from task-96.
- [x] #3 The homepage still leads with the Pixi project → orphan branch → airlock diagram and the zero-network language.
- [x] #4 Role selection is keyboard reachable and is not signaled by color alone.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: New RoleCards.astro component (four roles, outcome text, first-page link, lucide icon + arrow CTA) placed immediately after the hero badge strip under a 'Choose your path' heading, before the what-is/diagram story. Destinations: first-offline-bundle, guides/ci-publishing, restore, security/verification-guarantees — all exist in the new IA. Cards are real anchors (keyboard reachable), icon+text so color is never the only signal, focus-visible ring, responsive auto-fit grid; styles appended to custom.css in the house pattern. Verified in the built HTML: four role-card anchors with the expected hrefs. Build 20 pages zero warnings; biome 12 files green.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Homepage carries four role entry cards directly under the hero, each with its outcome and first-page link, keyboard reachable and not color-coded, with the transport diagram retained as the lead visual.
<!-- SECTION:FINAL_SUMMARY:END -->
