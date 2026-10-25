---
id: TASK-136
title: 'Cut and qualify the v0.9.0 milestone release (m-1 release gate)'
status: To Do
assignee: []
created_date: '2026-10-25 14:30'
labels:
  - release
  - versioning
  - ci
milestone: m-1
dependencies:
  - TASK-129
  - TASK-131
references:
  - backlog/milestones/m-1 - 0.9.0-nested-workspaces-in-one-transport.md
  - backlog/docs/plans/doc-18 - Sprint-Train-and-the-Milestone-Release-Gate.md
  - .github/workflows/auto-release.yml
  - .github/workflows/release.yml
  - .github/workflows/publish-sandbox.yml
  - .github/workflows/airlock.yml
  - crates/xtask/src/main.rs
  - CHANGELOG.md
documentation:
  - >-
    backlog/docs/plans/doc-18 - Sprint-Train-and-the-Milestone-Release-Gate.md
priority: high
type: enhancement
ordinal: 136000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release-gate task doc-18 §5.3 says every milestone needs: without one, "a milestone
closes by assertion". m-1's two build tasks are merged (TASK-129 in PR #209, TASK-131 in
PR #211) and main is green at `823a4b84`, but no task owns the release itself. This task is
that owner: prepare the 0.8.0 → 0.9.0 minor cut, prove the release branch on a native runner
including the full airlock, merge the release PR, push the milestone tag, and watch every
release-adjacent workflow to its verdict. Only then may the m-1 milestone file close.

**Version policy (milestone file).** Pre-1.0: a breaking transport or manifest change is a
minor bump. Schema 5 (`workspace_vendors[]`) is breaking for older readers, so the cut is
`prepare-release -- minor` → v0.9.0. Package versions and transport schema versions are
independent; manifests are not version-bumped. No suffix: a prerelease suffix makes
`release.yml` dry-run the registries instead of publishing.

**Sanction boundary.** Stage D contains human-gated actions. The `milestone-v0.9.0` tag is
human-pushed and any airlock dispatch needs explicit owner sanction; neither happens on
inference. The sanction, when given, is recorded in this task's notes with the date.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `pixi run --frozen xtask prepare-release -- minor` produces exactly its `.release-touched` report: workspace `Cargo.toml` 0.8.0 → 0.9.0, `pixi.toml`, `Cargo.lock` and the committed relock render restamped, and a generated `CHANGELOG.md` v0.9.0 section. Nothing else is modified, and `xtask check-release-refs` is green.
- [ ] #2 A release PR carries the prepared diff in house style; its body states the minor-bump rationale (schema 5), the suite count, and the airlock verdict for the release branch. The release PR is the last place the version is questioned — no merge over an unresolved review comment.
- [x] #3 m-1 exit criterion 3 is discharged on the release commit: the airlock proof is green with the nested-workspace transport claim satisfied — either a nested workspace present in the proof transport via a harness extension, or the owner's recorded ruling that the two real-runner `unshare -n` gates (`test-nested-offline`, `test-vendor-offline`) satisfy the milestone intent. The chosen resolution and its evidence are recorded here before the release merges. **Resolved 2026-10-25 by owner-selected ruling, recorded in the m-1 milestone file ("Ruling on criterion 3"): the harness-extension option is rejected and the two runner-executed namespace gates discharge the criterion's intent — `test-nested-offline` (runs `40144185678`, `40198959531`) and `test-vendor-offline` (run `40198959531`), real pixi/cargo, genuine locks, namespace offline builds and negative controls. Residual risk (no nested transport packed with the released binary through publish/fetch) is accepted in the ruling and reopens the harness-extension path if that plumbing changes.**
- [ ] #4 A full two-tier airlock run (the scheduled shape, or a dispatch explicitly sanctioned for this task) is green against the release commit, including the offline cargo compile — `AIRLOCK_SKIP_CARGO_BUILD=false`, the half PR runs deliberately skip. The run id is recorded; a PR-tier proof alone cannot check this box.
- [ ] #5 `fmt --check`, workspace clippy, the full test suite and every xtask gate are green on the release branch, with the suite count recorded against the 1,277-passed / 4-ignored main baseline (it must not fall).
- [ ] #6 After the release PR merges, the `milestone-v0.9.0` tag is pushed by a human on the explicit sanction recorded in this task; `auto-release.yml`'s `resolve-milestone-tag` step accepts the strict `X.Y.Z` spelling and starts the lane. No tag is manufactured from a workflow dispatch alone.
- [ ] #7 `release.yml` goes green end to end: five static release binaries, the complete `SHA256SUMS`, and conda packages for all five declared platforms, each leg mandatory and success-gated (the shape `xtask check-release-scheduling` pins). Every red leg keeps this task open with its run id.
- [ ] #8 `publish-sandbox.yml` repacks the transport green on the release commit, and the v0.9.0 GitHub Release exists with downloadable, checksum-verifiable assets confirmed through the releases API rather than the UI.
- [ ] #9 The m-1 milestone file is closed with all four of its exit criteria checked against evidence, and the next sprint's plan document plus milestone file (m-2, beginning with the TASK-130 non-Cargo vendoring spike already named as out-of-scope for m-1) are written — a sprint must not end handing the next session a planning job.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 Every checked AC links concrete evidence — workflow run ids, the tag SHA, the release id, the changelog commit — and nothing is claimed by association from a green unrelated run.
- [ ] #2 The tag push and any airlock dispatch each cite the explicit owner sanction recorded in the notes; absent sanction, the work stops at the release PR and reports the open item.
- [ ] #3 A red platform, airlock or publish leg leaves the task open with the failing run id and the diagnosis; there is no quiet "done", no re-run until green is hidden, and no force-push over a published tag.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Resolve and record the m-1 criterion-3 path (harness extension vs. owner ruling — AC#3); close any resulting implementation work first.
2. `prepare-release -- minor`; inspect the touched-file report against the actual diff; local gates.
3. Open the release PR with the version rationale, suite count and airlock evidence plan.
4. With sanction, run the full airlock proof against the release commit; merge when AC#2–#5 hold.
5. With sanction, push `milestone-v0.9.0`; watch `auto-release` → `release.yml` and `publish-sandbox.yml` to verdicts; verify the published assets via the API.
6. Close the m-1 milestone file against its four exit criteria; write the m-2 milestone and sprint plan.
<!-- SECTION:PLAN:END -->
