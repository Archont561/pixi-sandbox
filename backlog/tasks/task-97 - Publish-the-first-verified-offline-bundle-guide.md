---
id: TASK-97
title: Publish the first verified offline bundle guide
status: Done
assignee: []
created_date: '2026-10-12 07:55'
updated_date: '2026-10-12 08:42'
labels:
  - docs
  - quickstart
  - journey
  - airlock
dependencies:
  - TASK-96
references:
  - docs/src/content/docs/quickstart.mdx
  - docs/src/content/docs/index.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the Quickstart with an end-to-end, role-aware deployment exercise. The current Quick Start stops at pixi-sandbox init and never demonstrates the defining promise: a verified offline restore. Build the four-stage path — connected host, configure, publish, airlock — as a horizontal sequence matching the hero diagram, where each stage names who performs it, the machine/network condition, the exact command, the expected artifact (including the sandbox/<bundle>-<platform> branch name), and a failure link.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The guide walks connected host → configure → publish → airlock with, per stage, the actor, network condition, exact command, expected artifact/output, and a link to the relevant failure guidance.
- [x] #2 The publish stage shows how to run or trigger the generated workflow and names the expected sandbox/<bundle>-<platform> branch.
- [x] #3 The airlock stage clones/imports a bundle, runs ./restore.sh, and proves the expected command succeeds with --offline / --frozen.
- [x] #4 Audit acceptance: a new user can complete a minimal transport with one documented Pixi project, identify the expected sandbox branch, restore on a simulated offline machine, and verify a documented command succeeds without network access.
- [x] #5 The visual sequence uses Connected / CI / Transfer / Airlock labels consistent with the homepage diagram.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: first-offline-bundle.mdx rewritten as the four-stage journey. Stage strip (Connected/Configure/CI/Airlock) rendered in the homepage diagram's visual language (yellow/blue/green stage cards, responsive grid). The worked project is one env (default) x one platform (linux-64), bundle 'minimal', so the expected branch sandbox/minimal-linux-64 is derived and named in stages 2-4. Every stage carries actor, machine/network condition, exact command, expected artifact or output, and failure links (installation, configure-bundles, diagnostics, ci-publishing, troubleshooting, restore). Stage 4 uses the documented transfer shape (git bundle with both refs, clone, ./restore.sh, PIXI_SANDBOX_FETCH=skip for local-only) and ends on the offline proof: pixi install --frozen --offline must be a no-op, plus cargo build --offline for Rust projects. Commands checked against the CLI reference and the generated-workflow contract (plan -> pack -> doctor --verify -> publish, budgets before force-push, publish-diagnostics artifacts). Old quickstart maintainer material moved to project/repository.mdx with the real task names (pixi run -e package package, xtask smoke-conda-package, pixi run --frozen test/lint). Build 20 pages zero warnings; biome green; link-check ledger unchanged (pending links all belong to later tasks).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The Quickstart is replaced by a four-stage verified journey (Connected, Configure, CI, Airlock) with per-stage actor, machine/network condition, exact command, expected artifact, and failure links, ending in the offline proof (pixi install --frozen --offline no-op). A new user can complete and verify one minimal transport using only this page.
<!-- SECTION:FINAL_SUMMARY:END -->
