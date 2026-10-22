---
id: TASK-122
title: Reduce GitHub Actions minutes in generated workflows RFC 179
status: To Do
assignee: []
created_date: '2026-10-22 07:37'
updated_date: '2026-10-22 07:37'
labels:
  - ci
  - workflow
  - optimization
dependencies:
  - TASK-118
  - TASK-119
  - TASK-120
  - TASK-121
priority: high
type: enhancement
ordinal: 122000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Umbrella task for issue #179 RFC to reduce GitHub Actions minutes and eliminate redundant steps in generated workflows.

Issue #179 reports 4 inefficiencies in publish-sandbox.yml and relock.yml that increase billing minutes and latency by 40-60%.

This umbrella tracks:
- TASK-118: double installation bootstrap optimization
- TASK-119: plan job VM overhead / static matrix rendering
- TASK-120: default concurrency for publish workflow
- TASK-121: path filtering for relock workflow

Overall goal: save 1-2 billed minutes per PR/push across all consumers, faster feedback, without weakening security (checksum verification, static linkage) or correctness (stale lock detection, transport consistency).

Reference: https://github.com/Archont561/pixi-sandbox/issues/179
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Create 4 child tasks for each inefficiency identified in issue 179
- [ ] #2 For each child, measure current cost and document design tradeoffs
- [ ] #3 Implement optimizations that preserve security and correctness guarantees
- [ ] #4 Verify with actionlint, check-repository, consumer-proof, and real consumer runs
<!-- AC:END -->
