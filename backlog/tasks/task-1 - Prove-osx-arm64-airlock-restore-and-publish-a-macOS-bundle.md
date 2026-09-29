---
id: TASK-1
title: Prove osx-arm64 airlock restore and publish a macOS bundle
status: In Progress
assignee:
  - '@agent'
created_date: '2026-09-28 22:05'
updated_date: '2026-09-29 14:31'
labels:
  - platform
  - sandbox
dependencies: []
references:
  - .pixi-sandbox.toml
  - .knowledge/decisions.md
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
pixi.toml declares osx-arm64 but .pixi-sandbox.toml bundles only linux-64, so macOS developers get no sandbox branch. Decision D11 gates a new platform on a native-runner airlock proof: macOS is reasoned about but not yet verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A native macOS runner restores a packed transport end-to-end offline: doctor --verify passes, restore succeeds, pixi install --frozen --offline is a no-op, cargo check --offline builds against the vendored tree
- [x] #2 .pixi-sandbox.toml declares the developer bundle for osx-arm64 and pixi run lint-sandbox-plan stays green
- [ ] #3 A sandbox/developer-osx-arm64 branch is published and the README platform matrix documents macOS
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Declaring osx-arm64 in .pixi-sandbox.toml is itself the trigger: the pull_request run of airlock.yml plans from that file and fans the matrix onto the native runner, so the PR that adds the platform is the D11 proof. Add the platform, confirm the plan names macos-14 and that the repo-consistency badge check stays green offline, open the PR, then read the macos-14 job. AC1 is the verdict of that job, egress-denied tier plus the D13 restored-tree check. AC2 is the config edit with lint-sandbox-plan green. AC3 follows on merge, since publish-sandbox.yml plans from the same file and will publish sandbox/developer-osx-arm64 by itself. The README platform row now says proof in flight; flip it to proven only if the job is green.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Prerequisites verified offline before opening the proof PR: pixi.lock already solves the default environment for osx-arm64, and the embedded tool pins cover osx-arm64 for pixi, pixi-pack and pixi-unpack, so a macos-14 runner has everything it needs. Added osx-arm64 to the developer bundle; the plan now reports 2 targets and names runner macos-14 with branch sandbox/developer-osx-arm64. scripts/lint-repo-consistency.sh raises no platform or badge disagreement. AC2 checked.

AC1 and AC3 are deliberately left unchecked: they are the verdict of the native macos-14 airlock job, which only GitHub can run. A direct gh workflow run dispatch is not available from the sandbox, HTTP 403 Resource not accessible by integration, so the proof is scheduled through the pull_request trigger instead.
<!-- SECTION:NOTES:END -->
