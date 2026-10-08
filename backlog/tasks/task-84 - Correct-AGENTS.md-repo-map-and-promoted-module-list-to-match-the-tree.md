---
id: TASK-84
title: Correct AGENTS.md repo map and promoted-module list to match the tree
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-08 20:05'
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
- [x] #1 The repo map row names src/release/ with both of its files, and every other path in AGENTS.md's repo map resolves in the tree.
- [x] #2 The promoted-module list in the test-conventions paragraph matches the pub mod declarations in crates/pixi-sandbox/src/lib.rs exactly.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed on the 2026-10-08 session branch in two commits: the correction, then the guard that stops it recurring.

- AC#1 the repo map row names `crates/pixi-sandbox/src/release/` and both of its files (`mod.rs`, `github.rs`), with the split explained the way `mod.rs` explains it: `github.rs` holds the only part that needs a socket, which is why the coverage task excludes it by name. Every other root-anchored path in the map resolves; measured at 16 of the 113 backticked spans, the rest being command names, trait names, or paths the sentence anchors to a crate rather than to the root.
- AC#2 the promoted-module list names all six `pub mod`s of `lib.rs` — `generated`, `host_probe`, `release`, `self_update`, `standalone`, `user_tools` — in both places it appears: the `lib.rs` row and the Test conventions paragraph.

Plus one thing the task did not ask for, offered because the drift had no guard: `xtask check-repository` check 12 (`crates/xtask/src/repo_checks/agents_md.rs`) asserts both halves against the tree, checked in both directions so a module promoted without being documented fails too. D10 forbids a test from reading this repository, and `repo_checks/` is the module whose doc comment claims exactly this class of lint. It fires on the pre-fix AGENTS.md with `repo consistency: 2 check(s) failed`, naming `release.rs` and the two missing modules.

Suite 760 -> 767 passed / 1 skipped; lint 11 gates green.
<!-- SECTION:NOTES:END -->
