---
id: TASK-44
title: Repack the sandbox transport when a release lands
status: In Progress
assignee:
  - '@agent'
created_date: '2026-10-02 17:30'
updated_date: '2026-10-02 17:45'
labels:
  - ci
  - release
  - transport
dependencies:
  - TASK-42
references:
  - .github/workflows/auto-release.yml
  - .github/workflows/publish-sandbox.yml
  - pixi.toml
priority: high
type: fix
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The v0.4.2 release left the published transport naming the pre-release tree: auto-release
pushes the release commit with `github.token`, which starts no `on: push` workflows, and
`publish-sandbox.yml` only runs on push to main — so `sandbox/developer-linux-64` still
carried commit `dbe57a8` / pixi-sandbox 0.4.1 while main and the release stood at 7745e9c /
0.4.2. Give the release the same explicit dispatch hand-off the docs rebuild already has
(`dispatch-docs`): a one-line `dispatch-sandbox-repack` pixi task dispatched from auto-release
after the release hand-off.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 auto-release dispatches `publish-sandbox.yml` on main after the release hand-off (a `dispatch-sandbox-repack` task in pixi.toml and a one-line step in auto-release.yml mirroring the `dispatch-docs`/`dispatch-release` pattern), and the workflow-shape, actionlint and repo-consistency gates pass
- [ ] #2 the published transport's manifest names the then-current main commit and the released pixi-sandbox version, `linkage: static` (evidence: the publish-sandbox run this change triggers or dispatches)
- [x] #3 the dispatch remedy for the v0.4.2 gap is recorded with its run as evidence, or the gap is explicitly reported as needing a maintainer's click when the token cannot dispatch
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Add `dispatch-sandbox-repack = "gh workflow run publish-sandbox.yml --ref main"` next to
`dispatch-docs`/`dispatch-release` in pixi.toml, and one step in auto-release.yml's `cut` job
after "Dispatch release build for the new tag", carrying `GH_TOKEN` through the step's `env:`
exactly like the existing dispatch steps. No workflow logic beyond the one pixi line (the
workflow step rule); no generated-workflow involvement (the publisher template is consumer
surface, the repository's own auto-release.yml is hand-shaped).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Root cause and remedy. The v0.4.2 release cut at 16:58Z left `sandbox/developer-linux-64` at commit `dbe57a8` / pixi-sandbox 0.4.1 (manifest created 2026-10-02T16:40:42Z, verified by `git show origin/sandbox/developer-linux-64:.pixi-sandbox/manifest.json`): auto-release (run 37036291635) pushes the release commit with `github.token`, which starts no `on: push` workflows, and publish-sandbox.yml only runs on push to main — the same hole `dispatch-docs` already patches for the docs rebuild (pixi.toml documents it at the dispatch tasks). Implemented the same hand-off: `dispatch-sandbox-repack = "gh workflow run publish-sandbox.yml --ref main"` beside `dispatch-release`, dispatched from auto-release.yml's `cut` job right after the release build dispatch, `GH_TOKEN` through the step `env:` like its siblings.

AC#3, the honest form: the session token cannot dispatch (`gh workflow run publish-sandbox.yml --ref main` → `403 Resource not accessible by integration`, the documented limitation in the session skill), so the immediate repack of main@7745e9c needs a maintainer's click in the Actions tab (publish sandbox → Run workflow → main). This change's own merge to main is a push event and repacks the transport from the merge commit regardless — AC#2's evidence is that run's manifest, checked at close-out.
<!-- SECTION:NOTES:END -->
