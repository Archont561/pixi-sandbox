---
id: TASK-39
title: Add a relock bot that pushes refreshed lockfiles onto PR branches
status: To Do
assignee: []
created_date: '2026-10-01 18:36'
updated_date: '2026-10-01 18:40'
labels:
  - ci
  - tooling
milestone: m-0
dependencies: []
references:
  - .github/workflows/relock.yml
  - .github/workflows/ci.yml
  - pixi.toml
  - Cargo.toml
  - CONTEXT.md
documentation:
  - CONTEXT.md
priority: medium
type: chore
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Adding a dependency means editing pixi.toml (or a Cargo.toml) and re-solving the lockfile, and solving needs the network: prefix.dev and the crates.io index are exactly what an airlocked host cannot reach, and pixi add solves before it writes. The manifest edit is the part a human wants to make; the solve is machinery. Decided shape (method A, not the open-its-own-PR shape): hand-edit the manifest, push the branch, and a workflow relocks and pushes the refreshed lockfiles back onto the PR branch as pixi-sandbox[bot] (GitHub's bot convention: the name carries the identity, the email is the github-actions app's noreply address, so the commit renders with the bot avatar), then dispatches ci.yml explicitly because a GITHUB_TOKEN push triggers no workflow. This is the lane that unblocks task-38 (rstest). The scheduled pixi update flavour — move everything to the newest allowed versions rather than only what a new constraint forces — is deliberately out of scope: that one needs a PR of its own.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 .github/workflows/relock.yml exists and obeys the house step rule: every step is a SHA-pinned uses: or a one-line run:, and pixi lock runs as a bare one-liner rather than pixi run <task> — a pixi task would first have to solve the very environment whose lock is stale; triggers on pull_request with paths pixi.toml, Cargo.toml, crates/**/Cargo.toml, plus workflow_dispatch
- [ ] #2 a PR that changes a manifest and leaves the lock unsatisfied gets one commit pushed onto its branch refreshing pixi.lock and Cargo.lock (cargo fetch for the Cargo side), authored as pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com> (the github-actions app's noreply address, so the commit renders with the bot avatar) with a conventional chore(lock) message so convco accepts it when it lands on main; a PR whose lock already satisfies the manifest gets no commit, and a per-PR concurrency group with cancel-in-progress prevents a raced push
- [ ] #3 after pushing, the bot dispatches ci.yml on the PR head (gh workflow run) — the GITHUB_TOKEN push itself triggers nothing, so the explicit dispatch is the only red or green signal the lock commit ever gets
- [ ] #4 on a fork PR the token cannot push to the head branch; the workflow fails loudly with an error naming that limitation rather than silently leaving a stale lock
- [ ] #5 ci.yml gains a pixi lock --check guard so a stale lock fails with a message about the manifest — setup-pixi otherwise fails with a message about installation, verified once by pushing a throwaway branch that edits a manifest without its lock
- [ ] #6 the dependency lane is documented once for contributors: edit the manifest, push, let the bot relock, merge — and a new dependency is still not usable on an airlocked host until a transport that carries it is packed and published
- [ ] #7 pixi run lint and pixi run test are green, and one real PR demonstrates the three paths: lock refreshed and pushed, no-op on an already-satisfied lock, and the explicit ci.yml dispatch
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Graduated from the CONTEXT.md § Session scratchpad proposal (2026-10-01), where the traps were
worked out; the decision recorded there — method A over the open-its-own-PR shape — is the
specification for this task.

`relock.yml`, one job, house-shaped steps:

| step | shape |
| --- | --- |
| checkout | `uses: actions/checkout` @ SHA, `ref: ${{ github.head_ref }}`, `persist-credentials: true`, full `contents: write` job permissions |
| install pixi | `uses: prefix-dev/setup-pixi` @ SHA, `run-install: false` — solving is the job, not a side effect of installing |
| relock (pixi) | one-line `run: pixi lock` — bare, not `pixi run <task>` (AC#1) |
| relock (cargo) | one-line `run: cargo fetch` — only when the PR touches a `Cargo.toml`; minimal lock movement |
| commit + push | `uses: stefanzweifel/git-auto-commit-action` @ SHA with `commit-user-name: pixi-sandbox[bot]`, `commit-user-email: 41898282+github-actions[bot]@users.noreply.github.com` (the github-actions app's noreply address — the same identity transport commits now carry after the `pixi-sandbox[bot]` rename), `commit-message: chore(lock): relock for ${{ github.event.pull_request.title }}` — conventional so convco accepts it on main; the action is a no-op on an empty diff, which is the AC#2 no-commit path |
| dispatch CI | one-line `run: gh workflow run ci.yml --ref ${{ github.head_ref }}` with `GH_TOKEN` — the GITHUB_TOKEN-push suppression trap (AC#3) |

Guard rails: `concurrency: { group: relock-${{ github.head_ref }}, cancel-in-progress: true }`;
the fork-PR failure is the push step failing with `contents: write` unavailable — make the error
name the limitation (AC#4) rather than letting it read as a transient push error.

Then the `pixi lock --check` guard in `ci.yml` (AC#5, one line before the install-consuming
steps) and the lane documentation (AC#6) — README development section is the natural home, one
paragraph, linking the airlock ordering note.

Out of scope, deliberately: the scheduled `pixi update` flavour (needs a PR of its own), and any
change to the *airlock* lane — a merged lock changes nothing on a restored host until a
transport carrying it is packed and published.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
<!-- SECTION:SUMMARY:END -->
