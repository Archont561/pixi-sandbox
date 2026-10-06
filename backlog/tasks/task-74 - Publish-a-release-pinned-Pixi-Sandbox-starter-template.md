---
id: TASK-74
title: Publish a release-pinned Pixi Sandbox starter template
status: In Progress
assignee: []
created_date: '2026-10-05'
updated_date: '2026-10-06 20:54'
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
- [ ] #1 A canonical public `pixi-sandbox` starter repository is marked as a GitHub template and contains a language-neutral Pixi development scaffold, locked environment, `pixi-sandbox.toml`, generated consumer publisher, `.gitignore`, and a concise clone/use-template quickstart; it contains no credentials, prebuilt transport, or ignored runtime directories.
- [ ] #2 A repository created from the template needs no manual Pixi Sandbox initialization or connected-host setup: on a clean checkout, the documented Pixi development command succeeds, and a push to the default branch can execute the included publisher with explicitly scoped repository `GITHUB_TOKEN` permissions and no Archont561-owned secret.
- [x] #3 A template-update workflow starts only after `release.yml` has successfully published the exact prefix.dev package set, GitHub Release, standalone assets, and `SHA256SUMS`; it receives an explicit tag/immutable commit hand-off rather than depending on a `release.published` event suppressed by `GITHUB_TOKEN`.
- [x] #4 Before changing the template, the receiver independently verifies that the requested semver tag is a published `Archont561/pixi-sandbox` release, resolves to the handed-off commit, and contains the required release assets/checksum manifest; malformed, missing, unpublished, or mismatched input fails closed.
- [ ] #5 The verified released standalone binary regenerates all managed template setup. The resulting `pixi.toml`/lockfile, config, workflow, and template metadata pin the same exact version; a second run is byte-identical, `init --check` is clean, and no production path resolves `latest`.
- [ ] #6 The template's generated publisher is actionlint-clean and keeps existing consumer safety guarantees: checksum-verified bootstrap, native plan matrix, doctor-before-publish ordering, exact version pinning, and no publish before successful pack/doctor.
- [ ] #7 Cross-repository delivery uses a short-lived owner-operated GitHub App installation token (following TASK-73's proven credential model), scoped only to the source and starter repositories. It never uses a personal token, force-pushes, or modifies generated user repositories, forks, or prior template derivatives.
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
<!-- SECTION:NOTES:END -->
