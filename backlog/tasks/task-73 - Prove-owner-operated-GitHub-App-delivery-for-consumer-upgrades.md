---
id: TASK-73
title: Prove owner-operated GitHub App delivery for consumer upgrades
status: Deprecated
assignee: []
created_date: '2026-10-04 20:38'
updated_date: '2026-10-15 21:27'
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

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-15 — Deprecated at the owner's direction. The owner-operated GitHub App delivery model is no longer pursued, so this task closes unimplemented: none of its criteria was proven, and none will be.

Nothing shipped changes. TASK-71's optional credential path stays as released. A consumer that stores a workflow-capable token (a PAT or any GitHub App token) in `PIXI_SANDBOX_UPGRADE_TOKEN` gets its upgrade PR opened automatically. Without one, the lane stays green and uploads a `git apply`-able patch artifact. The CI publishing and migration guides document that generic secret, not an owner-operated App, so they stay accurate.

Still open elsewhere:
- Issue #101 stays open. It reports that `GITHUB_TOKEN` cannot push `.github/workflows/*`, and this decision does not change that.
- TASK-74 AC#7 and its implementation plan name this task's credential model; see the 2026-10-15 note there.
- doc-10 still specifies an owner-operated App for cross-repository starter publication.
<!-- SECTION:NOTES:END -->
