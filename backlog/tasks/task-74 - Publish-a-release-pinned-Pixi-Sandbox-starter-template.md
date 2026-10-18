---
id: TASK-74
title: Publish a release-pinned Pixi Sandbox starter template
status: In Progress
assignee: []
created_date: '2026-10-05'
updated_date: '2026-10-17 23:10'
labels:
  - onboarding
  - template
  - release
  - ci
  - consumer
dependencies:
  - TASK-62
references:
  - >-
    backlog/docs/specifications/release-published-starter-template/doc-10 -
    Release-Published-Pixi-Sandbox-Starter-Template.md
  - .github/workflows/release.yml
  - .github/workflows/consumer-proof.yml
  - >-
    backlog/tasks/task-47 -
    Give-consumers-a-self-updating-binary-and-reviewed-generated-file-upgrade-path.md
  - >-
    backlog/tasks/task-53 -
    Carry-consumer-publish-workflow-policy-in-pixi-sandbox.toml-so-init-output-regenerates-without-hand-edits-issue-79.md
priority: high
type: feature
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
A released pixi-sandbox version is currently proven against an ephemeral consumer fixture, but a
new user must still assemble the same pieces manually on a connected host: create a Pixi project,
run `pixi-sandbox init`, choose the release version, retain generated workflow policy, and verify
that the result is current. Publish a separate GitHub template repository containing a
language-neutral Pixi project plus the generated Pixi Sandbox publisher, and update it only from
a successfully published pixi-sandbox release.

The template is the fast-start connected-development path: use the template (or clone it), run
the documented Pixi command, and push to use the included publisher. It must carry the exact
released version, configuration, lockfile, workflow policy, and checksum-verification contract;
it must not require a user to configure an Archont561 secret, copy a workflow, or run init before
they can start. The detailed release, ownership, validation, and rollback contract is doc-10.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A canonical public `pixi-sandbox` starter repository is marked as a GitHub template and contains a language-neutral Pixi development scaffold, locked environment, `pixi-sandbox.toml`, generated consumer publisher, `.gitignore`, and a concise clone/use-template quickstart; it contains no credentials, prebuilt transport on `main`, or ignored runtime directories.
- [ ] #2 A repository created from the template needs no manual Pixi Sandbox initialization or connected-host setup: on a clean checkout, the documented Pixi development command succeeds, and a push to the default branch can execute the included publisher with explicitly scoped repository `GITHUB_TOKEN` permissions and no Archont561-owned secret.
- [x] #3 A template-update workflow starts only after `release.yml` has successfully published the exact prefix.dev package set, GitHub Release, standalone assets, and `SHA256SUMS`; it receives an explicit tag/immutable commit hand-off rather than depending on a `release.published` event suppressed by `GITHUB_TOKEN`.
- [x] #4 Before changing the template, the receiver independently verifies that the requested semver tag is a published `Archont561/pixi-sandbox` release, resolves to the handed-off commit, and contains the required release assets/checksum manifest; malformed, missing, unpublished, or mismatched input fails closed.
- [x] #5 The verified released standalone binary regenerates all managed template setup. Every file that names a pixi-sandbox version names the same release; a second run is byte-identical, `init --check` is clean, and no production path resolves `latest`. `pixi.toml`, `pixi.lock` and `pixi-sandbox.toml` carry no pixi-sandbox version by design, and there is no separate version manifest.
- [x] #6 The template's generated publisher is actionlint-clean and keeps existing consumer safety guarantees: checksum-verified bootstrap, native plan matrix, doctor-before-publish ordering, exact version pinning, and no publish before successful pack/doctor.
- [ ] #7 Cross-repository delivery uses `PIXI_SANDBOX_STARTER_TOKEN`, a fine-grained token limited to the starter repository (Contents and Workflows read/write, with an expiry), never a personal access token with broader scope or an owner-operated GitHub App. It never modifies generated user repositories, forks, or prior template derivatives. The canonical starter main alone is replaced by a lease-protected, atomic single-commit snapshot per ADR-22; immutable release tags are retained.
- [ ] #8 A failed generation, lock refresh, validation, or delivery leaves the prior starter revision usable and unchanged. The failure emits a bounded GitHub-visible summary/artifact with the tag, source commit, failed phase, and remediation without exposing a token or secret.
- [ ] #9 CI creates a clean temporary consumer from the template and proves locked Pixi install plus the documented development task, byte-identical regeneration, and the fixture/bare-remote pack → doctor → publish → fetch → offline restore lifecycle using the release-pinned binary.
- [ ] #10 Each successful starter update creates immutable evidence (a namespaced starter tag or equivalent release) linking the pixi-sandbox tag, source commit, and template commit. The main release notes and starter README cross-link, and the project documentation advertises the starter only after the connected proof succeeds.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
- [ ] #4 Connected proof records a clean template-derived repository publishing and restoring a release-pinned transport
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Create the starter repository and one manually validated baseline first; do not introduce a
second generator or hand-maintained publisher. The starter tag and source commit are recorded
in the starter commit message, the immutable starter tag and the README; there is no separate
version-manifest file. The checksum-verified released standalone binary renders the same
consumer files ordinary users receive. Establish clean-clone and full transport lifecycle tests
before wiring publication.

Add a narrowly scoped release hand-off after `release.yml` creates the GitHub Release, delivered
with `PIXI_SANDBOX_STARTER_TOKEN` (TASK-73's owner-operated App model is Deprecated and is not a
dependency). The starter-side job must verify the release independently, generate and validate
in a disposable checkout, and update only its managed files atomically. Publish a namespaced
immutable starter tag/evidence after the target repository is updated; retain the preceding
template revision on every error. Finally add the canonical template URL and three-step
quickstart to user documentation.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed 2026-10-06 (commits 02c0684, b359b5f): the starter lane exists end to end and is wired into release.yml.

AC#3 and AC#4 are done. release.yml dispatches `dispatch-starter <tag> <commit>` after the GitHub Release step, alongside the consumer proof and the docs rebuild and for the same reason (a GITHUB_TOKEN release emits no `release: published` event). starter.yml receives the tag plus the immutable commit and re-resolves the tag through `repos/{repo}/commits/{tag}` rather than `targetCommitish`, which can be a branch name. `xtask starter-verify-release` fails closed on a loose tag, a draft, a tag that names a different commit, a missing SHA256SUMS, or a missing standalone asset, each reported separately.

Implementation is tested Rust in crates/xtask/src/starter.rs (19 tests; xtask 98 to 117, workspace 654 to 673), not shell: starter.yml keeps every `run:` to a single command and passes check 9 with no multiline-run-allowed opt-out, where consumer-proof.yml needs three. Subcommands: starter-scaffold, starter-verify, starter-verify-asset, starter-verify-release, starter-publish.

Publication is fail-closed and idempotent: it refuses a remote that is not the canonical starter repository, refuses to overwrite an existing immutable `pixi-sandbox-vX.Y.Z` tag, never force-pushes, and treats an unchanged tree as a success rather than a no-op failure.

Still open. AC#1/#2/#6/#9/#10 are implemented but unproven: Archont561/pixi-sandbox-starter is marked as a GitHub template and is still empty, so nothing is proven until the lane runs once. AC#7 is not done: cross-repository delivery currently falls back to `secrets.PIXI_SANDBOX_STARTER_TOKEN || github.token`, and GITHUB_TOKEN cannot push `.github/workflows/*` (issue #101 is the same restriction in the consumer upgrade lane). The GitHub App credential model TASK-73 is proving is the intended replacement. AC#9 proves locked install, the documented dev task and byte-identical regeneration, but the bare-remote pack/doctor/publish/fetch/restore lifecycle is still consumer-proof.yml exclusively and is not replicated in the starter lane.

Note on the repository name: doc-10 specified Archont561/pixi-sandbox-starter, the repository was created as pixi-sandbox-template on 2026-10-06 and renamed to pixi-sandbox-starter the same day, so the spec and the artifact now agree and no doc change was needed.
2026-10-11 — the publication path was broken, and is now fixed (PR #133, `46efd1a7`, doc-13 §1.1).

The starter lane ran green for v0.6.1 (run 38130262129) and published **one file**. `starter::publish` staged whatever `worktree_status_files` reported, and that query is `git status --porcelain --untracked-files=no`, so over a repository whose only tracked file was `README.md` every generated file was invisible: commit `945ed1a7` holds README.md alone and the immutable tag `pixi-sandbox-v0.6.1` seals it. `starter-verify` could not catch it because it judges the *directory*, which did hold all seven required files on the runner; nothing compared the staged set to the required one. Reproduced end to end with the real `xtask` against a bare-remote fixture (`publishing 1 file(s) for v0.0.1` → pushed tree `README.md`), and the regression test was confirmed to fail with the fix neutralised.

Publish now stages `ls-files --cached --others --exclude-standard` — the revision, not the diff — and refuses a revision missing a required file or carrying a forbidden one. The refusal sits at the last gate before the push, which is where the directory-scoped verdict could not reach.

AC#1 also grew two things the owner asked for: the launcher moved to `scripts/restore.sh` (via `init --script-path`, which already existed and creates the parent directory, so no core change was needed), and a devcontainer pinned to the pixi image the release pins, whose setup installs `git`, `gh` and `pixi-sandbox` from the canonical `archont561/archont561` channel and no compiler toolchain. `REQUIRED_FILES` is now ten entries and names the launcher for the first time — until now a starter could publish without one and pass.

Still unproven, and not provable from here: the starter repository still holds only `README.md`. AC#1/#2/#6/#9/#10 remain open because the corrected lane has not run once, and running it needs two owner actions first — delete the tag `pixi-sandbox-v0.6.1` (it names an empty tree and `publish_refusals` refuses an existing tag), and confirm `PIXI_SANDBOX_STARTER_TOKEN` carries `Workflows: write` on the starter repository, since the push now carries `.github/workflows/*` into another repository (AC#7's concern, and issue #101's exact shape).

2026-10-11 — v0.6.2 starter publication is now proven, superseding the README-only repository observation above. Run https://github.com/Archont561/pixi-sandbox/actions/runs/38163988956 passed dispatch validation and the complete starter job (114542760526): released-binary checksum verification, generation, locking, byte-identical regeneration, actionlint, fresh-clone development task, assembled-tree verification, publication, and evidence upload. The GitHub API confirms Archont561/pixi-sandbox-starter is public and is_template=true. Immutable tag pixi-sandbox-v0.6.2 resolves through annotated tag 7f24bfa0977441cea7c7e8c042614c2bba8d2248 to commit 1655285c27f162f24be33428fb1c1f791bd2e063. Its recursive tree contains all ten required files, including both workflows, pixi.toml, pixi.lock, pixi-sandbox.toml, .gitignore, README, scripts/restore.sh, and both devcontainer files; no runtime directories or transport are tracked. README names release v0.6.2 and source d0fc39ad35d8833b69a937079272db1ce0b9504a and documents use-template, clone, pixi run dev, and push. AC#1 is now checked. Remaining compound criteria are not closed merely because this run is green: no template-derived publisher/restore proof was established (#2/#9), no owner-operated App credential proof (#7), no failure-path evidence (#8), and no complete cross-link/documentation review (#10). Regeneration and lint evidence supports #5/#6, but their full version-resolution and safety contracts remain unchecked in this evidence-only review. TASK-73 is still open. Raw workflow log downloads were egress-blocked; evidence here is job/step conclusions plus the published Git tree and files.

2026-10-11 — owner-approved history policy supersedes the previous never-force-push clause in AC#7: ADR-22 / TASK-95 implement one parentless commit on canonical starter main, lease-protected and atomic with a new immutable release tag. Older tags remain untouched. Consumer repositories and forks are still outside publication scope; the App credential requirement remains open. This is implemented locally and fixture-proven, not yet exercised by a connected starter publication.

2026-10-15 — v0.7.0 starter evidence. Sources: starter run https://github.com/Archont561/pixi-sandbox/actions/runs/38804692103, which published starter revision 66704220209df2294655bc61570a8aa77eda40df as tag pixi-sandbox-v0.7.0, and the starter's own publisher run https://github.com/Archont561/pixi-sandbox-starter/actions/runs/38804893239. The status stays In Progress: TASK-73 is still To Do, and the compound criteria below are not all met.
- **AC#6 checked.**
  - actionlint passed on both generated workflows (step "Lint the generated workflows").
  - The published publisher pins `PIXI_SANDBOX_VERSION: 0.7.0` and installs `pixi-sandbox==0.7.0`.
  - Its bootstrap, `fetch-release --version`, verifies the asset against the release's `SHA256SUMS` and refuses a mismatch (`fetch_release_refuses_a_checksum_mismatch_and_leaves_no_partial_file`).
  - The `plan --json` matrix drove the native leg live.
  - `pipeline` orders install → pack → doctor → publish and stops at the first failed phase (`plan_runs_install_pack_doctor_publish_in_order`, `a_failed_phase_stops_the_pipeline_and_reports_it`).
  - The live publisher run passed end to end.
- **AC#1** is re-confirmed for v0.7.0: public template, ten required files, launcher executable, and no runtime state, credentials or binaries on main. One owner question remains. The canonical repository also holds a `sandbox/developer-linux-64` transport branch written by its own publisher; it was first created by the 2026-10-11 run, after this box was checked. doc-10 says the template must not commit a prebuilt transport branch. ("Use this template" copies only the default branch unless "Include all branches" is chosen.)
- **AC#2 stays open.** The fresh-clone dev task passed, and the canonical starter's push-triggered publisher succeeded with only the repository `GITHUB_TOKEN`. But no repository has yet been created from the template.
- **AC#5 stays open, on wording.** Generation by the released binary and the byte-identical `init --check` both passed; the workflows, launcher and README name 0.7.0, and the publish path never resolves latest. However, `pixi.toml`, `pixi.lock` and `pixi-sandbox.toml` carry no pixi-sandbox version, doc-10's version manifest does not exist, and the scaffold step runs from the source checkout rather than the released binary.
- **AC#7 is half proven.** The ADR-22 half is now proven live (TASK-95 AC#5). The App half is not. The starter publisher run lists `Archont561` as actor and triggering actor, which indicates `PIXI_SANDBOX_STARTER_TOKEN` authenticates as the owner's account; an App installation token would surface as `<app>[bot]`. This is inferred from run metadata; the secret is not readable from here.
- **AC#8 stays open.** Superseded run 38800327950 failed at delivery and left starter main unchanged at c971ef77: the 2026-10-12 scheduled run still ran there, and no push-triggered run occurred until the v0.7.0 publication. It also uploaded its bounded `starter-v0.7.0` artifact. But its failure annotation names no tag, source commit or remediation.
- **AC#9 and DoD#4 stay open.** There is still no template-derived consumer and no fetch → offline-restore lifecycle in the starter lane. consumer-proof is now only the asset-boundary smoke.
- **AC#10 stays open.** The immutable tag names the template commit, whose message records the release and source commit, and the starter README links release v0.7.0 and the source commit. The v0.7.0 release notes do not link the starter.

2026-10-15 — TASK-73 is now Deprecated: the owner-operated GitHub App model is no longer pursued. The App half of AC#7, and the plan step that waits for TASK-73, can no longer be met as written. AC#7 and the TASK-73 dependency are left unchanged until the owner decides how AC#7 should read. One option is to name the credential the starter actually uses, `PIXI_SANDBOX_STARTER_TOKEN`; run metadata suggests it is a personal token, which AC#7 also forbids. The other is to drop the credential half. Until then, the dependency keeps this task shown as blocked. The status stays In Progress.

2026-10-15 — owner decisions applied (Slice 1). AC#7 now names `PIXI_SANDBOX_STARTER_TOKEN`, a
fine-grained token limited to the starter repository (Contents and Workflows read/write, with an
expiry), and the TASK-73 dependency is removed; TASK-73's owner-operated App model is Deprecated.
AC#7 stays unchecked until the configured token's delivery is proven on a starter run. AC#5 is
reworded to where the version actually appears — every file naming a pixi-sandbox version names
the same release, a rerun is identical, `init --check` is clean, and nothing uses `latest`;
`pixi.toml`, `pixi.lock` and `pixi-sandbox.toml` carry no version by design and the never-built
version-manifest file is dropped — and is checked on the v0.7.0 evidence above. doc-10 is updated
to match (credentials, no version manifest, validation items 1–2), and the transport rule is now
that `main` carries no prebuilt transport while the starter keeps its
`sandbox/developer-linux-64` branch. Issue #101 is fixed by TASK-71 (PR #102, shipped in v0.6.0);
closing it from this sandbox was refused with 403 (`Resource not accessible by integration`), so
the closure itself needs an owner click.
2026-10-15 — starter-lane failure reporting implemented test-first (slice 2, owner-sanctioned this
session). AC#8's reporting half and AC#10's release-notes half are implemented and locally
proven; both stay unchecked because neither has been observed live.
- **Credential preflight.** `xtask starter-check-token` (`starter::check_token`) runs in the
  dispatch job before the release is resolved and before the starter job exists, and fails closed
  on an empty `PIXI_SANDBOX_STARTER_TOKEN`, naming the secret, the repository it must be limited to
  and both scopes (`Contents: read and write`, `Workflows: read and write`). The
  `secrets.PIXI_SANDBOX_STARTER_TOKEN || github.token` fallback is gone from all three write sites
  (`starter-check-main`, the starter checkout, `starter-publish`), so a missing secret can no
  longer degrade into a token that can push to neither another repository nor a workflow file —
  the refusal v0.7.0's run 38800327950 reported as a branch problem after the scaffold had run.
- **Failure report.** Every step in `starter.yml` now declares an id, and a trailing
  `if: failure()` step hands the job's own `${{ toJSON(steps) }}` to `xtask
  starter-report-failure`, which writes a bounded summary naming the failed phase, the release tag,
  the source commit and that phase's remediation, and pointing at the `starter-<tag>` artifact the
  job already uploads. `STARTER_PHASES` holds the ids in execution order, one remediation each; two
  tests in `tests/starter.rs` hold the table and the workflow together in both directions (no step
  without an id, no phase without a remediation, the reporter last), the same drift guard check 13
  applies to the consumer proof. The values of `STARTER_TOKEN`, `GH_TOKEN` and `GITHUB_TOKEN` are
  redacted out of the text before it reaches `$GITHUB_STEP_SUMMARY`.
- **Release-notes cross-link (AC#10's notes half).** `release.yml`'s release step now supplies a
  body linking the starter repository and naming the immutable `pixi-sandbox-<tag>` evidence tag
  for the release being cut. GitHub prepends a supplied body to the notes it generates, so
  `generate_release_notes: true` still delivers the changelog. Held by `tests/release_notes.rs`.
- **Evidence.** 15 new tests: xtask 227 → 242, workspace 1,176 → 1,191 passing / 2 skipped.
  `pixi run --frozen fmt`, `pixi run --frozen lint` (11/11 tasks, actionlint included) and
  `pixi run --frozen test` are green on the branch; `pixi run --frozen xtask check-repository` is
  clean with the workflow edits in place.
- **Landed as PR #168** (merge commit `21a238ae`); post-merge `ci`, `docs` and `publish sandbox`
  were green, the last repacking the transport from that commit.
- **Still open, all needing a live run or a release.** AC#7: the configured token's delivery (the
  secret is unreadable from here; AC#7 now names the scopes it must carry). AC#8's live half: a
  failing starter run producing this summary. AC#2/#9 and DoD#4: a repository created from the
  template and its fetch → offline-restore lifecycle. AC#10's live half: the next release's notes.
2026-10-17 — locally verifiable failure-phase ordering tightened. The `STARTER_PHASES` table is the reporter's priority order when more than one step outcome is marked failed; it had `summary` ahead of `evidence`, opposite the actual starter job order. Reordered those entries and added a regression test that compares the ordered remediation IDs against the starter job's workflow IDs. Test-first red confirmed the mismatch; focused starter/report tests, `pixi run --frozen fmt`, `pixi run --frozen lint`, and the full workspace suite pass (1,199 passing / 2 skipped; xtask 243). No AC is newly checked: live failure-summary evidence and all owner/live-run criteria remain open.
<!-- SECTION:NOTES:END -->
