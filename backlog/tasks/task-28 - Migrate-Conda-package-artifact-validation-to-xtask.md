---
id: TASK-28
title: Migrate Conda package artifact validation to xtask
status: To Do
assignee: []
created_date: '2026-10-01 14:10'
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
- [ ] #1 A pure Rust function validates a caller-supplied artifact root and explicit supported-platform list without reading the repository root
- [ ] #2 Tempdir or fixture tests cover all-five success, missing platform, duplicate package, absent root, and a package placed in the wrong platform directory
- [ ] #3 `xtask check-conda-platforms <dir>` emits actionable platform-specific diagnostics and exits non-zero on every invalid layout
- [ ] #4 The release workflow reaches the checker through a Pixi task or xtask command without inline multi-line shell
- [ ] #5 `scripts/check-conda-platforms.sh` is removed and TASK-23's missing-platform release invariant remains enforced
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Port the directory walk and cardinality policy without copying shell parsing behavior. Drive the pure checker with synthetic artifact trees, add the xtask adapter, update the release workflow/Pixi task, and delete the script after equivalent failures and success are demonstrated.
<!-- SECTION:PLAN:END -->
