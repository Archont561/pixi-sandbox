---
id: ADR-29
title: "Main is integrated through pull requests, and ci judges how each push arrived"
date: '2026-10-14'
status: accepted
tags:
  - decision
---
## Context

The proofs that guard `main` each judge one commit. `ci` runs on the pushed tip, Codecov's
patch status scores that tip against its parent's report, and the airlock proof (`airlock.yml`)
runs on pull requests only, filtered to the paths it proves. A change is proven only when it
reaches `main` as one judged unit, and twice in a row it did not (TASK-111):

- **The five-commit push behind PR #144.** Five TASK-78 commits (`8832ba10`..`13d64ef8`) reached
  `main` in one direct push. `ci` ran once, on the tip; `codecov/patch` scored the five-commit
  window and failed (79.09%); and the airlock never ran, although three of the five changed
  `crates/**`.
- **The red merge of PR #146.** A pull request merged with its `ci` failing, because `ci` is not
  a required check, and left `test-payload-roundtrip` red on `main`.

Nothing in the repository enforces how `main` is integrated, and the automation cannot add it:
`GET repos/Archont561/pixi-sandbox/branches/main/protection` answers 403 to this token
("Resource not accessible by integration"), `GET .../rules/branches/main` answers `[]`, and all
three merge buttons are enabled.

## Decision

**Main moves by a pull request merged on GitHub, and `ci` judges how every push arrived.**

1. **The rule.** `main` moves by a pull request merged on GitHub — squash or merge commit — whose
   `ci` is green. Two direct pushes stay allowed: the release workflow's version-stamp commit,
   and one hand-made commit that touches nothing the airlock proves (a `CONTEXT.md` entry, a
   documentation fix). `AGENTS.md` and the session skill carry the rule.
2. **The guard.** On a push to `main`, the last step of `ci` runs
   `xtask check-main-push <before> <after>` (`crates/xtask/src/main_push.rs`). It reads what git
   records — the tip's parents and committer, which a depth-2 checkout holds — and passes:
   - a push that created the branch or moved nothing;
   - a tip GitHub committed (`GitHub <noreply@github.com>`), which is what a pull request merged
     by merge commit or squash looks like;
   - the release commit, recognised by its committer (`pixi-sandbox[bot]`, the identity
     `commit-release` commits under) *and* its `chore(release): vX.Y.Z` subject;
   - one direct commit whose changed paths miss the airlock's `on.pull_request.paths` filter,
     which the guard reads from `airlock.yml` itself so the two cannot drift.

   It fails, with an `::error::` annotation that names the remedy and a run-summary line, on:
   - a tip that does not sit directly on the previous tip: several commits in one push, or a
     rewrite of `main`;
   - a merge commit made in a clone;
   - a direct commit that changes an airlock path. The remedy is to prove it now with
     `gh workflow run airlock.yml --ref main` and open a pull request for the next one.

   An airlock filter the guard cannot read is an error, never a pass.
3. **Concurrency.** `ci` groups push runs by commit, so a newer push to `main` never cancels the
   run judging an older one; a pull request's newer push still supersedes its older run.

**Maintainer action — the automation cannot take it.** Branch protection is what refuses a red
pull request; the guard only reports after the fact. In the repository settings:

- Protect `main` with a ruleset that requires the status check `ci (lint · test · coverage)`.
- A required check also refuses a direct push whose commit has not passed it. Give the ruleset a
  bypass for the repository-admin role (the owner's documentation pushes) and for whoever owns
  `RELEASE_PUSH_TOKEN` (the release workflow pushes its version-stamp commit with it). Without
  those bypasses, the sanctioned direct pushes would be refused too.
- Untick "Allow rebase merging". Rebase-merging several commits recreates the #144 shape, and
  the guard fails it.

## Consequences

- The guard runs after the push. It cannot refuse one; it turns `main` red with the remedy. The
  maintainer action above is what makes the rule bite *before* the merge.
- A force push that rewrites `main` fails the guard by design. Recovery is a fix-forward on top,
  never another rewrite.
- **Known limit.** A commit made in GitHub's web editor is committed by GitHub as well, so it
  passes as a pull-request merge. The guard judges what git records, and git records the same
  committer for both.
- Replayed against `main`'s history:
  - the guard fails the push behind #144 (`7d539c72` → `13d64ef8`), and fails its three
    `crates/**` commits even one at a time;
  - it passes #144, #146, #149 and #150, both release commits, and the owner's `CONTEXT.md`
    pushes.
- Each sanctioned shape and each violation is a throwaway repository in
  `crates/xtask/tests/main_push.rs`. The git facts the guard reads are pinned in
  `crates/pixi-sandbox-git/tests/commit_facts.rs`, including the depth-2 clone `ci` makes.

## Alternatives excluded

- **Branch protection alone.** The automation cannot apply it (403). And it cannot tell an
  allowed documentation push from one that changes `crates/**`.
- **The written rule alone.** Both incidents happened while the rule was already written down.
- **A `push` trigger on `airlock.yml`.** It would spend about five minutes of runner time on
  every push to `main`, re-proving what the pull request had already proven for each merged
  change.
- **A pre-push hook.** It is local and opt-in, so it binds only the clones that installed it, and
  neither the release workflow nor the web editor would run it.
