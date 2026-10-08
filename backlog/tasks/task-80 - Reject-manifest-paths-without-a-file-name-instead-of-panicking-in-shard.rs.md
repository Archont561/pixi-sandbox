---
id: TASK-80
title: Reject manifest paths without a file name instead of panicking in shard.rs
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-08 19:26'
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
- [x] #1 check_rel_path (or a sibling check) rejects paths whose last component is not a normal name, including `.` and `./`, with Error::Invalid naming the path.
- [x] #2 A proptest in crates/pixi-sandbox-core/tests/manifest.rs asserts that every path the validator accepts has a file_name().
- [x] #3 shard.rs no longer calls file_name().expect on manifest-derived paths; any remaining call site on a self-built path carries a comment that justifies it.
- [x] #4 A transport fixture whose blob part path is "." makes doctor --verify and restore exit non-zero with a diagnostic, and never panic (test in crates/pixi-sandbox/tests/ or crates/pixi-sandbox-core/tests/verify.rs).
- [x] #5 The regression tests are written first and observed failing on the current tree.
- [x] #6 The manifest schema is unchanged, and the existing test count does not drop.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed on the 2026-10-08 session branch in three commits: the tests red, then the rule, then the surface tests.

Evidence, all local:

- check_rel_path now rejects any path whose file_name is None with Error::Invalid reading "<what> has no file name: <path>". The rule sits with the other shape rules, so every position it guards is covered by one branch. "a/." stays legal: its components normalise to "a", which has a name.
- shard.rs takes the final component through the new manifest::rel_path_file_name, so assemble, read_blob and split_file return Err instead of panicking. No expect remains in the file.
- Before the fix, join_parts on a part path of "." panicked at shard.rs:234 with "file name"; tests/shard.rs pins that case.
- A real transport whose demo-big part path is "." — every byte still hashing correctly — made doctor --verify report only "verify failed: 1 failure(s)" and no reason, because verify.rs:484 already tolerated a missing file name and the directory read surfaced as an io error. Verify-before-write is why the panic stayed latent rather than unreachable.
- Suite 746 -> 760 passed / 1 skipped; lint 11 gates green; schema unchanged; no push, no PR.

Baselines for the next session: 760 passed / 1 skipped.
<!-- SECTION:NOTES:END -->
