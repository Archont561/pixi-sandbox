---
id: TASK-28
title: Migrate Conda package artifact validation to xtask
status: Done
assignee:
  - '@agent'
created_date: '2026-10-01 14:10'
updated_date: '2026-10-01 14:05'
labels:
  - rust
  - tooling
  - release
  - packaging
milestone: m-0
dependencies:
  - TASK-27
references:
  - scripts/check-conda-platforms.sh
  - .github/workflows/release.yml
  - pixi.toml
priority: medium
type: enhancement
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Replace the shell-only release artifact gate with a pure Rust checker and an xtask command. The checker must retain the release invariant that all five supported platforms contribute exactly one package in separate artifact directories, while making missing, duplicate, and malformed layouts fixture-testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A pure Rust function validates a caller-supplied artifact root and explicit supported-platform list without reading the repository root
- [x] #2 Tempdir or fixture tests cover all-five success, missing platform, duplicate package, absent root, and a package placed in the wrong platform directory
- [x] #3 `xtask check-conda-platforms <dir>` emits actionable platform-specific diagnostics and exits non-zero on every invalid layout
- [x] #4 The release workflow reaches the checker through a Pixi task or xtask command without inline multi-line shell
- [x] #5 `scripts/check-conda-platforms.sh` is removed and TASK-23's missing-platform release invariant remains enforced
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Port the directory walk and cardinality policy without copying shell parsing behavior. Drive the pure checker with synthetic artifact trees, add the xtask adapter, update the release workflow/Pixi task, and delete the script after equivalent failures and success are demonstrated.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as crates/xtask/src/conda_platforms.rs. validate(dir, platforms) is pure: explicit artifact root, explicit platform list, no repository access; it returns ok-lines plus all diagnostics in one pass so a single run names every broken platform. Tempdir tests cover all-five success, missing platform, duplicate package, absent root, and a .conda outside the conda-<platform> directories (the wrong-directory case is detectable only by location, since all five packages share one filename). `xtask check-conda-platforms <dir>` (default dist/conda) prints GitHub ::error:: annotations and exits non-zero on any invalid layout; release.yml's "Require one Conda package per platform" step is the one-line `cargo run -q -p xtask -- check-conda-platforms dist/conda` (a Rust cache step was added to the publish job since it now compiles xtask). scripts/check-conda-platforms.sh is deleted; the TASK-23 missing-platform invariant is enforced by the same checker plus its fixture tests.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Conda artifact validation is a fixture-tested Rust checker reached as one xtask command from the release workflow; the shell gate is gone and the five-platform invariant is stronger (stray-package detection) than before.
<!-- SECTION:SUMMARY:END -->
