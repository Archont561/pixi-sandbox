---
id: TASK-65
title: 'relock: guard Cargo.lock drift when Cargo vendoring is enabled'
status: Done
updated_date: '2026-10-04 20:31'
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
- [x] #1 When the publish plan enables Cargo vendoring, the generated guard validates Cargo.lock with a non-mutating locked Cargo command before reporting success
- [x] #2 When Cargo vendoring is disabled, the generated workflow adds no Cargo toolchain requirement or Cargo lock check
- [x] #3 Cargo lock drift activates the existing relock repair path, which refreshes Cargo.lock and dispatches the configured CI and publisher workflows only after a real lock commit
- [x] #4 Renderer and fixture tests cover vendoring enabled, disabled, clean, and stale Cargo.lock behavior, and init regeneration remains byte-identical
- [x] #5 The generated workflow remains actionlint-clean and formatting, lint, generated-workflow checks, and the full test suite pass
- [x] #6 A consumer-shaped connected workflow proves stale Cargo.lock is detected and repaired; the run is recorded in this task and issue #92
<!-- AC:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented the local slice test-first at the public generated-workflow seam. The first renderer test failed because a vendoring project's guard contained only `pixi lock --check`; the template now adds `cargo fetch --locked` to that guard while retaining bare `cargo fetch` exclusively in the repair job. Conda-only renders contain neither a Cargo command nor `Cargo.lock`.

A tempdir consumer fixture generates a real clean Cargo.lock under isolated HOME/CARGO_HOME, proves the rendered locked-fetch contract succeeds without changing its bytes, then adds a path dependency and proves the same command fails closed without rewriting the stale lock. The committed `.github/workflows/relock.yml` was regenerated from the template. `xtask lint-generated-workflow` is actionlint-clean; `pixi run --frozen lint` is green; the full suite is 612 passed / 1 skipped (609/1 baseline).

2026-10-04 — AC#6 closed with the connected qgis-rs reproduction already recorded in issue #92 and PR #24. Consumer commit `46ccc613ce1582c949c6d968d2252f62c5dfe5f7` added the locked Cargo guard; run 37151096917 failed `lock guard` on the stale Cargo.lock and its conditional `relock` job succeeded. The bot pushed `56b0b9408cdda66e9b438df3ae7ca6d19898e699` (`chore(lock): refresh lockfiles for #24`); dispatched run 37151144865 then passed the lock guard, and publish run 37151146548 successfully repacked the sandbox. This proves the requested detection-and-repair boundary. The separate absence of a useful PR check rollup on the repaired SHA is intentionally tracked by TASK-66 rather than keeping this Cargo-lock guard task open.
<!-- SECTION:NOTES:END -->

## Final Summary
<!-- SECTION:SUMMARY:BEGIN -->
Vendoring projects now guard both lockfiles before CI or packing: `pixi lock --check` validates the Pixi lock and `cargo fetch --locked` validates Cargo.lock without writing. The existing repair lane remains intentionally mutating (`pixi lock`, then bare `cargo fetch`) and still commits and dispatches only after real changes. Non-vendoring consumers receive no Cargo command. All six criteria are complete. qgis-rs PR #24 supplied the connected proof: the locked Cargo guard detected drift, the relock job committed the repaired Cargo.lock, the follow-up guard passed, and the sandbox publisher repacked successfully. Repaired-head check-rollup UX remains separately scoped to TASK-66.
<!-- SECTION:SUMMARY:END -->
