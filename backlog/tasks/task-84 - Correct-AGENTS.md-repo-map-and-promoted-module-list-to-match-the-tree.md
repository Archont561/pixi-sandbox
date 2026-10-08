---
id: TASK-84
title: Correct AGENTS.md repo map and promoted-module list to match the tree
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
labels:
  - docs
  - agents-md
dependencies: []
priority: low
type: docs
ordinal: 84000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding, documentation drift, found while checking the paths AGENTS.md names. (1) The repo map row at AGENTS.md:48 names crates/pixi-sandbox/src/release.rs, which is now src/release/mod.rs plus src/release/github.rs. (2) The test-conventions paragraph lists the promoted modules as generated, release, self_update and user_tools, but lib.rs (lines 7-12) also exports host_probe and standalone. Other unresolved paths in AGENTS.md are false positives (test-relative paths, pixi feature names, Turborepo's managed block) and are not drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The repo map row names src/release/ with both of its files, and every other path in AGENTS.md's repo map resolves in the tree.
- [ ] #2 The promoted-module list in the test-conventions paragraph matches the pub mod declarations in crates/pixi-sandbox/src/lib.rs exactly.
<!-- AC:END -->
