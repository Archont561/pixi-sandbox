---
id: TASK-73
title: Prove owner-operated GitHub App delivery for consumer upgrades
status: To Do
assignee: []
created_date: '2026-10-04 20:38'
labels:
  - ci
  - github-actions
  - release
  - consumer-proof
dependencies:
  - TASK-71
  - TASK-47
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/101'
  - 'https://github.com/Archont561/castellan/pull/13'
priority: high
type: enhancement
ordinal: 73000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-71 implements the optional workflow-capable token path and patch fallback, but its connected proof is still open. Define the supported production contract as an Archont561-operated GitHub App installed in an opted-in consumer repository. The generated workflow receives its short-lived installation token through PIXI_SANDBOX_UPGRADE_TOKEN, uses it for checkout, workflow-file push, and gh PR creation, and never pushes main. Consumers that cannot install the App retain the green artifact fallback from TASK-71.

Prove the complete reviewed upgrade cycle in Castellan: exact release discovery, regenerated owned files, workflow-capable delivery, human review, merge, explicit publisher dispatch, and exact-version manifest evidence.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The CI publishing guide documents the owner-operated App installation, repository access, required Workflows: write capability, secret name, rotation expectations, and the no-secret artifact fallback.
- [ ] #2 The generated workflow uses the injected installation token consistently for checkout, branch push, and gh PR creation; no production step resolves latest or pushes main.
- [ ] #3 A connected Castellan run with the owner-operated App creates a reviewable upgrade PR containing the exact self-updated version and matching generated templates.
- [ ] #4 After human merge and explicit publisher dispatch, the consumer transport manifest names the exact pixi-sandbox release and the generated publisher succeeds.
- [ ] #5 The fallback path remains green, produces the documented patch artifact, and does not claim the connected PR proof when no App token is present.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
