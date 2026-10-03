---
id: TASK-63
title: 'Fix publish bootstrap checksum path resolution (issue 87)'
status: In Progress
assignee: []
created_date: '2026-10-03'
updated_date: '2026-10-03'
labels:
  - bug
  - ci
  - generated-workflow
  - release
dependencies:
  - TASK-52
priority: high
ordinal: 64000
references:
  - .github/workflows/publish-sandbox.yml
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml
  - https://github.com/Archont561/pixi-sandbox/issues/87
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #87 reports that the v0.5.1 generated consumer publisher always fails during its Unix bootstrap checksum verification. The downloaded binary is stored under `$RUNNER_TEMP`, but the publish-job `sha256sum --check` receives a checksum line containing only the asset filename and runs from the consumer checkout. The file cannot be opened, `--status` hides the diagnostic, and every publish job exits at the download step even when the release and asset are valid.

The upgrade bootstrap in the same generated workflow already rewrites the checksum line to the absolute binary path. Apply the equivalent fix to the publish bootstrap, update the generated fixture, and make the verification failure attributable. Add execution-level coverage for the generated shell snippet against a temporary asset and `SHA256SUMS`; string snapshots alone must not be the only guard because the existing fixture encoded the broken behavior.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The Unix publish bootstrap verifies the downloaded asset at its `$RUNNER_TEMP` path, without depending on the consumer checkout containing a copy of the asset
- [x] #2 A checksum mismatch or missing asset fails with a clear error naming the asset and verification step; successful verification remains fail-closed and executes no unverified binary
- [x] #3 Generated workflow fixtures and renderer expectations are updated, and the generated workflow remains actionlint-clean for both publish and upgrade paths
- [ ] #4 Fixture-based execution tests run the generated Unix bootstrap against a valid temporary asset and `SHA256SUMS`, and cover missing/mismatched assets; tests do not use this repository or a real HOME
- [x] #5 The Windows bootstrap remains correct and is covered by the existing renderer/execution assertions
- [ ] #6 The fix is included in a released patch and a consumer-shaped publish proof succeeds using the released generated workflow; record the workflow run in this task and issue #87
- [x] #7 Formatting, lint, generated-workflow checks, and the full test suite pass
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Change the publish Unix template to rewrite the checksum entry from the bare asset name to the absolute downloaded path, matching the proven upgrade path. Improve the error surface without weakening checksum enforcement. Extract only the shell fragment that needs execution-level testing into a fixture-driven helper or xtask test seam; keep workflow steps within the repository's one-line/task conventions. Regenerate the committed fixture, run actionlint, then validate the released workflow through the consumer-proof workflow from task 62.
<!-- SECTION:PLAN:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-03 — fixed the Unix publish bootstrap template and generated fixture to rewrite the checksum entry to the downloaded `$RUNNER_TEMP` path, matching the upgrade bootstrap. Verification failures now emit an explicit SHA256 error. Release and consumer-proof validation remains pending.

2026-10-03, evidence reconciliation — v0.5.2 contains the absolute `$RUNNER_TEMP` checksum rewrite and explicit failure message. CI run 37136644662 passed formatting, lint, actionlint/generated-workflow checks, and the full suite; the reviewed fixture covers the unchanged PowerShell path. AC#1-3, #5, and #7 are complete.

Still open: AC#4 and #6. There is no execution-level fixture test for valid/missing/mismatched Unix checksum inputs yet, and consumer-proof run 37137917674 inspects the generated publisher but does not execute its publish bootstrap. Add that test, then run a consumer-shaped publish using the v0.5.2-generated workflow and record it here and on issue #87.
<!-- SECTION:NOTES:END -->
