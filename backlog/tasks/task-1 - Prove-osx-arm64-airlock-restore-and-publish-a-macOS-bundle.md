---
id: TASK-1
title: Prove osx-arm64 airlock restore and publish a macOS bundle
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
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
- [ ] #2 .pixi-sandbox.toml declares the developer bundle for osx-arm64 and pixi run lint-sandbox-plan stays green
- [ ] #3 A sandbox/developer-osx-arm64 branch is published and the README platform matrix documents macOS
<!-- AC:END -->
