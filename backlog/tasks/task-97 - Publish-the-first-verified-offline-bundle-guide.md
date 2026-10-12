---
id: TASK-97
title: Publish the first verified offline bundle guide
status: To Do
assignee: []
created_date: '2026-10-12 07:55'
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
- [ ] #1 The guide walks connected host → configure → publish → airlock with, per stage, the actor, network condition, exact command, expected artifact/output, and a link to the relevant failure guidance.
- [ ] #2 The publish stage shows how to run or trigger the generated workflow and names the expected sandbox/<bundle>-<platform> branch.
- [ ] #3 The airlock stage clones/imports a bundle, runs ./restore.sh, and proves the expected command succeeds with --offline / --frozen.
- [ ] #4 Audit acceptance: a new user can complete a minimal transport with one documented Pixi project, identify the expected sandbox branch, restore on a simulated offline machine, and verify a documented command succeeds without network access.
- [ ] #5 The visual sequence uses Connected / CI / Transfer / Airlock labels consistent with the homepage diagram.
<!-- AC:END -->
