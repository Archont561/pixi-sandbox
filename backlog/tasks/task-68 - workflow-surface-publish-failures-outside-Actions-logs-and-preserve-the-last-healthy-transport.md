---
id: TASK-68
title: >-
  workflow: surface publish failures outside Actions logs and preserve the last
  healthy transport
status: In Progress
updated_date: '2026-10-04'
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - generated-workflow
  - diagnostics
  - publish
  - airlock
dependencies:
  - TASK-67
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/93'
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox-git
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: high
type: enhancement
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #93 asks the generated publisher to make failures diagnosable when the Actions log CDN is unavailable and to avoid replacing a healthy transport with incomplete output. Use the durable pipeline diagnostics from task-67, publish bounded failure evidence through a second GitHub-visible channel, and prove the publish boundary preserves the previous healthy orphan snapshot on every pre-publish or rejected-push failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The generated install → pack → doctor → publish lane writes one bounded diagnostic log and uploads it as an artifact on success or failure
- [x] #2 On failure, a second GitHub-visible surface independent of the Actions log CDN identifies the failed phase and exposes or links the bounded log without leaking secrets
- [x] #3 Workflow permissions and fork/event behavior are explicit; an unavailable comment or check API does not hide the original pipeline failure
- [x] #4 Pack or doctor failure never invokes publish, and any failed/rejected publish leaves the previously healthy transport branch byte-identical and fetchable
- [x] #5 The failure log is not committed into or published over the healthy transport branch; retention and size limits are documented
- [x] #6 Renderer and tempdir/bare-remote tests cover phase failures and transport preservation, generated output remains actionlint-clean and byte-stable under init --check
- [ ] #7 A connected consumer run demonstrates the secondary evidence path when the primary workflow step fails; record it in this task and issue #93
<!-- AC:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented the generated workflow diagnostics and transport preservation contracts test-first.

The generated publisher (`render_github_workflow`) in both bash and pwsh now captures all phase boundaries (`install`, `pack`, `doctor`, `publish`) and executes `pack`, `doctor`, and `publish` with durable `--log-file` paths under `$RUNNER_TEMP/pixi-sandbox-logs/`. On failure, the step writes a structured markdown summary directly to `$GITHUB_STEP_SUMMARY` naming the failed phase, branch, artifact name, and a bounded 50-line excerpt without exposing tokens or passwords. An `upload-artifact` step running on `always()` uploads the diagnostic directory as `publish-diagnostics-${{ matrix.platform }}` with 7-day retention.

A new test in `tests/cli.rs` (`publish_failure_preserves_the_existing_healthy_transport_branch`) proves that an invalid publish attempt or push failure leaves an existing healthy orphan transport snapshot completely unchanged, byte-identical, and fetchable/verifiable. Renderer tests in `tests/generated_workflow.rs` assert the presence of logging, artifact upload with retention, and failure step summary generation. `xtask lint-generated-workflow` (actionlint) passes across all workflow policy combinations; the full suite is 626 passed / 1 skipped (623/1 baseline).

AC#7 remains open: it requires a connected run in a consumer repo demonstrating the secondary step summary and uploaded artifact when a phase fails.
<!-- SECTION:NOTES:END -->

## Final Summary
<!-- SECTION:SUMMARY:BEGIN -->
Generated consumer publish workflows now capture bounded execution logs and expose failure diagnostics outside the Actions log CDN. Each leg records pipeline progress and command-level logs, formatting an actionable summary on `$GITHUB_STEP_SUMMARY` and uploading the diagnostics artifact on completion. Pre-publish failures halt immediately, ensuring existing healthy transport branches remain intact and fetchable. Six criteria are locally proven; the connected consumer run in AC#7 is open pending release/connected verification.
<!-- SECTION:SUMMARY:END -->

