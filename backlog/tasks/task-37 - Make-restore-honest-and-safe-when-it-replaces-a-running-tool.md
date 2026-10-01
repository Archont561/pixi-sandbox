---
id: TASK-37
title: Make restore honest and safe when it replaces a running tool
status: Done
assignee: []
created_date: '2026-10-01 20:05'
updated_date: '2026-10-01 20:40'
labels:
  - restore
  - bug
  - shell
milestone: m-0
dependencies:
  - TASK-33
references:
  - scripts/restore.sh
  - crates/pixi-sandbox-core/src/shard.rs
  - crates/pixi-sandbox-core/tests/shard.rs
documentation:
  - backlog/docs/specifications/transport-and-restore/doc-2
priority: high
type: bug
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two defects found by running `bash scripts/restore.sh` against the published `sandbox/developer-linux-64` branch on a real airlocked machine.

**1. The script claimed a registration that never happened.** Every run ended with "user tools: the restore also registered pixi and pixi-sandbox for new shells". Registration (task-33) is a *restore-side* feature shipped in 0.3.7, and the binary performing the restore comes from the packed branch — the published branch carries 0.3.6, which ignores `PIXI_SANDBOX_USER_TOOLS` entirely (unknown variable, ignored by design, which is exactly why the policy travels as a variable rather than a flag). So nothing was registered, `~/.local/bin` stayed empty, no profile block was written, and the script said otherwise. A developer who believed it opens a new shell with no `pixi` on `PATH`.

**2. A second restore into the same project failed with `Text file busy`.** `shard::materialise` copies an unsplit blob with `fs::copy(src, dst)`, which opens the *destination* for writing; on Linux that is `ETXTBSY` when the destination is a binary the machine is currently executing — and `.pixi/tools/<platform>/pixi` is exactly that while any `pixi run` is in flight. The restore died at `materialise tools`, and the error named the *source* path, so the message pointed at the wrong file. The split-blob path (`join_parts`) never had this problem because it already stages in a sibling and renames.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `scripts/restore.sh` reports the registration that happened — it looks for the managed launcher before claiming one, distinguishes the `skip` policy, and otherwise names the bundled version and says to keep sourcing `sandbox-env.sh`
- [x] #2 The version in that message is read from the restored copy under `.pixi/tools/<platform>/`, not from the branch worktree the script removes a few lines earlier
- [x] #3 `materialise` replaces an unsplit destination through a staged sibling and a rename, so a tool that is currently executing can be replaced and a failed copy leaves no half-written file
- [x] #4 A copy failure names the side that failed: a missing blob names the source, anything else names the file being written
- [x] #5 A test reproduces the real failure (a running ELF binary as the destination) and fails with `ETXTBSY` when the fix is reverted
- [x] #6 The session skill states which world a machine is in — the line the restore printed decides it — and records that today's published branch does not register
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Report evidence instead of intent in the script; stage-and-rename in `materialise` the way `join_parts` already does, with the rename tried first so Unix keeps the atomic path and Windows keeps its unlink fallback.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
`scripts/restore.sh` now greps the managed launcher (`managed by pixi-sandbox`) in `${PIXI_SANDBOX_USER_BIN:-$HOME/.local/bin}/pixi` before it claims anything, branches on the `skip` policy, and otherwise prints the bundled version with the remedy. The version comes from `$OUTPUT/.pixi/tools/$PLATFORM/pixi-sandbox`: the first draft used `$BIN`, which lives in the branch worktree removed a few lines earlier, and printed "unknown version" on a live run.

`shard::materialise`'s unsplit path copies into `temp_sibling(dst)`, verifies the digest there, and hands over to a new `replace_with`, which `join_parts` now shares. `replace_with` renames first and only unlinks as a fallback: on Unix the rename is atomic and swaps the directory entry while the busy inode keeps serving the running process, where the old remove-then-rename opened a window with no tool at all; Windows cannot rename onto an existing file, so the fallback remains for it.

`materialise_replaces_a_tool_that_is_currently_executing` copies `/bin/sleep` (a real ELF — a `#!` script is not a busy text file), spawns it from the destination, and materialises over it; reverting the fix makes it fail with `Os { code: 26, kind: ExecutableFileBusy }`, which is the bug as it was observed. The test returns early where `/bin/sleep` is absent rather than failing.

Suite: 207 passing, 1 skipped (was 206+1). Verified end to end by re-running `scripts/restore.sh` on this machine: the final line now reads "user tools: NOT registered — the bundled pixi-sandbox 0.3.6 predates --user-tools (0.3.7)".
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
A restore no longer tells a developer it put tools on their PATH when the packed binary cannot do that, and no longer dies with `Text file busy` when it replaces a tool the machine is running. Both defects were found by running the real bootstrap against the real published branch, and both are now covered: the message by the branch it reports, the replacement by a test that reproduces `ETXTBSY` without the fix.
<!-- SECTION:SUMMARY:END -->
