---
id: TASK-20
title: >-
  Fix release: pixi publish rejects the relative --build-dir the package task
  passes
status: In Progress
assignee:
  - '@arena'
created_date: '2026-10-01 08:33'
updated_date: '2026-10-01 10:25'
labels:
  - release
  - ci
dependencies: []
priority: high
type: bug
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The v0.3.3 release workflow failed at the Build Conda package step (exit 101, 17 s - it panics before compiling): pixi 0.81.0 requires absolute paths and the package task added in the feat(release) prefix.dev commit passes relative ones. Reproduced locally: panicked at crates/pixi_cli/src/publish/mod.rs:618 build dir is not absolute: NotAbsolute(.publish/build). This is the root cause of the stalled v0.3.3 release found in task-14: the tag is pushed but no GitHub Release and no prefix.dev package exist, so the install one-liner still serves v0.3.2 binaries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The package task passes absolute build and target dirs (PIXI_PROJECT_ROOT form, per the design 6 paths-from-environment rule)
- [x] #2 pixi run package completes locally in the restored environment and produces .publish/out/*.conda
- [x] #3 The task output dirs are gitignored so a local run leaves nothing committable
- [ ] #4 A release re-run on a ref containing the fix can complete without the panic (verified by the local run plus the workflow steps that consume dist/*.conda)
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Fix the pixi.toml package task to -absolute paths; gitignore /.publish/ and /dist/; run pixi run package locally to prove the panic is gone and the .conda lands; close with the release re-run instructions
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Root cause confirmed by local reproduction: pixi 0.81.0 panicked at crates/pixi_cli/src/publish/mod.rs:618 (build dir is not absolute: NotAbsolute(.publish/build)) - exit 101 matches the CI step, and the 17 s duration matches a panic before compilation. Fixed by passing $PIXI_PROJECT_ROOT-absolute paths in the package task (same idiom as the docs-dev/docs-build tasks) and gitignoring /.publish/ and /dist/.

Verification: pixi run package now passes argument validation and proceeds into build-backend initialisation - the panic is gone. The run then stops in THIS sandbox at the conda gateway (prefix.dev/conda-forge repodata: tls handshake eof) because the host has the airlock egress profile; an online CI runner does not. So the .conda build itself must be proven by the release re-run, not here.

Watch-out for the re-run: the NEW step after the build (pixi upload prefix --channel archont561/pixi-sandbox with OIDC attestation) requires prefix.dev Repository Access to be configured for the release.yml workflow of this repository - the workflow comment says so explicitly. If that is not configured, the release will get past the build and fail at the prefix.dev upload instead.

AC#2 proven in this environment on 2026-10-01: prefix.dev/conda-forge reachable (HEAD 200) and pixi run package completed end-to-end in ~89s, publishing .publish/out/pixi-sandbox-0.3.5-ha35fb5c_0.conda (3.32 MiB). The earlier TLS handshake eof blocker does not reproduce. AC#4 still needs the tagged release re-run on CI.
<!-- SECTION:NOTES:END -->
