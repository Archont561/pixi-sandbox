---
id: TASK-101
title: Add machine and network context chips to command blocks
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 08:54'
labels:
  - docs
  - ux
  - safety
dependencies: []
references:
  - docs/src/components/
  - docs/src/content/docs/restore.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Running a connected-host command on an airlock is a high-cost mistake, and today the machine/network context is prose. Add compact context chips above command blocks across guides and reference: Connected host, Airlock, CI, Network required, Offline only, Read-only verification. Chips must be semantic — text plus icon, never color alone — so screen readers and print get the same signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A shared chip component renders the six audited states with accessible labels.
- [x] #2 Command blocks in the guides and reference pages that run under a specific machine/network condition carry the matching chip.
- [x] #3 The airlock restore page marks every command that must never touch the network.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: ContextChips.astro renders the six states from a single map (lucide icon + text label; accessible because the state is text, not color). Applied to 41 command blocks across first-offline-bundle, restore, configure-bundles, installation, ci-publishing, cargo-vendoring, sizes-and-budgets, security/diagnostics, reference/cli, reference/configuration. AC#3: all six restore.mdx blocks carry airlock+offline (plus readonly on the two doctor blocks). Two mechanical lessons recorded for future passes: MDX in guides//security//reference/ needs the ../../../components import depth, and pages under a TabItem need the chip indented to the fence. The configure-bundles Rust tab was split from one mixed block into four separately chipped blocks (build/pack/verify/publish) so doctor reads read-only and publish reads network. Verified in built HTML: chips render with icon+label on every chipped page. Build 22 pages zero warnings; biome 13 files green; link ledger unchanged.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Command blocks across the guides and reference now carry machine/network context chips (connected, airlock, CI, network, offline, read-only) as icon+text, with every airlock restore command marked offline.
<!-- SECTION:FINAL_SUMMARY:END -->
