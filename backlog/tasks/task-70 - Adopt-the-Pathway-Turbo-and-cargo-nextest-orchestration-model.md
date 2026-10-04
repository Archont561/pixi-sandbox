---
id: TASK-70
title: Adopt the Pathway Turbo and cargo-nextest orchestration model
status: Done
assignee:
  - '@agent'
created_date: '2026-10-04 17:50'
updated_date: '2026-10-04 20:11'
labels:
  - tooling
  - turbo
  - rust
dependencies:
  - TASK-26
priority: high
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the split Pixi Cargo and Bun task graph with the proven Archont561/pathway model: one development environment, Pixi as the environment facade, Turbo as the cross-language task graph, Bun as dependency owner, and per-crate cargo-nextest and clippy packages for affected selection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root package.json and bun.lock pin Turbo and expose one repo-wide script per build/check verb; Pixi public test lint fmt coverage and docs commands delegate to that graph without duplicate Cargo flags
- [x] #2 Each Cargo crate is represented by a private Turbo workspace package with dependency edges matching Cargo path dependencies; per-crate test scripts use cargo nextest and include doctests where applicable
- [x] #3 The default Pixi environment carries Bun with Rust and repository tools; the redundant web environment is removed from the publish plan and restored developer workflow; connected PR CI proved the merged lock through successful setup-pixi installations and Linux/Windows jobs
- [x] #4 Turbo inputs invalidate on Cargo manifests Cargo.lock pixi.lock source tests task configuration and relevant repo policy files; mutable and release tasks are never incorrectly cache-skipped
- [x] #5 CI restores the local Turbo cache and runs the same Pixi public commands used locally; remote caching remains disabled
- [x] #6 Repository policy checks exercise the Turbo graph and reject drift or phantom task wiring; fmt lint test docs build and generated-workflow checks pass
- [x] #7 Decision D15 and user/developer documentation are updated with the Pathway evidence and the explicit reversal of the earlier JS-only threshold
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Adopt in vertical, behavior-preserving slices. First make Bun and Turbo available in the single default Pixi environment and freeze the root graph. Then add one private package per Cargo crate with dependency edges matching path dependencies and move test and clippy to per-crate nextest scripts while workspace-only fmt deny coverage and doc commands stay in the aggregate Rust package. Replace duplicate Pixi command bodies with stable facade verbs, add local and CI cache wiring, and verify dry-run graph selection and input invalidation at the approved public seams. Finally update D15 and user and agent docs, run fmt lint test docs and generated workflow checks, and record measurements and any model differences from Pathway.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-04: Implemented from Pathway main at 4d2ebac with the approved full scope. The root Bun workspace pins Turbo 2.11.7 through bun.lock; Pixi test, lint, fmt, coverage and docs are environment facades; four private Rust packages own crate-scoped nextest, doctest and clippy commands; @repo/rust owns workspace fmt, deny and coverage.

The Pixi default lock now contains the former default and web package sets as one environment and the publish plan packs only default. CI installs only default, restores .turbo/cache with OS/ref/commit fallback keys, runs the public Pixi verbs, and uploads crates/lcov.info. Remote caching remains disabled.

Evidence: Turbo dry-run selected exactly four test tasks from @repo/xtask... with no phantom package; pixi run test passed all crate suites and a repeat was FULL TURBO in 35 ms; coverage independently ran 626 passing / 1 skipped; pixi run lint passed 11 graph tasks; docs build produced 13 pages; fmt --check passed. D15 records why Pathway cross-language evidence supersedes the JS-only deferral.

Connected proof completed in PR #99: GitHub Actions run 37223002673 installed with the pinned setup-pixi action and passed the consolidated Linux CI gate (fmt, lint, test, coverage, and docs) plus the Windows replacement test. Lock guard, airlock planning, and the linux-64 airlock job also passed in runs 37223002753 and 37223002708. This closes AC#3 and the task.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Adopted the Pathway Pixi to Bun to Turbo model across Rust and docs. One default environment now carries Bun and Cargo, each Cargo crate has an affected-aware Turbo package using nextest, workspace-only Rust operations stay aggregated, CI persists the local cache, and documented public commands remain Pixi facades. Full gates are green at 626 passing / 1 skipped, with a demonstrated four-task cache hit.
<!-- SECTION:FINAL_SUMMARY:END -->
