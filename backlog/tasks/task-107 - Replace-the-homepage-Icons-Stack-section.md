---
id: TASK-107
title: Replace the homepage Icons & Stack section
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 09:18'
labels:
  - docs
  - homepage
  - ux
dependencies:
  - TASK-98
references:
  - docs/src/content/docs/index.mdx
  - docs/src/content/docs/design.mdx
  - docs/src/content/docs/repository.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: low
type: docs
ordinal: 107000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The badge strip, six feature cards, Quick Start, and "Icons & Stack" compete for attention after the hero. With the role cards in place, replace "Icons & Stack" with useful endings — role paths, comparison, support, or security information — and move design-system/icon details into the contributor docs (design/repository pages).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 "Icons & Stack" is gone from the homepage; the section it leaves is replaced with role paths, support, or security content.
- [x] #2 Design-system and icon-stack details live in the Project docs instead.
- [x] #3 The homepage still ends with a clear next operational step.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: AC#1 — the "Icons & Stack" section (Astro Icon/Iconify prose plus the six-item icon showcase) is gone from index.mdx, replaced by a "Security and support" row of three link cards: Verification guarantees (→ security/verification-guarantees/), Compatibility and releases (→ /compatibility/), Troubleshooting (→ /troubleshooting/), rendered as static linked cards in the same idiom as the restore page's "does not contain" block; verified absent from the built HTML (icon-showcase/Iconify probes both false). AC#2 — the design-system and icon-stack details moved into project/repository.mdx under "Editing this documentation" as "Icons and the docs design system": the astro-icon import/call shape, the four bundled sets (lucide, mdi, tabler, simple-icons — verified against docs/astro.config.mjs), the Iconify browse link, and the fact that astro-icon emits SVGs aria-hidden (verified in the built HTML) so icons never carry meaning alone. The dead .icon-showcase/.icon-item CSS was removed from src/styles/custom.css. AC#3 — the homepage now ends with a "Start now" CTA box (rocket icon) pointing at the Quick Start commands above and then First verified offline bundle; verified as the last block in the built HTML. Also added ContextChips (connected/network) to the Quick Start command block for consistency with the rest of the site after task-101. Build 28 pages, zero warnings; biome 15 files clean; link-check 1313 links, failures only the known favicon defect (task-110).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The homepage's Icons & Stack section is replaced by a security-and-support row linking verification guarantees, compatibility, and troubleshooting; icon-stack/design-system details now live in the Project docs (repository page) with the dead showcase CSS removed; the page closes with a clear "Start now" operational step.
<!-- SECTION:FINAL_SUMMARY:END -->
