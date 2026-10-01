---
id: TASK-31
title: Migrate release preparation and version-reference maintenance to xtask
status: Done
assignee:
  - '@agent'
created_date: '2026-10-01 14:10'
updated_date: '2026-10-01 14:05'
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
- [x] #1 Rust code reads and cross-checks the workspace, crate dependency, package-manifest, README, and docs version regimes from an explicit fixture root
- [x] #2 Tests cover a consistent tree, stale literal release tags, forbidden docs literals, package/workspace mismatch, invalid requested versions, and idempotent fixes
- [x] #3 `xtask version`, `xtask check-release-refs`, and `xtask prepare-release` expose check/dry-run behavior and actionable file-level diagnostics
- [x] #4 Release preparation composes the tested reference updater, uses atomic writes, and leaves no partial edits when validation fails
- [x] #5 Pixi/release tasks use xtask and `scripts/prepare-release.sh`, `scripts/release-refs.sh`, and `scripts/version.sh` are removed
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Extract the current two-regime version policy into parsed Rust operations over an explicit root. Build check and fix tests from miniature fixture repositories, add command adapters and dry-run output, migrate Pixi and release callers, then delete the three scripts once repeated execution is idempotent.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented as crates/xtask/src/{version,release_refs,prepare_release}.rs. One predicate: release_refs owns "a reference to OURS" (project identity from action.yml's repository default; README family carries literals, docs/src/content must carry none), and both the lint (check-repository) and the fix (prepare-release) call it, so reporter and fixer cannot disagree. `xtask version` reads [workspace.package].version via a real TOML parse; `xtask check-release-refs` is the scan; `xtask prepare-release [selector]` resolves via convco (auto|major|minor|patch|vX.Y.Z, PIXI_SANDBOX_RELEASE honoured), stamps Cargo.toml/pixi.toml/crates/pixi-sandbox/pixi.toml plus the internal path-dependency pins, refreshes Cargo.lock, repins the README family, regenerates CHANGELOG.md, and writes the .release-touched report — tag on stdout only, logs on stderr, so auto-release.yml's `$(pixi run prepare-release … | tail -n1)` contract is unchanged. All writes go through a sibling-tempfile atomic rename; rewrites are byte-faithful via split_inclusive (a missing trailing newline survives, proven by test). Tests cover the version regimes, stale literals, docs literals, package/workspace mismatch, invalid selectors, opt-out, third-party pins staying untouched, and idempotent rewrite/stamp. Verified end-to-end against the working tree: prepare-release v0.9.9 then v0.3.6 round-tripped every manifest and reference cleanly. scripts/prepare-release.sh, scripts/release-refs.sh and scripts/version.sh are deleted; the docs tasks no longer need a version wrapper because docs/astro.config.mjs reads the workspace Cargo.toml itself (verified: docs-build output carries v0.3.6, no __VERSION__ leak).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Release preparation, version discovery and reference maintenance are tested xtask commands sharing one predicate; dry-run reviewability (never commits/tags) and the stdout tag contract are preserved, and the three shell scripts are removed.
<!-- SECTION:SUMMARY:END -->
