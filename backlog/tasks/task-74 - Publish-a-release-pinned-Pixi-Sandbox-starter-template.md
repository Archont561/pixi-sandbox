---
id: TASK-74
title: Publish a release-pinned Pixi Sandbox starter template
status: In Progress
assignee: []
created_date: '2026-10-05'
updated_date: '2026-10-11 18:52'
labels:
  - onboarding
  - template
  - release
  - ci
  - consumer
dependencies:
  - TASK-62
  - TASK-73
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
- [x] #1 A canonical public `pixi-sandbox` starter repository is marked as a GitHub template and contains a language-neutral Pixi development scaffold, locked environment, `pixi-sandbox.toml`, generated consumer publisher, `.gitignore`, and a concise clone/use-template quickstart; it contains no credentials, prebuilt transport, or ignored runtime directories.
- [ ] #2 A repository created from the template needs no manual Pixi Sandbox initialization or connected-host setup: on a clean checkout, the documented Pixi development command succeeds, and a push to the default branch can execute the included publisher with explicitly scoped repository `GITHUB_TOKEN` permissions and no Archont561-owned secret.
- [x] #3 A template-update workflow starts only after `release.yml` has successfully published the exact prefix.dev package set, GitHub Release, standalone assets, and `SHA256SUMS`; it receives an explicit tag/immutable commit hand-off rather than depending on a `release.published` event suppressed by `GITHUB_TOKEN`.
- [x] #4 Before changing the template, the receiver independently verifies that the requested semver tag is a published `Archont561/pixi-sandbox` release, resolves to the handed-off commit, and contains the required release assets/checksum manifest; malformed, missing, unpublished, or mismatched input fails closed.
- [ ] #5 The verified released standalone binary regenerates all managed template setup. The resulting `pixi.toml`/lockfile, config, workflow, and template metadata pin the same exact version; a second run is byte-identical, `init --check` is clean, and no production path resolves `latest`.
- [ ] #6 The template's generated publisher is actionlint-clean and keeps existing consumer safety guarantees: checksum-verified bootstrap, native plan matrix, doctor-before-publish ordering, exact version pinning, and no publish before successful pack/doctor.
- [ ] #7 Cross-repository delivery uses a short-lived owner-operated GitHub App installation token (following TASK-73's proven credential model), scoped only to the source and starter repositories. It never uses a personal token or modifies generated user repositories, forks, or prior template derivatives. The canonical starter main alone is replaced by a lease-protected, atomic single-commit snapshot per ADR-22; immutable release tags are retained.
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
second generator or hand-maintained publisher. Define a small machine-readable version manifest
that names the source tag and commit, then let the checksum-verified released standalone binary
render the same consumer files ordinary users receive. Establish clean-clone and full transport
lifecycle tests before wiring publication.

After TASK-73 supplies the proven owner-operated GitHub App delivery model, add a narrowly scoped
release hand-off after `release.yml` creates the GitHub Release. The starter-side job must verify
the release independently, generate and validate in a disposable checkout, and update only its
managed files atomically. Publish a namespaced immutable starter tag/evidence after the target
repository is updated; retain the preceding template revision on every error. Finally add the
canonical template URL and three-step quickstart to user documentation.
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
<!-- SECTION:NOTES:END -->
