---
id: TASK-62
title: 'Add a consumer-proof workflow for release and transport regressions'
status: In Progress
assignee: []
created_date: '2026-10-03'
updated_date: '2026-10-03'
labels:
  - testing
  - ci
  - release
  - consumer
dependencies:
  - TASK-52
  - TASK-53
  - TASK-54
priority: high
ordinal: 63000
references:
  - .github/workflows/airlock.yml
  - .github/workflows/release.yml
  - .github/workflows/publish-sandbox.yml
  - crates/xtask/src/airlock.rs
  - backlog/tasks/task-52 - Fix-the-generated-consumer-publisher-resolving-its-release-download-from-the-consumer-repository-issue-80.md
  - backlog/tasks/task-53 - Carry-consumer-publish-workflow-policy-in-pixi-sandbox.toml-so-init-output-regenerates-without-hand-edits-issue-79.md
  - backlog/tasks/task-54 - Make-pack-refuse-a-self-bin-that-cannot-run-standalone-and-prove-the-embedded-tool-executes-issue-81.md
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add a connected GitHub Actions workflow that proves the published release at the consumer boundary instead of waiting for a maintainer to run a separate consumer repository manually. The workflow must build a clean, temporary consumer-shaped project on a native runner, use the newly published pixi-sandbox release assets rather than the source checkout, and publish its evidence as artifacts and a check result.

The proof should cover the regressions tracked by issues #79, #80, and #81: `init` and regeneration with workflow policy are byte-identical without hand edits; generated bash and PowerShell download URLs resolve to `Archont561/pixi-sandbox` and checksum verification remains active; a downloaded standalone release asset executes in an isolated HOME; and a packed transport containing that asset can pass doctor, publish to a throwaway local remote, fetch, restore, and run the consumer lifecycle. The workflow should trigger on `release.published` and support `workflow_dispatch` with an optional release-tag override. It must fail closed when the release asset, checksum, generated workflow, or restored transport is invalid.

This workflow proves current releases and prevents recurrence. It does not by itself repair or repack an already-affected external consumer transport; that remains a separate remediation item.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `.github/workflows/consumer-proof.yml` triggers on `release.published` and `workflow_dispatch`, resolves the published tag safely, and does not use the source-built binary as the proof subject
- [ ] #2 A clean fixture consumer is created on the runner with an isolated HOME and no dependency on this repository's working tree or user profile
- [ ] #3 The workflow runs the real released binary to initialize the fixture, renders the publisher workflow, and proves a second initialization/regeneration is byte-identical without hand edits
- [ ] #4 Generated bash and PowerShell publisher workflows both reference the canonical pixi-sandbox release repository, retain SHA256SUMS verification, and pass actionlint or an equivalent checked renderer assertion
- [ ] #5 The workflow downloads the selected standalone release asset, verifies its SHA256SUMS entry, and executes `--version` and `--help` with a bare HOME and minimal PATH
- [ ] #6 The workflow packs a fixture transport with the verified standalone binary, runs doctor/verification, publishes to a throwaway local remote, fetches it as a fresh host, restores it offline, and runs the restored consumer assertion
- [ ] #7 Evidence includes the selected tag, asset name, checksums, generated workflow files, manifest, restore output, and test logs as bounded workflow artifacts; secrets and tokens are not uploaded
- [ ] #8 The workflow supports a release-tag override for rerunning historical proofs, rejects missing or non-release tags, and is safe to rerun concurrently
- [ ] #9 The proof is documented in the relevant issue/task notes and supplies the connected evidence required to close issues #79 and #80; issue #81 is closed only after the affected transport remediation is separately proven
- [ ] #10 Fixture tests, lint, generated-workflow checks, and the full test suite pass; the workflow is actionlint-clean and uses pinned third-party actions
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Create a dedicated workflow rather than adding a consumer simulation to the existing airlock matrix. Reuse the release-tag and asset-resolution helpers from the airlock machinery where their contracts fit, but keep the consumer fixture and release-asset download explicit so the proof boundary remains visible. Prefer a small xtask command for multi-step orchestration and unit-test its path/checksum/error behavior; keep workflow steps as pinned actions or one-line Pixi tasks.

Run the proof after a release is published, with a manual tag override for recovery and historical verification. Use a temporary fixture project and isolated HOME. Capture bounded evidence only after all checks pass or fail. Add issue/task evidence links from the workflow run, then update the task acceptance criteria only when the corresponding connected proof exists.
<!-- SECTION:PLAN:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-03 — started implementation. Added `.github/workflows/consumer-proof.yml`, triggered by published releases and manual release-tag dispatch. The workflow downloads a pinned Linux standalone asset, verifies SHA256SUMS, executes it with an isolated HOME, initializes a clean consumer fixture twice, checks byte-identical generated output, and uploads bounded evidence. Full release/consumer execution remains to be proven by the next release run.
<!-- SECTION:NOTES:END -->
