---
id: TASK-31
title: Migrate release preparation and version-reference maintenance to xtask
status: To Do
assignee: []
created_date: '2026-10-01 14:10'
updated_date: '2026-10-01 16:30'
labels:
  - rust
  - tooling
  - release
  - versioning
milestone: m-0
dependencies:
  - TASK-27
  - TASK-29
references:
  - scripts/prepare-release.sh
  - scripts/release-refs.sh
  - scripts/version.sh
  - .versionrc
  - Cargo.toml
priority: medium
type: enhancement
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Move release version discovery, reference checking/fixing, and release preparation into tested xtask commands. Establish one Rust implementation for the version/reference predicate so preparation fixes exactly what repository consistency later checks, while retaining dry-run reviewability and refusing partial or invalid release edits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rust code reads and cross-checks the workspace, crate dependency, package-manifest, README, and docs version regimes from an explicit fixture root
- [ ] #2 Tests cover a consistent tree, stale literal release tags, forbidden docs literals, package/workspace mismatch, invalid requested versions, and idempotent fixes
- [ ] #3 `xtask version`, `xtask check-release-refs`, and `xtask prepare-release` expose check/dry-run behavior and actionable file-level diagnostics
- [ ] #4 Release preparation composes the tested reference updater, uses atomic writes, and leaves no partial edits when validation fails
- [ ] #5 Pixi/release tasks use xtask and `scripts/prepare-release.sh`, `scripts/release-refs.sh`, and `scripts/version.sh` are removed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Extract the current two-regime version policy into parsed Rust operations over an explicit root. Build check and fix tests from miniature fixture repositories, add command adapters and dry-run output, migrate Pixi and release callers, then delete the three scripts once repeated execution is idempotent.
<!-- SECTION:PLAN:END -->
