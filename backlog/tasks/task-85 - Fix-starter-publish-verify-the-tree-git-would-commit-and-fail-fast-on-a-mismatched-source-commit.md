---
id: TASK-85
title: >-
  Fix starter publish: verify the tree git would commit, and fail fast on a
  mismatched source commit
status: In Progress
assignee: []
created_date: '2026-10-08 18:40'
updated_date: '2026-10-09 08:03'
labels:
  - starter
  - workflow
  - release
dependencies: []
references:
  - .github/workflows/starter.yml
  - crates/xtask/src/starter.rs
priority: high
type: bug
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The `starter` workflow (`.github/workflows/starter.yml`, TASK-74) fails closed in its latest run, so no starter revision is published.

Evidence. Run 37824048564 (2026-10-08 18:24 UTC, dispatched from main) concluded failure at step 17, "Verify the assembled starter". Steps 1-16 passed, including "Prove a fresh clone runs the documented dev task". Its annotations are: ".pixi must not be committed to the starter (runtime state or credential)" and "1 starter finding(s); the previous starter revision is left unchanged". I could not download the raw job log from the audit sandbox, so the diagnosis rests on those annotations and the code.

Root cause. `verify` in crates/xtask/src/starter.rs (around lines 217-236) checks whether forbidden paths exist in the working directory, not whether git would commit them. Earlier steps run pixi inside starter/ (`pixi lock --manifest-path starter/pixi.toml` and `pixi run --manifest-path starter/pixi.toml dev`), which leaves a starter/.pixi/ directory behind. The step named "Prove a fresh clone runs the documented dev task" does not clone anything, so the verifier inspects the same tree the tooling just wrote into. I reproduced the verifier's finding against a scratch directory containing .pixi/.

Related input problem. Run 37823783846 (18:22 UTC, same day) failed at "Verify the handed-off release" because tag v0.6.0 resolves to 8298edc (chore(release): v0.6.0, 2026-10-06) while the dispatch passed source-commit 469f408 (current main). The check refused correctly, but the mismatch surfaced only after a full job start, and nothing in the workflow tells the operator which commit the tag names.

Intended behaviour. The starter is verified as the tree that would be published, from a clean state, and a mismatched manual dispatch fails with an actionable message before any work starts. The "fresh clone" step does what its name says.

Seam to agree before tests are written: `starter::verify` tested with a directory containing ignored runtime state (.pixi/, .pixi-sandbox/) versus the same state tracked by git, in crates/xtask/tests/ (per the AGENTS.md test conventions, tests live under tests/). Workflow changes are checked by the existing workflow lint gates.

Non-goals: no change to the starter template content, the publication mechanism (branch and immutable tag), or the release-pinning policy. Only dry-run dispatches are allowed while working this task. A real publish pushes to Archont561/pixi-sandbox-starter and needs an explicit go-ahead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The "Prove a fresh clone runs the documented dev task" step runs against a fresh `git clone` (or `git archive`) of the assembled starter in a temporary directory, not against the tree the earlier steps wrote into.
- [x] #2 starter::verify reports a forbidden path (.pixi/, .pixi-sandbox/, .env, SHA256SUMS, pixi-sandbox binaries) only when git would track it; an ignored runtime directory in the working tree is not a finding.
- [x] #3 A regression test in crates/xtask/tests/ fails on the current code for a working tree that contains an ignored .pixi/ and passes after the fix; a second case fails when .pixi/ is actually tracked.
- [x] #4 A manual dispatch whose source-commit does not match the commit the release tag names fails in the first job step, before checkout or any scaffolding, with a message naming both commits.
- [x] #5 The workflow still passes the existing workflow lint gates (pixi run --frozen lint-actions and the check-repository workflow checks) and keeps the single-line run: rule from AGENTS.md.
- [ ] #6 A dry-run starter workflow dispatch with the correct source-commit (8298edce13049ab01a7bc9d3e09c9daf8d14fc08 for v0.6.0) completes the verify step successfully. Recorded as evidence in the task notes; it must not publish.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed on the 2026-10-09 session branch as one commit, test-first: the working-directory false
positive red, then the git-aware rule, then the clone verb, then the dispatch guard, then the
workflow. Everything below is locally provable; AC#6 is not, and is left open on purpose.

- **Root cause, reproduced.** `starter::verify` asked the filesystem whether a forbidden path
  existed. The steps above it (`pixi lock`, and the dev task itself) install an environment into
  `starter/.pixi`, so the lane refused its own success. `runtime_state_the_ignore_file_keeps_out_of
  _the_revision_is_not_a_finding` failed on the pre-fix tree with exactly the three findings the
  failing run's annotations carried (`.pixi`, `.pixi-sandbox`, `SHA256SUMS`).
- **The verdict is now git's.** `verify` consults `ShellGit::ls_publishable` — index plus
  untracked-not-ignored, i.e. the tree `git add -A` would commit — and reports a forbidden path
  only when it is both present and in that answer. `git status --porcelain` was not enough: it is
  silent about tracked-and-unmodified files, so a `.pixi/` an earlier revision already committed
  would have gone unreported. `a_tracked_file_survives_until_its_removal_is_staged`
  (crates/pixi-sandbox-git/tests/worktree_state.rs) pins both halves of that, and
  `a_credential_git_would_take_is_a_finding_even_untracked` pins that the fix is not "skip
  whatever is ignored" — the starter's `.gitignore` does not cover `.env`, and it stays a finding.
- **No work tree, no privilege.** Outside a repository `verify` falls back to the existence check
  and says nothing more, which is what keeps `committed_runtime_state_is_reported` (a scratch
  directory) honest rather than deleted: with no index and no ignore rules to ask, absence cannot
  be proven. `clone_publishable` refuses the same case loudly instead, because a clone cannot be
  assembled from nothing.
- **AC#1 is a fresh clone in the literal sense.** `xtask starter-clone -- --dir starter --out
  "$RUNNER_TEMP/starter-clone"` materialises exactly the `ls_publishable` set into `out/seed`,
  commits it with the tool identity, and returns a `git clone --no-hardlinks` at `out/starter`;
  the dev task then runs against `$RUNNER_TEMP/starter-clone/starter/pixi.toml`. `--no-hardlinks`
  is deliberate: a plain local clone hardlinks the object store, which would make "fresh" a claim
  about arguments rather than about bytes. The one mode bit git itself records is carried across
  (`an_executable_launcher_keeps_its_bit_across_the_clone`), and
  `the_clone_is_a_starter_revision_the_verifier_accepts_without_qualification` proves the tree the
  proof runs in is the tree the verifier accepts.
- **AC#4 is a job, not a step.** The guard runs `xtask starter-check-dispatch` in its own `dispatch`
  job, which `starter` `needs:` — a step placed before the starter job's checkout would still be
  after `actions/checkout` and `setup-pixi`, and after the `pixi run` task graph had to be installed,
  so it would not be "before any work starts" in any sense that costs the lane something. On a
  mismatch it prints `::error::` lines naming both commits and which one to pass, writes the same to
  the job summary, and fails before the starter repository is fetched. On agreement it still reports
  which commit the tag names: the task's complaint was that nothing told the operator that.
  `resolve_tag_commit` is now shared with `verify_release`, and `commit_matches` is one rule for both
  the guard and `release_findings` instead of two spellings of the same prefix comparison.
- **Tests at the seam, not in the module.** New files under `crates/xtask/tests/`
  (`starter_clone.rs`, `starter_cli.rs`) plus the shared starter fixture in `tests/support/mod.rs`,
  which three binaries now need identically; `starter_cli.rs` drives the two new verbs through the
  built binary with a `gh` stub on `PATH`, in the shape `starter_check_main.rs` established.
  `pixi-sandbox`'s `LEGACY` list is untouched and no inline test module was added (D10, invariant 8).

**AC#6 left open, deliberately.** It asks for a `workflow_dispatch` of `.github/workflows/starter.yml`
with `source-commit 8298edce13049ab01a7bc9d3e09c9daf8d14fc08` and the verify step completing on a
real runner. This session was scoped to local proof only, and workflow dispatch from this sandbox's
token has 403'd before (the `Resource not accessible by integration` class); a maintainer's click is
the missing proof, not a missing test. Until that run is green the task stays `In Progress`.

Suite 774 → 802 passing / 1 skipped (xtask 133 → 155, pixi-sandbox-git 21 → 27).
`pixi run --frozen lint` 11 gates green, `xtask check-repository` clean (check 9 included — every
new step is a single `pixi run` line), `lint-actions` on the changed workflow clean.

**AC#6 rehearsed 2026-10-10; the dispatch itself still needs a maintainer's click.** The
dispatch was attempted and refused from this sandbox's token:
`gh workflow run starter.yml --ref main -f release-tag=v0.6.0
-f source-commit=8298edce13049ab01a7bc9d3e09c9daf8d14fc08 -f dry-run=true` returns
`HTTP 403: Resource not accessible by integration`, so the run cannot start here.
Everything the lane checks *before* touching the starter was instead executed directly
against the remote with the flags the workflow passes: `starter-check-dispatch` ("v0.6.0
names commit 8298edce..., and the dispatch agrees"), `starter-verify-release` ("v0.6.0 is
a published release of Archont561/pixi-sandbox at 8298edce...") and `starter-check-main`
("starter Archont561/pixi-sandbox-starter has refs/heads/main at d3b0ce0") all exit 0. The
local steps were rehearsed in a tempdir: `starter-scaffold` writes the three files it owns,
`init --force` adds `pixi-sandbox.toml` and the two workflows, and `init --check` reports
every owned file byte-identical to a fresh render. `starter-verify` then reports exactly one
finding — `missing required file pixi.lock` — the single step that needs network: `pixi lock`
fails here against `https://prefix.dev/conda-forge/linux-64/repodata_shards.msgpack.zst`
with `tls handshake eof`, the airlock boundary this sandbox sits behind. So every gate the
lane runs is either green or provably network-bound, and `dry-run` reaches the publish step
as `starter-publish ... --dry-run`, which does not push. Note `--ref main` is the correct
ref: `starter.yml` does not exist at v0.6.0 (`git cat-file -e
v0.6.0:.github/workflows/starter.yml` fails) while `source-commit` names the v0.6.0 SHA. A
rehearsal is not the dispatch, so AC#6 stays open.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The starter lane now judges the revision instead of the directory: the verifier asks git what would
be committed, so the runtime state the lane's own steps install no longer blocks a publishable
starter, while runtime state that *is* in the index or not covered by an ignore rule is still
refused. The template's development-task proof moved to a fresh `git clone` of exactly that tree,
assembled in a scratch directory, and a dispatch whose `source-commit` the release tag does not name
now fails in its own first job with both commits named and the remedy stated. AC#1-#5 are proven
locally; AC#6 (one successful dry-run dispatch on a real runner) waits for a maintainer's click, so
the task stays In Progress.
<!-- SECTION:SUMMARY:END -->
