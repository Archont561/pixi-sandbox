---
id: TASK-85
title: >-
  Fix starter publish: verify the tree git would commit, and fail fast on a
  mismatched source commit
status: To Do
assignee: []
created_date: '2026-10-08 18:40'
labels:
  - starter
  - workflow
  - release
dependencies: []
references:
  - .github/workflows/starter.yml
  - crates/xtask/src/starter.rs
priority: high
type: bug
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `starter` workflow (`.github/workflows/starter.yml`, TASK-74) fails closed in its latest run, so no starter revision is published.

Evidence. Run 37824048564 (2026-10-08 18:24 UTC, dispatched from main) concluded failure at step 17, "Verify the assembled starter". Steps 1-16 passed, including "Prove a fresh clone runs the documented dev task". Its annotations are: ".pixi must not be committed to the starter (runtime state or credential)" and "1 starter finding(s); the previous starter revision is left unchanged". I could not download the raw job log from the audit sandbox, so the diagnosis rests on those annotations and the code.

Root cause. `verify` in crates/xtask/src/starter.rs (around lines 217-236) checks whether forbidden paths exist in the working directory, not whether git would commit them. Earlier steps run pixi inside starter/ (`pixi lock --manifest-path starter/pixi.toml` and `pixi run --manifest-path starter/pixi.toml dev`), which leaves a starter/.pixi/ directory behind. The step named "Prove a fresh clone runs the documented dev task" does not clone anything, so the verifier inspects the same tree the tooling just wrote into. I reproduced the verifier's finding against a scratch directory containing .pixi/.

Related input problem. Run 37823783846 (18:22 UTC, same day) failed at "Verify the handed-off release" because tag v0.6.0 resolves to 8298edc (chore(release): v0.6.0, 2026-10-06) while the dispatch passed source-commit 469f408 (current main). The check refused correctly, but the mismatch surfaced only after a full job start, and nothing in the workflow tells the operator which commit the tag names.

Intended behaviour. The starter is verified as the tree that would be published, from a clean state, and a mismatched manual dispatch fails with an actionable message before any work starts. The "fresh clone" step does what its name says.

Seam to agree before tests are written: `starter::verify` tested with a directory containing ignored runtime state (.pixi/, .pixi-sandbox/) versus the same state tracked by git, in crates/xtask/tests/ (per the AGENTS.md test conventions, tests live under tests/). Workflow changes are checked by the existing workflow lint gates.

Non-goals: no change to the starter template content, the publication mechanism (branch and immutable tag), or the release-pinning policy. Only dry-run dispatches are allowed while working this task. A real publish pushes to Archont561/pixi-sandbox-starter and needs an explicit go-ahead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The "Prove a fresh clone runs the documented dev task" step runs against a fresh `git clone` (or `git archive`) of the assembled starter in a temporary directory, not against the tree the earlier steps wrote into.
- [ ] #2 starter::verify reports a forbidden path (.pixi/, .pixi-sandbox/, .env, SHA256SUMS, pixi-sandbox binaries) only when git would track it; an ignored runtime directory in the working tree is not a finding.
- [ ] #3 A regression test in crates/xtask/tests/ fails on the current code for a working tree that contains an ignored .pixi/ and passes after the fix; a second case fails when .pixi/ is actually tracked.
- [ ] #4 A manual dispatch whose source-commit does not match the commit the release tag names fails in the first job step, before checkout or any scaffolding, with a message naming both commits.
- [ ] #5 The workflow still passes the existing workflow lint gates (pixi run --frozen lint-actions and the check-repository workflow checks) and keeps the single-line run: rule from AGENTS.md.
- [ ] #6 A dry-run starter workflow dispatch with the correct source-commit (8298edce13049ab01a7bc9d3e09c9daf8d14fc08 for v0.6.0) completes the verify step successfully. Recorded as evidence in the task notes; it must not publish.
<!-- AC:END -->
