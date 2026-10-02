---
id: TASK-49
title: Publish releases to the archont561 ecosystem channel
status: In Progress
assignee:
  - '@agent'
created_date: '2026-10-02 21:30'
updated_date: '2026-10-02 21:30'
labels:
  - release
  - conda
  - channels
  - ci
dependencies: []
references:
  - pixi.toml
  - .github/workflows/release.yml
  - crates/xtask/src/repo_checks.rs
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - docs/src/content/docs/installation.mdx
  - README.md
priority: high
type: enhancement
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The owner created the namespace ecosystem channel **`archont561/archont561`** on prefix.dev
(the URL shape issue #71's option 2 proposed) and directed that releases publish there
**rather than** the package-specific `archont561/pixi-sandbox` channel. This moves the
upload target and every surface that installs or names the package channel: the
`publish-conda` pixi task (the actual `pixi upload prefix --channel …`), the airlock proof's
`airlock-install-released`, the generated consumer publisher template plus this repo's own
committed render and the golden fixture, check 5's canonical-install constants and fixtures,
the README badge/one-liner/release description, eight docs pages, the session skill's
bootstrap, and CONTEXT's online-task instruction.

Two boundaries, stated: **D17 is unchanged** — nothing appends any channel to a consumer's
`pixi.toml`; only the *global install command's* URL moves. And per the owner's "rather
than", the old channel is **not** double-published: it freezes at 0.4.3 (pinned installs
keep working; upgrading means the new channel's URL — the migration note in the
installation guide says exactly that).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `publish-conda` uploads to `archont561/archont561`, and release.yml's Repository-Access comment names the new channel
- [x] #2 Every install surface names the ecosystem channel — README one-liner and badge, installation guide (with the ≤ 0.4.3 migration note), quickstart, index, both actions pages, the CI-publishing guide, using-in-your-project, the configuration reference, the session skill's bootstrap, and `airlock-install-released` — with check 5's canonical constants and fixtures moved and `check-repository` green
- [x] #3 The generated publisher template, this repository's committed `publish-sandbox.yml`, and the golden fixture agree byte-for-byte on the new channel (golden-equality test and `lint-generated-workflow` both hold)
- [x] #4 installation.mdx carries no stale claim that init configures channels (one sentence survived task-48's docs sweep and is caught here) and states the migration path: pinned installs from the old channel keep working, upgrading uses the ecosystem channel
- [x] #5 D17's install-channel mention and revisit clause match reality: the ecosystem channel exists, releases publish to it from 0.4.4 on, and the no-append decision stands
- [ ] #6 The next release run publishes to `archont561/archont561` and its proofs pass against it — `package-smoke`, the airlock leg's `airlock-install-released`, and prefix.dev serving the new channel's repodata — or the release is explicitly reported as awaiting the owner's dispatch; this sandbox cannot reach prefix.dev to verify the channel, and Repository Access for `release.yml` on the new channel must be granted by the owner in its prefix.dev settings
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
One mechanical URL migration (`https://prefix.dev/archont561/pixi-sandbox` →
`https://prefix.dev/archont561/archont561`, page form `@archont561/pixi-sandbox` →
`@archont561/archont561`, upload channel `'archont561/pixi-sandbox'` →
`'archont561/archont561'`) across pixi.toml (2 tasks), release.yml (comment),
publish-sandbox.yml + github_workflow.rs + the golden fixture (lockstep), two test
assertions, repo_checks.rs (constants, comments, fixtures), README (badge, one-liner,
release description), eight docs pages, SKILL.md, CONTEXT.md and D17; then the hand edits:
installation.mdx's ecosystem wording + migration note + stale-claim removal, check 5's doc
comments, and D17's revisit clause. Historical records (backlog tasks, v1 evolution plan,
CHANGELOG, CONTEXT's task-22 line, D17's namespace-root evidence) keep the old URL on
purpose — they describe what was true when written.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Implemented on the session branch, riding PR #73 — recreated from #72 so the description could carry both changes; the session token can open and close PRs but cannot edit a body (same branch, same squash
release, so the `fix(init):` headline still drives convco's patch bump to 0.4.4 — the
first release on the ecosystem channel). 21 files, every change a channel-URL move plus
the four hand edits named in the plan. Rejected alternative, recorded: dual-publishing to
both channels during a transition — the owner said "rather than", and the consequence is
accepted and documented (old channel freezes at 0.4.3; the installation guide's migration
note covers pinned installs). Verification is the PR run (this sandbox cannot reach
prefix.dev or run a toolchain): the golden-equality test, `lint-generated-workflow`,
check-repository's check 5, actionlint and the docs build all execute there — `ci (lint ·
test · coverage)` is green on the channel-move commit (run 37066597785).

2026-10-02, transition evidence — the channel must be seeded before merge: the PR's own
airlock proof (run 37066597737, `airlock linux-64`) fails at *Install the released
pixi-sandbox package*, because `airlock-install-released` now names the ecosystem channel,
which holds no package until the first release lands there — and that release needs this
merge. Everything downstream (version proof, pack, both gate tiers) was skipped. Resolution
(recorded): the owner seeds `archont561/archont561` with the five v0.4.3 `.conda` assets
from the v0.4.3 GitHub Release — `pixi upload prefix --channel 'archont561/archont561'
<assets>` with their prefix.dev credentials — before merging, then re-runs the failed
airlock job; seeding also keeps `pixi-sandbox==0.4.3` pins installable from the new
channel. Rejected: merging with the red proof (every PR in the release window stays red),
and a two-phase flip of `airlock-install-released` back to the old channel (a silent
inconsistency window that no consistency check covers).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
In progress in PR #73: the release upload target, every install surface, the generated
publisher, and the repo-consistency constants all name the `archont561/archont561`
ecosystem channel; the installation guide documents the ≤ 0.4.3 migration; D17 is updated
without changing its no-append decision. Open: the owner seeds the new channel with the
v0.4.3 packages (the PR's airlock proof is red at the released-package install until then,
run 37066597737) and grants `release.yml` Repository Access on it; the next release run
then proves the channel end-to-end.
<!-- SECTION:SUMMARY:END -->
