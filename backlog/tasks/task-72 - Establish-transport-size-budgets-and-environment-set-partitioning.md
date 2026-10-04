---
id: TASK-72
title: Establish transport size budgets and environment-set partitioning
status: To Do
assignee: []
created_date: '2026-10-04 20:38'
labels:
  - transport
  - airlock
  - architecture
  - measurement
dependencies: []
references:
  - 'https://github.com/Archont561/castellan/pull/13'
priority: high
type: spike
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The Castellan transport is approximately 1.86 GiB and is approaching GitHub practical repository and push limits. Measure the current transport model before changing the manifest or restore protocol. Keep environment sets as the partition boundary: do not compose partial environments or introduce external payloads without evidence.

Establish reviewed budgets for the largest blob, packed transport, repository/push size, fetch/clone time, restore disk headroom, and publish behavior. Use fixture/tempdir transports plus the connected Castellan transport as measurements. If the budget requires partitioning, make the planner/configuration expose separate environment-set bundles and preserve self-contained, backward-compatible transports.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A documented measurement records current largest blob, packed transport size, repository size, fetch/clone cost, restore disk requirement, and publish/push behavior for representative fixture and Castellan transports.
- [ ] #2 Reviewed hard refusal thresholds exist for blob, transport, repository/push, and restore-disk limits, with actionable errors naming the exceeded budget and the partition remedy.
- [ ] #3 The planner/configuration can represent separate environment-set bundles without splitting an individual environment or weakening manifest self-containment; existing single-bundle transports remain restorable.
- [ ] #4 Tempdir/fixture tests cover under-budget, over-budget, and partitioned environment-set plans; no network or this checkout is used as a test fixture.
- [ ] #5 The spike records whether external immutable payloads are necessary; no payload-format migration is implemented unless the measurements demonstrate that environment-set partitioning is insufficient.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
