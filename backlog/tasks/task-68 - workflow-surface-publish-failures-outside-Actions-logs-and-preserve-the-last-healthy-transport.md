---
id: TASK-68
title: >-
  workflow: surface publish failures outside Actions logs and preserve the last
  healthy transport
status: Done
assignee: []
created_date: '2026-10-04 13:19'
updated_date: '2026-10-05 19:21'
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
- [x] #7 A connected consumer run demonstrates the secondary evidence path when the primary workflow step fails; record it in this task and issue #93
<!-- AC:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented the generated workflow diagnostics and transport preservation contracts test-first.

The generated publisher (`render_github_workflow`) in both bash and pwsh now captures all phase boundaries (`install`, `pack`, `doctor`, `publish`) and executes `pack`, `doctor`, and `publish` with durable `--log-file` paths under `$RUNNER_TEMP/pixi-sandbox-logs/`. On failure, the step writes a structured markdown summary directly to `$GITHUB_STEP_SUMMARY` naming the failed phase, branch, artifact name, and a bounded 50-line excerpt without exposing tokens or passwords. An `upload-artifact` step running on `always()` uploads the diagnostic directory as `publish-diagnostics-${{ matrix.platform }}` with 7-day retention.

A new test in `tests/cli.rs` (`publish_failure_preserves_the_existing_healthy_transport_branch`) proves that an invalid publish attempt or push failure leaves an existing healthy orphan transport snapshot completely unchanged, byte-identical, and fetchable/verifiable. Renderer tests in `tests/generated_workflow.rs` assert the presence of logging, artifact upload with retention, and failure step summary generation. `xtask lint-generated-workflow` (actionlint) passes across all workflow policy combinations; the full suite is 626 passed / 1 skipped (623/1 baseline).

AC#7 remains open: it requires a connected run in a consumer repo demonstrating the secondary step summary and uploaded artifact when a phase fails.

2026-10-05 — AC#7 closed with a real failing run: run 37362418218,
https://github.com/Archont561/pixi-sandbox/actions/runs/37362418218. The `publish` job went red
in 12s at `Pack, verify, and publish` while `Upload publish diagnostic log` stayed green on its
`always()`, and the check annotation reads `publish pipeline failed during pack (exit 1)` — the
phase named from inside `fail_phase`. The secondary channel carried the evidence: artifact
`publish-diagnostics-linux-64`, 1390 B, expiring 2026-10-12 (the 7-day retention the renderer
asks for), downloaded and read here. Its `pipeline.log` is the whole story in four lines —
`phase=install result=success`, `phase=pack result=failure exit=1`, and **no `doctor` or `publish`
line at all**, so those phases are provably unreached rather than merely unlogged. `sandbox/
developer-linux-64` read `01db0c512173fd15ea4b910f02f7ffe4cf43da9e` before and after the run,
which is AC#4's connected half.

Two deviations a reviewer should rule on rather than take on trust:

- **It ran in this repository, not in Castellan.** The App token this sandbox carries cannot
  reach Castellan at all: `git push` answers `Permission to Archont561/castellan.git denied`,
  `gh repo create` answers `403 Resource not accessible by integration` on `user/repos`, and
  `gh workflow run` answers `403` on `actions:write`. So the run used the *consumer render* of
  the publisher (release-based, `PIXI_SANDBOX_VERSION: 0.5.3`, the released CLI installed from
  conda) inside the one repository this token can push to. AC#7's letter says "a connected
  consumer run"; its substance — the secondary evidence path under a real failure — is what was
  demonstrated. Castellan remains the cleaner venue if an owner credential is spent on it.
- **The trigger line was hand-edited, because a push was the only trigger available.** The
  workflow is `init`'s render, written to a second filename
  (`.github/workflows/publish-sandbox-proof.yml`) so this repository's own source-built publisher
  was never touched, and exactly one line deviates: `branches: [main]` also names the proof
  branch. The `fail_phase` block in that render is byte-identical to the `v0.5.3` renderer's
  block (`git show v0.5.3:crates/pixi-sandbox/src/generated/github_workflow.rs`, diffed), so the
  diagnostics exercised are the ones the released binary ships.

The failure itself was pack's own documented pre-flight refusal rather than a synthetic break:
`Cargo.lock` carrying `anyhow 1.0.104` from two sources, which `cargo vendor` cannot represent
(design.md §11). Checked locally first — exit 1, the message naming both sources and both
remedies, and no output directory created. Because it fires in pack, the install phase had
already succeeded, which is what makes this the mid-pipeline case.

The step summary is the one surface that could not be read back from here: GitHub exposes no REST
endpoint for it and the anonymous job page does not carry it, and this token has no web session.
Its content was reconstructed by replaying the released `fail_phase` snippet against the captured
`pipeline.log`, and it names the failed phase, the branch, the artifact, and embeds the log
excerpt — reconstruction, not observation, and the annotation plus the artifact are the observed
parts.

Two findings worth keeping. **`pixi install --frozen` — what the generated install phase runs —
tolerates manifest/lockfile drift by design** (`--frozen` installs "as defined in the lock file,
doesn't update lock file if it isn't up-to-date with the manifest"; `--locked` is the flag that
aborts). The first attempt at this proof added a direct dependency with no `pixi.lock` entry and
the run went **green**, publishing a transport built from the stale lock. So the publisher cannot
catch a consumer's unlocked manifest edit — that is a gap worth a task of its own, not a fix to
guess at here. And because that first run succeeded, the proof branch *did* create
`sandbox-proof/developer-linux-64` (the `branch_prefix` move to `sandbox-proof` is what kept the
healthy snapshot out of its reach); both proof branches were deleted after the evidence was read.
<!-- SECTION:NOTES:END -->

## Final Summary
<!-- SECTION:SUMMARY:BEGIN -->
Generated consumer publish workflows now capture bounded execution logs and expose failure diagnostics outside the Actions log CDN. Each leg records pipeline progress and command-level logs, formatting an actionable summary on `$GITHUB_STEP_SUMMARY` and uploading the diagnostics artifact on completion. Pre-publish failures halt immediately, ensuring existing healthy transport branches remain intact and fetchable. All seven criteria are met: AC#7 closed on a connected failing run (37362418218) where the step error named the `pack` phase, the `always()` upload step ran, the artifact's log proved `doctor` and `publish` were never reached, and `sandbox/developer-linux-64` stayed byte-identical at `01db0c51` across it. The run used the consumer render inside this repository rather than Castellan, because the sandbox's App token can push nowhere else — the deviation and its one hand-edited trigger line are recorded above for review. The proof also turned up that the install phase's `pixi install --frozen` cannot detect manifest/lockfile drift.
<!-- SECTION:SUMMARY:END -->
