---
id: TASK-89
title: Port the consumer proof recipe into xtask subcommands
status: In Progress
updated_date: '2026-10-11 13:30'
assignee: []
created_date: '2026-10-11 11:20'
labels:
  - ci
  - github-actions
  - task-36
dependencies: []
references:
  - .github/workflows/release.yml
  - crates/xtask/src/starter.rs
  - 'backlog/docs/plans/release-lane-repair/doc-13 - Release-Lane-Repair-Proof-Before-Publish-and-a-Starter-That-Publishes.md'
priority: medium
type: chore
ordinal: 89000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The release lane's consumer proof is the last place in the workflows that carries embedded shell. Doc-13's slice 5 moved the proof in front of the publish so a failing proof stops a release instead of annotating one, and it moved the lifecycle steps verbatim: they still spend their lines behind `multiline-run-allowed` opt-outs inside the `consumer-proof` job of `release.yml`.

`starter.yml`'s own header names exactly this as the debt it avoided — "consumer-proof.yml proves the same recipe but spends ~60 lines of `run:` blocks behind `multiline-run-allowed` opt-outs to do it, and per-platform shell is where the v0.3.6 release died" — and task-36's rule is that a step is a pinned `uses:` or a one-line pixi task.

Port the recipe into `xtask consumer-proof-*` subcommands the way `crates/xtask/src/starter.rs` already does for the starter lane: fixture assembly, generate-and-compare, and the pack → doctor → publish → fetch → offline restore lifecycle as tested Rust over tempdir fixtures, leaving the workflow one line per step. The starter lane is the template to copy, including how it keeps the refusal rules in a separately testable pure function.

Deliberately not folded into doc-13. That slice had to move the proof before the publish *without changing what it proves*, and rewriting the recipe in the same commit would have made the move unverifiable — a red gate would not have said which half broke.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [x] #1 Every step of the `consumer-proof` job in `release.yml` is a pinned `uses:` or a one-line pixi task, and no `multiline-run-allowed` opt-out remains in that job.
- [x] #2 The ported recipe is covered by `xtask` tests over tempdir fixtures per D10, and each test is confirmed to fail with the behaviour it checks neutralised, so the tests prove the recipe rather than restating it.
- [ ] #3 A full proof run on a real release reaches the same verdict as the shell recipe it replaces, recorded as evidence here with the run link.
- [x] #4 `check-repository` still reports every workflow `run:` as a single command line, `actionlint` stays clean, and the proof's assertions on the generated publisher stay inside check 13's reach.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1, AC#2 and AC#4 are implemented and locally proven. **AC#3 stays open**: it needs a release
cut *after* this work merges, and a proof run over it. No post-port release exists (the newest
tag is v0.6.1, whose consumer proof ran the shell recipe this replaces), an agent token cannot
dispatch a workflow or push to `main`, and a rehearsal is not the evidence the AC asks for. It
is named here rather than checked.

**What landed, in four commits.**

1. `feat(git)` — `ShellGit::init_bare`, `init_repo_branch` and `checkout_detached`. Invariant 8
   keeps the proof's git journey inside `pixi-sandbox-git`, and `git init --bare` /
   `git checkout --detach FETCH_HEAD` existed nowhere else. `GitProtocol::fetch_into` was
   deliberately *not* reused: it materialises with plumbing and no work tree, so a proof built
   on it would be a proof about this crate's materialisation instead of about a consumer's
   `git`. `init_repo` is untouched — the airlock host depends on its ambient default branch —
   so the named-branch variant is a new method.
2. `feat(xtask)` — `crates/xtask/src/consumer_proof.rs` and five subcommands
   (`consumer-proof-fixture`, `-adopt`, `-version`, `-init`, `-lifecycle`), fronted by the
   existing `xtask` task, so no `pixi.toml` line was added.
3. `refactor(release)` — the `consumer-proof` job rewritten: every step is a pinned `uses:` or
   one `pixi run -e package xtask consumer-proof-<verb>` line, and the job gains the pinned
   `actions/checkout` + `prefix-dev/setup-pixi` pair `starter.yml` uses, because the
   subcommands build with cargo and the shell recipe needed neither the repository nor pixi.
4. `fix(xtask)` — check 13 re-pointed (below).

**The port changes the spelling of the proof, never what it proves.** Three properties of the
shell recipe are load-bearing and are preserved and tested: the two isolation levels
(`pixi lock`/`install`/`pack` keep the runner's `PATH` with an isolated `HOME`; `restore` and
the restored `pixi --version` run under an empty environment whose `PATH=/usr/bin:/bin` *is*
the offline claim); the ten-file evidence set, declared once as `EVIDENCE_FILES` so a step that
quietly stops writing one fails a test instead of shrinking the artifact behind a green run;
and the two publisher assertions, now `PUBLISHER_ASSERTIONS`.

Three deviations, each recorded because a reviewer should not have to find them:

* The shell had **three** isolation levels, not two — `doctor` and `publish` ran with no
  override at all. The port gives them the isolated `HOME`, which removes an ambient
  dependency and so tightens the proof rather than changing it.
* The second publisher render is copied to the evidence *before* the reproducibility
  comparison, where the shell compared and only then copied. A red run now leaves both renders
  to diff. Verdict unchanged.
* The lifecycle's preconditions are named refusals (`lifecycle_refusals`), so a skipped step
  fails at the step that was skipped instead of three tools later. Verdict unchanged.

**AC#2's mutation evidence.** Sixteen behaviours were neutralised one at a time and every one
was caught by a failing test: thirteen in `consumer_proof.rs` (the stale-fixture wipe, the
`ripgrep` dependency, its explanatory comment, dropping `SHA256SUMS` from the assertions,
`empty_env`'s `PATH`, `isolated_env` clearing the environment, the adopt mode bit, the
reproducibility comparison, the publisher assertions, the version step's isolation, the refusal
ladder, `--fetch-tools`, the refusal gate) and three in the git primitives (`--bare`,
`--initial-branch`, `--detach`). Two of those mutations exposed tests that passed while proving
nothing, and both were fixed rather than left:

* Dropping `--detach` was **not** caught at first, because `git checkout FETCH_HEAD` already
  detaches when the revision is not the current branch. `a_detached_checkout_of_a_branch_name_
  leaves_head_off_that_branch` checks out a *branch name*, where the flag is load-bearing.
* Switching the version step from `Isolation::Empty` to `Ambient` was **not** caught, because
  both apply the same `PATH` override. `the_version_check_runs_with_nothing_but_the_home_and_
  the_offline_path` dumps the child's environment instead: under `Empty` it carries only
  `HOME`, `PATH` and the shell's own `PWD`.

**Check 13 was passing vacuously, and this task is what made that visible.** The check read
`.github/workflows/consumer-proof.yml`, but doc-13's slice 5 moved the publisher greps into
`release.yml`, which the check does not read. So `asserted_literals` returned an empty list,
every filter over an empty list passed, and the check reported clean while comparing nothing —
while its fixtures stayed green, because `tests/support/mod.rs` injected a `grep -F` of its
own. The tests exercised the check; the repository never did. The re-point reads
`PUBLISHER_ASSERTIONS` in `consumer_proof.rs` *and* still reads the workflow, so a `grep -F`
added back there cannot escape. And because the failure that shipped was not a wrong assertion
but *no* assertion, a module declaring no `PUBLISHER_ASSERTIONS` is now a finding in its own
right; without that guard the check would go vacuous again the next time the assertions move.
The fixture was changed to model the real tree, which is what exposed the workflow half masking
the module half in the first tests of the re-point.

Non-vacuity is proven on the real tree, not only in fixtures: retiring both occurrences of
`fetch-release --dest` from `crates/pixi-sandbox/src/generated/github_workflow.rs` makes
`xtask check-repository` fail with `the consumer proof asserts the publisher contains
"fetch-release --dest"` and exit 1.

**What is not locally provable, by construction.** The lifecycle's *execution* runs
`pixi lock` and `pixi install --frozen` in the consumer fixture against prefix.dev, which an
airlocked host cannot reach. Its argv, environment shapes, refusals and git choreography are
proven here; running them end to end is exactly what AC#3 asks for.

**Evidence.** `pixi run --frozen test` → 982 passing / 1 skipped (42 git / 219 core /
525 pixi-sandbox / 196 xtask), from a 952/1 baseline. `pixi run --frozen lint` → 11/11.
`pixi run --frozen lint-actions` clean over the rewritten `release.yml`.
`xtask check-repository` green. `convco check` clean over all four commits.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The release lane's consumer proof is tested Rust now. Its ~60 lines of embedded shell and four
`multiline-run-allowed` opt-outs are five `xtask consumer-proof-*` subcommands over
`crates/xtask/src/consumer_proof.rs`, and the `consumer-proof` job is one line per step with no
opt-out marker left in it — the last of them in the workflows. What the proof proves is
unchanged: same fixture, same binary, same isolation levels, same evidence, same lifecycle.
Re-pointing check 13 on the way past turned up that it had been comparing nothing at all since
doc-13 moved the assertions out of the file it reads, which is the second time that check has
caught a proof and a generator disagreeing — this time by first admitting it had stopped
looking. AC#3 is the one open criterion and needs a release this task did not cut.
<!-- SECTION:SUMMARY:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 11:20

2026-10-11 13:30 — implemented in four commits; AC#1/#2/#4 closed, AC#3 carried.

The finding worth keeping out of this file's own history: **check 13 was vacuous.** It read
`.github/workflows/consumer-proof.yml`, doc-13's slice 5 had moved the publisher greps into
`release.yml`, and the check therefore scanned no literals — passing clean while comparing
nothing, with green fixtures, because the fixture injected a `grep -F` of its own. Re-pointed
at `PUBLISHER_ASSERTIONS` (and still reading the workflow), plus a finding for a proof that
declares no assertions at all, since an empty list is the failure mode that actually shipped.

AC#3 needs a release cut after this merges, and the proof run over it. Not checked on a
rehearsal.
<!-- COMMENTS:END -->
