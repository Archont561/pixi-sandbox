---
id: TASK-80
title: Reject manifest paths without a file name instead of panicking in shard.rs
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
labels:
  - core
  - transport
  - robustness
dependencies: []
priority: medium
type: bug
ordinal: 80000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding. `check_rel_path` (`crates/pixi-sandbox-core/src/manifest.rs:304`) accepts `.` and `./`, but those paths have no `file_name()`. Verified with a standalone rustc probe: `Path::new(".").file_name()` is None and `"."` passes the check, while `"a/.."` and `""` are rejected. `crates/pixi-sandbox-core/src/shard.rs:234` and `:364` then call `file_name().expect("file name")` on `part.path`, which comes from the branch's own manifest.json. A malformed or hostile transport with a part path of `.` therefore panics the restore instead of returning `Error::Invalid`. The branch is untrusted input for an airlock that verifies it byte by byte, and AGENTS.md style forbids unwrap in library code that handles user input. `shard.rs:117` uses the same pattern on a path pack builds itself; it is lower risk and can use the same helper.

Intended behaviour: every path the validator accepts has a usable final component; a manifest that fails this check yields `Error::Invalid` naming the path.

Seam to agree before tests are written: `Manifest::validate` through the public API in `crates/pixi-sandbox-core/tests/manifest.rs` (a proptest over relative paths, plus a named case per rejected shape).

Non-goals: no schema change, and no change to paths that are already valid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 check_rel_path (or a sibling check) rejects paths whose last component is not a normal name, including `.` and `./`, with Error::Invalid naming the path.
- [ ] #2 A proptest in crates/pixi-sandbox-core/tests/manifest.rs asserts that every path the validator accepts has a file_name().
- [ ] #3 shard.rs no longer calls file_name().expect on manifest-derived paths; any remaining call site on a self-built path carries a comment that justifies it.
- [ ] #4 A transport fixture whose blob part path is "." makes doctor --verify and restore exit non-zero with a diagnostic, and never panic (test in crates/pixi-sandbox/tests/ or crates/pixi-sandbox-core/tests/verify.rs).
- [ ] #5 The regression tests are written first and observed failing on the current tree.
- [ ] #6 The manifest schema is unchanged, and the existing test count does not drop.
<!-- AC:END -->
