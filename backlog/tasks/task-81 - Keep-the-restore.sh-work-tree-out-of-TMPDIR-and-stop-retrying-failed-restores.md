---
id: TASK-81
title: Keep the restore.sh work tree out of TMPDIR and stop retrying failed restores
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-08 21:55'
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
- [x] #1 With TMPDIR set to an empty directory, scripts/restore.sh creates nothing under it; the branch worktree is created under the restore's work directory inside the output tree.
- [x] #2 The worktree is removed after a successful restore, and kept with its path printed when post-restore verification fails (current behaviour preserved).
- [x] #3 A restore that fails on its first attempt is reported once; the script reruns a second full restore only when the first failure is the flag-compatibility case.
- [x] #4 The pre-restore doctor status is either enforced or logged as informational with a comment explaining why.
- [x] #5 A regression test in tests/restore_script.rs (fixture branch, no network) covers the three behaviours above, was written first, and was observed failing on the current tree.
- [x] #6 pixi run --frozen test does not drop below its last known count, and pixi run --frozen xtask check-repository stays green (Bash 3.2 surface).
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->

Landed on the session branch as two commits: the fix commit carrying the regression tests and
the `scripts/restore.sh` change together (every commit stays green), and this close.

Evidence, all local:

- AC#5 the five new tests in `tests/restore_script.rs` were written first against the fixture
  transport with TMPDIR at an empty directory, and observed failing on the pre-fix tree
  (23 passed; 5 failed — every new test red, and the failing log showed the defect in plain
  sight: `worktree …/home/tmp/sb-…`). After the fix: 28 passed.
- AC#1 `the_branch_worktree_lives_in_the_restore_work_dir_not_tmpdir`: nothing appears under
  TMPDIR; the `→ worktree` line names `<output>/.pixi/.restore-work/sb-<pid>` — the same work
  directory `support::work_dir` gives the binary's own staging (invariant 3) — and a successful
  restore leaves neither worktree nor work directory behind. The directory is created only
  once the branch resolves, so a failed lookup still leaves the output path alone (existing
  test held). A smoke check confirmed `git worktree add` accepts the nested layout the real
  airlock uses (output = the repository root).
- AC#2 `a_post_restore_verification_failure_keeps_the_worktree_and_prints_its_path`: the
  worktree is kept under the work directory as evidence with its path printed in the error,
  and stays out of TMPDIR even then. On success the cleanup mirrors the binary's
  `clean_work_dir`: named entries first, the directory itself only once empty.
- AC#3 `a_failed_restore_is_reported_once_and_never_retried` — the old fallback retried every
  failure with `--path-to-main-repo-code`, an alias of `--output-path` on the same binary
  (`cli.rs`), replaying a restore that had already failed — and
  `a_bootstrap_that_rejects_output_path_gets_one_alias_retry`: clap's "unexpected argument" on
  that exact flag (exit 2) earns exactly one retry with the alias, counted in the fixture
  shim's invocation log. The description's alternative — probing `restore --help` — was weighed
  and would also satisfy AC#3; the usage-error retry was chosen because AC#3 names that shape,
  and a help-text parse can drift from clap's real output.
- AC#4 `a_failing_pre_restore_doctor_is_an_informational_pre_check`: the status is logged as
  informational with a comment explaining why it is not a gate — `restore` re-verifies before
  writing (invariant 1, enforced inside the binary), so its refusal is the enforcement.
- AC#6 suite 767 → 772 passed / 1 skipped (411 in pixi-sandbox, +5), lint 11 gates green,
  `xtask check-repository` green — including check 8, the Bash 3.2 surface (the script's new
  code uses only 3.2-safe constructs: `case`, `[`, `grep -q`; no mapfile/declare -A/&>>).

The retry keeps the old spelling deliberately: `--path-to-main-repo-code` is an alias in
`cli.rs`, so a bootstrap packed before the flag rename still restores.

Baselines for the next session: 772 passed / 1 skipped.

<!-- SECTION:NOTES:END -->
