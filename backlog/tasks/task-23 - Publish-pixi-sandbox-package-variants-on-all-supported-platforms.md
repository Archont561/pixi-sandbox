---
id: TASK-23
title: Publish pixi-sandbox package variants on all supported platforms
status: To Do
assignee: []
created_date: '2026-10-01 09:49'
labels:
  - release
  - platform
  - packaging
milestone: m-0
dependencies: []
references:
  - .knowledge/rust-bootstrap.md
documentation:
  - backlog/docs/plans/v1-platform-workflow-transport/doc-1
priority: high
type: enhancement
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Build and publish Pixi/Conda package variants for linux-64, linux-aarch64, osx-64, osx-arm64, and win-64 using native runners; retain standalone Rust release assets for transport bootstrap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All five package variants are produced from the same release version
- [ ] #2 Each package installs and runs pixi-sandbox on its native runner
- [ ] #3 Standalone binaries retain checksum verification
- [ ] #4 Missing platform artifacts fail the release job
<!-- AC:END -->
