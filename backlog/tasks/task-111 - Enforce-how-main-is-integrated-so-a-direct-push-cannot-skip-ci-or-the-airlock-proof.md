---
id: TASK-111
title: >-
  Enforce how main is integrated so a direct push cannot skip ci or the airlock
  proof
status: To Do
assignee: []
created_date: '2026-10-13 19:00'
updated_date: '2026-10-13 19:00'
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
- [ ] #1 A reviewed decision records which lever enforces main integration and why the others were rejected, naming the two incidents (the five-commit direct push behind PR #144 and the red `ci` merge of PR #146) and `branches/main/protection` returning `[]` with a 403 to this token. The decision lives in an ADR under `backlog/docs/adr/`, not only in prose.
- [ ] #2 The rule is stated where an agent meets it before pushing: a direct push to `main` is a single commit, so the tip `ci`, codecov and `airlock` reports describe the whole change, and multi-commit work goes through a pull request and squash-merge. Both AGENTS.md and the session skill carry it.
- [ ] #3 A repository-enforceable guard fails on the offending case with a named remediation and passes on the sanctioned cases: a multi-commit direct push to `main` is detected, while a single-commit direct push and a squash-merged pull request both pass.
- [ ] #4 The decision states whether the guard must be a required check to bite, and if so records the exact maintainer action (branch protection requiring `ci`), since this repository cannot apply it itself. It also closes the `airlock` skip: a direct push that would bypass the pull_request-triggered proof for `crates/**` is caught or explicitly accepted with a reason.
- [ ] #5 Evidence is recorded before closing: the guard failing on a multi-commit push and passing on a single-commit push or a PR merge, and (if branch protection is applied) a pull request where the required check blocks a red head.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
