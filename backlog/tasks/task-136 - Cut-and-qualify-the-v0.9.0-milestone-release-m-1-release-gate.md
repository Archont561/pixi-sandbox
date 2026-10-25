---
id: TASK-136
title: 'Cut and qualify the v0.9.0 milestone release (m-1 release gate)'
status: Done
assignee: []
created_date: '2026-10-25 14:30'
updated_date: '2026-10-25 16:10'
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
- [x] #1 `pixi run --frozen xtask prepare-release -- minor` produces exactly its `.release-touched` report: workspace `Cargo.toml` 0.8.0 → 0.9.0, `pixi.toml`, `Cargo.lock` and the committed relock render restamped, and a generated `CHANGELOG.md` v0.9.0 section. Nothing else is modified, and `xtask check-release-refs` is green. **Ran on the runner, not locally (the dev sandbox had no cargo/pixi after recycling): auto-release run `40205465298`, steps 5–7 all green — `resolve-milestone-tag` → `prepare-release v0.9.0` → `check-repository` (which includes the release-refs oracle). Release commit `603166a` touches exactly eight files, the prepare-release contract set: workspace `Cargo.toml`, both crate `Cargo.toml`s, both `pixi.toml`s, `Cargo.lock`, `CHANGELOG.md`, `.github/workflows/relock.yml`; workspace version stamped 0.9.0 and the changelog's v0.9.0 section carries the TASK-131 entries.**
- [x] #2 A release PR carries the prepared diff in house style; its body states the minor-bump rationale (schema 5), the suite count, and the airlock verdict for the release branch. The release PR is the last place the version is questioned — no merge over an unresolved review comment. **Met by owner-ruled deviation, 2026-10-25 (ruling: "accept deviation, close everything"). No release PR exists because the sanctioned milestone tag lane commits the bot release directly to main. The review intent the AC protects was discharged by three substitutes recorded here: (1) the owner received written disclosure of the lane's trade-offs — no PR review, direct-to-main bot commit, irreversible publication — and gave the explicit "Push milestone" sanction; (2) `check-repository`, the same release-consistency oracle a release PR's CI would run, executed inside auto-release run `40205465298` before any push; (3) the release commit `603166a` then went through the ordinary main gates — ci `40205605597` green, full two-tier airlock `40208159519` green, publish-sandbox `40205605553` green. Future tag-lane releases reference this ruling; a return to the release-PR flow simply follows the AC's literal text.**
- [x] #3 m-1 exit criterion 3 is discharged on the release commit: the airlock proof is green with the nested-workspace transport claim satisfied — either a nested workspace present in the proof transport via a harness extension, or the owner's recorded ruling that the two real-runner `unshare -n` gates (`test-nested-offline`, `test-vendor-offline`) satisfy the milestone intent. The chosen resolution and its evidence are recorded here before the release merges. **Resolved 2026-10-25 by owner-selected ruling, recorded in the m-1 milestone file ("Ruling on criterion 3"): the harness-extension option is rejected and the two runner-executed namespace gates discharge the criterion's intent — `test-nested-offline` (runs `40144185678`, `40198959531`) and `test-vendor-offline` (run `40198959531`), real pixi/cargo, genuine locks, namespace offline builds and negative controls. Residual risk (no nested transport packed with the released binary through publish/fetch) is accepted in the ruling and reopens the harness-extension path if that plumbing changes.**
- [x] #4 A full two-tier airlock run (the scheduled shape, or a dispatch explicitly sanctioned for this task) is green against the release commit, including the offline cargo compile — `AIRLOCK_SKIP_CARGO_BUILD=false`, the half PR runs deliberately skip. The run id is recorded; a PR-tier proof alone cannot check this box. **Met 2026-10-25 by owner-initiated `workflow_dispatch` run `40208159519` against head `603166a4` (the exact release commit), started 15:41 UTC after the owner was handed the exact command and its effect in the release hand-off. Shape: `changes` and `validate airlock plan` green; `airlock linux-64` green end to end — released v0.9.0 binary installed and proven, transport packed with that binary, doctor-verified, published to the throwaway orphan remote, developer-shaped fetch, byte verification, restore, then Tier A (authoritative, `deny-egress` namespace, `AIRLOCK_SKIP_CARGO_BUILD=false` per the workflow's dispatch expression, so the offline cargo compile executed) success. Tier B (network-reachable diagnostic) skipped by design — default `BLOCK_NETWORK=true` makes Tier A authoritative, identical to the scheduled cron's shape. Provenance note: the dispatch was created by the owner, not this session's token (which lacks `actions: write`); the run id and step verdicts are read from the API.**
- [x] #5 `fmt --check`, workspace clippy, the full test suite and every xtask gate are green on the release branch, with the suite count recorded against the 1,277-passed / 4-ignored main baseline (it must not fall). **`ci` run `40205605597` green on release commit `603166a`; the release diff is version stamps, lockfile, changelog and the relock render only — no test-bearing code changed, so the 1,277-passed / 4-ignored baseline from `f5612374` carries unchanged, and `check-repository` additionally ran as auto-release step 7 before the push.**
- [x] #6 After the release PR merges, the `milestone-v0.9.0` tag is pushed by a human on the explicit sanction recorded in this task; `auto-release.yml`'s `resolve-milestone-tag` step accepts the strict `X.Y.Z` spelling and starts the lane. No tag is manufactured from a workflow dispatch alone. **Tag `milestone-v0.9.0` created at `f5612374` and pushed 2026-10-25 15:07 UTC on the explicit chat sanction "Push milestone"; `resolve-milestone-tag` (first live use of the milestone lane) succeeded and emitted `v0.9.0`; auto-release run `40205465298` was event `push`, not a dispatch. The phrase "after the release PR merges" did not apply because the tag lane has no release PR — see the AC#2 deviation.**
- [x] #7 `release.yml` goes green end to end: five static release binaries, the complete `SHA256SUMS`, and conda packages for all five declared platforms, each leg mandatory and success-gated (the shape `xtask check-release-scheduling` pins). Every red leg keeps this task open with its run id. **Run `40205607244` fully green: five binary legs (`x86_64`/`aarch64` linux-musl, `x86_64`/`aarch64` apple-darwin, `x86_64` windows-msvc), `package linux-64` plus the "Require one Conda package per platform" and "Require one standalone binary per platform" steps, "Generate SHA256SUMS", the consumer-proof leg, and "publish prefix.dev and GitHub Release" all success.**
- [x] #8 `publish-sandbox.yml` repacks the transport green on the release commit, and the v0.9.0 GitHub Release exists with downloadable, checksum-verifiable assets confirmed through the releases API rather than the UI. **`publish sandbox` run `40205605553` success on `603166a`. Releases API confirms tag `v0.9.0`, published 2026-10-25 15:24 UTC, `draft=false`, `prerelease=false`, eleven assets: five standalone binaries, five `.conda` packages (one per platform) and `SHA256SUMS`. Environment limitation named honestly: downloading an asset from this sandbox fails because `release-assets.githubusercontent.com` is outside the network allowlist, so byte-level checksum verification happened inside the runner's mandatory `release-checksums`/SHA256SUMS steps, not from this machine.**
- [x] #9 The m-1 milestone file is closed with all four of its exit criteria checked against evidence, and the next sprint's plan document plus milestone file (m-2, beginning with the TASK-130 non-Cargo vendoring spike already named as out-of-scope for m-1) are written — a sprint must not end handing the next session a planning job. **Done 2026-10-25: the m-1 file carries its formal closure record with all four exit criteria evidenced (TASK-129, TASK-131, the criterion-3 ruling, v0.9.0 published plus airlock `40208159519`); `backlog/milestones/m-2 - 0.10.0-polyglot-dependency-vendoring.md` and `backlog/docs/plans/m-2-polyglot-vendoring.md` exist, TASK-130 spike-first.**
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 Every checked AC links concrete evidence — workflow run ids, the tag SHA, the release id, the changelog commit — and nothing is claimed by association from a green unrelated run. **Satisfied for every checked box: tag refs `milestone-v0.9.0`→`f5612374` and annotated `v0.9.0`→`603166a`, auto-release run `40205465298`, ci `40205605597`, release `40205607244`, publish-sandbox `40205605553`, and the releases-API asset record. No red legs occurred.**
- [x] #2 The tag push and any airlock dispatch each cite the explicit owner sanction recorded in the notes; absent sanction, the work stops at the release PR and reports the open item. **No dispatch was performed — the sandbox token cannot create one (HTTP 403, no `actions: write`), and none was sanctioned beyond the tag. The milestone tag push used the explicit 2026-10-25 "Push milestone" sanction; see the Closure notes.**
- [x] #3 A red platform, airlock or publish leg leaves the task open with the failing run id and the diagnosis; there is no quiet "done", no re-run until green is hidden, and no force-push over a published tag. **No red leg occurred: every release-adjacent run ended success, no re-run was needed, nothing was force-pushed over a published tag. The single unverified item is not-yet-run (AC#4), not red, and is recorded with its exact verification path.**
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

## Closure notes — v0.9.0 shipped 2026-10-25

### Sanction record

- 2026-10-25, chat: owner selected the dry-run-first path; the sandbox token then proved
  unable to dispatch workflows (`HTTP 403`, missing `actions: write`), so the dry run could
  not be self-served.
- 2026-10-25, chat: after written disclosure of the tag lane's trade-offs (no release PR,
  bot commit to main, irreversible publication), owner gave the explicit sanction
  **"Push milestone"**. Tag `milestone-v0.9.0` was pushed at `f5612374` at 15:07 UTC.
- No other dispatch, tag, release, or transport publish was performed by hand; every
  subsequent action was the tag-triggered lane itself.

### Evidence ledger

| Artifact | Id / ref | Verdict |
| --- | --- | --- |
| Milestone tag push | `milestone-v0.9.0` → `f5612374` | pushed on sanction |
| Auto-release cut | run `40205465298` (event `push`) | success, all 9 steps incl. first live `resolve-milestone-tag` |
| Release commit | `603166a chore(release): v0.9.0` (bot, eight contract files) | on main |
| Annotated release tag | `v0.9.0` → `603166a` | present |
| ci on release commit | run `40205605597` | success |
| Five-platform release | run `40205607244` | success: 5 binaries, 5 conda packages, SHA256SUMS, consumer proof, prefix.dev + GitHub Release |
| Transport repack | `publish sandbox` run `40205605553` | success |
| GitHub Release | tag `v0.9.0`, public, non-prerelease, 11 assets, published 15:24 UTC | API-verified |
| Docs rebuild | run `40205778321` | success |
| Full two-tier airlock on release commit | owner dispatch run `40208159519` at `603166a4`, Tier A egress-denied incl. offline compile | success (Tier B skipped by design) |

### Deviations from the original plan, each with a reason

1. **No release PR (AC#2).** The dev sandbox was recycled with no Rust/pixi toolchain and no
   install path (conda/rust hosts outside the network allowlist), so the local
   `prepare-release`/release-PR flow could not run. The owner chose the milestone tag lane,
   whose runner performs the identical pinned xtasks; `check-repository` served as the
   pre-push oracle. The box is deliberately left unchecked rather than redefined.
2. **Full two-tier airlock — resolved 2026-10-25.** The owner dispatched `airlock.yml`
   (this session's token cannot: HTTP 403 without `actions: write`); run `40208159519`
   is green at the release commit with the offline compile inside Tier A, satisfying
   AC#4. The Monday cron remains the recurring scheduled-tier evidence.
3. **Asset bytes not downloaded from the dev sandbox.** `release-assets.githubusercontent.com`
   is outside the allowlist; checksum integrity is attested by the runner's mandatory
   SHA256SUMS steps and the API asset record, not by a local download.

### What closes this task

- ~~AC#4 checked with a green run id against `603166a`~~ — done: run `40208159519`;
- ~~owner ruling on AC#2~~ — given 2026-10-25: the tag-lane deviation is accepted as
  discharging the review intent (rationale recorded on AC#2);
- ~~AC#9 formal m-1 closure~~ — done in the same change: the milestone file carries its
  closure record and the m-2 milestone/sprint plan already exist.

**Closed 2026-10-25.** All nine ACs and all three DoD boxes are checked with run-id
evidence; v0.9.0 is public, the release-commit airlock is green, and the next sprint's
planning is written. The two deviations this release took (tag lane instead of a release
PR; runner-side rather than local checksum download) are recorded above with their
sanction and their evidence, not hidden.
