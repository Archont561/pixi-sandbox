---
id: TASK-111
title: >-
  Enforce how main is integrated so a direct push cannot skip ci or the airlock
  proof
status: In Progress
assignee: []
created_date: '2026-10-13 19:00'
updated_date: '2026-10-14 11:04'
labels:
  - ci
  - tooling
dependencies: []
references:
  - AGENTS.md
  - .agents/skills/session/SKILL.md
  - .github/workflows/ci.yml
  - crates/xtask
  - 'https://github.com/Archont561/pixi-sandbox/pull/144'
  - 'https://github.com/Archont561/pixi-sandbox/pull/146'
priority: high
type: chore
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
AGENTS.md and the session skill both say pull request, then squash-merge; nothing enforces it. Two consecutive sessions opened on red `main` caused by direct pushes. PR #144 took five commits to main in one push, so `ci` ran once on the tip, `codecov/patch` scored a five-commit window and failed, and the pull_request-triggered `airlock` proof for `crates/**` never ran. PR #146 then merged with a failing `ci` because `ci` is not a required check, leaving `test-payload-roundtrip` red on main. `branches/main/protection` returns `[]` and the protection API is 403 to this token, so branch protection is a maintainer action this sandbox cannot take. Decide and implement the lever: branch protection (maintainer), a repository-enforceable guard, or a documented single-commit rule. The recommendation stays the written rule plus a repository-enforceable guard, because those are the two this repository can enforce on itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A reviewed decision records which lever enforces main integration and why the others were rejected, naming the two incidents (the five-commit direct push behind PR #144 and the red `ci` merge of PR #146) and `branches/main/protection` returning `[]` with a 403 to this token. The decision lives in an ADR under `backlog/docs/adr/`, not only in prose.
- [x] #2 The rule is stated where an agent meets it before pushing: a direct push to `main` is a single commit, so the tip `ci`, codecov and `airlock` reports describe the whole change, and multi-commit work goes through a pull request merge (squash or merge commit). Both AGENTS.md and the session skill carry it.
- [x] #3 A repository-enforceable guard fails on the offending case with a named remediation and passes on the sanctioned cases: a multi-commit direct push to `main` is detected, while a single-commit direct push and a squash-merged pull request both pass.
- [x] #4 The decision states whether the guard must be a required check to bite, and if so records the exact maintainer action (branch protection requiring `ci`), since this repository cannot apply it itself. It also closes the `airlock` skip: a direct push that would bypass the pull_request-triggered proof for `crates/**` is caught or explicitly accepted with a reason.
- [ ] #5 Evidence is recorded before closing: the guard failing on a multi-commit push and passing on a single-commit push or a PR merge, and (if branch protection is applied) a pull request where the required check blocks a red head.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decided in ADR-29 (accepted): main moves by a pull request merged on GitHub, squash or merge commit, on a green ci. Two direct pushes stay allowed: the release workflow's version-stamp commit, and one hand-made commit that touches nothing in the pull_request.paths filter of airlock.yml. AC#2 was reworded from "squash-merge" to "a pull request merge (squash or merge commit)" with the maintainer's approval, because merge-commit merges are sanctioned too.

The guard is `xtask check-main-push <before> <after>` (crates/xtask/src/main_push.rs), the last step of ci on a push to main, on a depth-2 checkout; it runs unless the run was cancelled, so a red step above still gets its verdict. It reads the tip's parents and committer through three new ShellGit read primitives (`commit_facts`, `changed_paths`, `rev_parse`; pinned by crates/pixi-sandbox-git/tests/commit_facts.rs, including a depth-2 clone), and the airlock filter from airlock.yml itself, so the two cannot drift. It fails several commits in one push or a rewrite (NotOneCommit), a merge made in a clone (LocalMerge), and a direct commit on an airlock path (UnprovenAirlockPaths; remedy `gh workflow run airlock.yml --ref main`). It passes branch creation, a push that moved nothing, a GitHub-committed tip (a pull request merged by merge commit or squash), the release commit (bot committer and a `chore(release): vX.Y.Z` subject, both required), and one direct commit outside the airlock paths. An airlock filter it cannot read is an error, never a pass. ci's concurrency group now keys push runs by commit, so a newer push to main never cancels an older push's run. Refactor: the bot identity is defined once, in pixi-sandbox-git (`BOT_NAME`, `BOT_EMAIL`), and the relock template re-exports it; the NUL-path parsing shared by three ShellGit primitives is one helper.

Evidence. Seams agreed before the tests: the ShellGit read primitives against real tempdir repositories, `judge_push` against one throwaway repository per push shape, and the `check-main-push` command black-box. 8 git-fact tests and 15 guard tests (13 shapes, 2 CLI); all 11 hand-made mutants of the verdict rules were killed. Replayed with the real binary against main's history: the push behind #144 (7d539c72 → 13d64ef8) fails NotOneCommit through `pixi run --frozen xtask check-main-push -- <before> <after>` (exit 1, `::error::` remedy), and its three `crates/**` commits fail one at a time; #144, #146, #149 and #150 pass as pull-request merges, v0.6.1 and v0.6.2 as release commits, and the CONTEXT.md direct pushes c3d09beb and ba54fe13 as direct commits. Gates: `pixi run --frozen fmt`; `lint` 11/11 tasks; `test` 1,148 passed / 2 skipped (baseline 1,125 / 2: +8 git, +15 xtask).

Open: AC#5 waits for the guard's first live verdict in ci on main, on this pull request's merge commit. Branch protection is the maintainer action ADR-29 records, because the protection API answers 403 to this token: a ruleset requiring `ci (lint · test · coverage)`, with a bypass for the admin role and for the owner of `RELEASE_PUSH_TOKEN` (a required check also refuses unchecked direct pushes), and "Allow rebase merging" unticked.
<!-- SECTION:NOTES:END -->
