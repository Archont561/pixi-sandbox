---
id: TASK-89
title: Port the consumer proof recipe into xtask subcommands
status: To Do
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
- [ ] #1 Every step of the `consumer-proof` job in `release.yml` is a pinned `uses:` or a one-line pixi task, and no `multiline-run-allowed` opt-out remains in that job.
- [ ] #2 The ported recipe is covered by `xtask` tests over tempdir fixtures per D10, and each test is confirmed to fail with the behaviour it checks neutralised, so the tests prove the recipe rather than restating it.
- [ ] #3 A full proof run on a real release reaches the same verdict as the shell recipe it replaces, recorded as evidence here with the run link.
- [ ] #4 `check-repository` still reports every workflow `run:` as a single command line, `actionlint` stays clean, and the proof's assertions on the generated publisher stay inside check 13's reach.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 11:20
<!-- COMMENTS:END -->
