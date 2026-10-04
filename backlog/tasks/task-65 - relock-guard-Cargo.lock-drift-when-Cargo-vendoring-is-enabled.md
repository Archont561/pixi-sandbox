---
id: TASK-65
title: 'relock: guard Cargo.lock drift when Cargo vendoring is enabled'
status: To Do
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - relock
  - cargo
  - generated-workflow
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/92'
  - 'https://github.com/Archont561/qgis-rs/pull/24'
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
  - .github/workflows/relock.yml
priority: high
type: bug
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #92 reports that the generated relock guard validates only pixi.lock. For projects with cargo_vendor = true, a Rust manifest change can therefore leave Cargo.lock stale while the guard passes, and the failure appears later in CI or packing. Render a locked Cargo validation in the guard and keep the existing relock path responsible for refreshing Cargo.lock and triggering the publisher.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When the publish plan enables Cargo vendoring, the generated guard validates Cargo.lock with a non-mutating locked Cargo command before reporting success
- [ ] #2 When Cargo vendoring is disabled, the generated workflow adds no Cargo toolchain requirement or Cargo lock check
- [ ] #3 Cargo lock drift activates the existing relock repair path, which refreshes Cargo.lock and dispatches the configured CI and publisher workflows only after a real lock commit
- [ ] #4 Renderer and fixture tests cover vendoring enabled, disabled, clean, and stale Cargo.lock behavior, and init regeneration remains byte-identical
- [ ] #5 The generated workflow remains actionlint-clean and formatting, lint, generated-workflow checks, and the full test suite pass
- [ ] #6 A consumer-shaped connected workflow proves stale Cargo.lock is detected and repaired; the run is recorded in this task and issue #92
<!-- AC:END -->
