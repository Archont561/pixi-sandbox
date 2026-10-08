---
id: TASK-81
title: Keep the restore.sh work tree out of TMPDIR and stop retrying failed restores
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
labels:
  - restore
  - shell
  - airlock
dependencies: []
priority: high
type: bug
ordinal: 81000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding (whole-repo audit, 2026-10-08). `scripts/restore.sh` checks the whole sandbox branch out into `$TMPDIR/sb-$$` (lines 181-182; `TMPDIR` defaults to `/tmp`) using `git worktree add`. That checkout carries every packed blob: `sandbox/developer-linux-64` holds 814.7 MiB of blobs (measured with `git ls-tree -r -l`), and the restore log shows `worktree /tmp/sb-...` with 10,826 files written. AGENTS.md invariant 3 says never use `/tmp` as a work dir, because a small tmpfs fails mid-restore, and defaults work dirs to `<project>/.pixi/.restore-work`. The script breaks that invariant on the airlock path.

Two control-flow problems sit in the same file. (a) Line 250-253: any failure of the first `restore --output-path` attempt triggers a second full restore with `--path-to-main-repo-code`. That alias is accepted by the same binary (`crates/pixi-sandbox/src/cli.rs:185`), so the retry gains nothing and reruns a failed verification. (b) Line 239: the pre-restore `doctor --verify` status is discarded with `|| true` and no comment.

Intended behaviour: the branch worktree lives under the restore's own work directory inside the output tree (covered by .gitignore), and nothing the restore writes goes under TMPDIR. A failing restore is reported once. The flag-compatibility fallback is chosen by probing the binary, or retried only on clap's usage-error exit, never on any failure.

Seam to agree before tests are written: run `scripts/restore.sh` against a fixture branch from `crates/pixi-sandbox/tests/restore_script.rs` with TMPDIR pointed at an empty directory, and assert that nothing lands under it. Use the fixture transport, not this repository (D10).

Non-goals: no manifest, branch-format, or verification change; no change to the user-tool registration policy. The `--help` header range is cosmetic and out of scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 With TMPDIR set to an empty directory, scripts/restore.sh creates nothing under it; the branch worktree is created under the restore's work directory inside the output tree.
- [ ] #2 The worktree is removed after a successful restore, and kept with its path printed when post-restore verification fails (current behaviour preserved).
- [ ] #3 A restore that fails on its first attempt is reported once; the script reruns a second full restore only when the first failure is the flag-compatibility case.
- [ ] #4 The pre-restore doctor status is either enforced or logged as informational with a comment explaining why.
- [ ] #5 A regression test in tests/restore_script.rs (fixture branch, no network) covers the three behaviours above, was written first, and was observed failing on the current tree.
- [ ] #6 pixi run --frozen test does not drop below its last known count, and pixi run --frozen xtask check-repository stays green (Bash 3.2 surface).
<!-- AC:END -->
