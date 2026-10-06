---
id: TASK-66
title: 'relock: attach an authoritative verdict to the repaired PR head'
status: In Progress
updated_date: '2026-10-06'
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - relock
  - ci
  - github-actions
dependencies:
  - TASK-65
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/92#issuecomment-5974132039'
  - 'https://github.com/Archont561/castellan/pull/8'
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
priority: high
type: enhancement
ordinal: 66000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The issue #92 consumer evidence shows that a successful generated relock leaves reviewers with one visible red guard on the old SHA and no check rollup on the bot-authored repair SHA. The detached workflow_dispatch validations pass, but are not attached to the pull request. Make the generated relock lane publish one authoritative verdict on the repaired head without weakening fork safety or treating an unvalidated repair as green.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A detected drift that is successfully repaired does not leave the relock workflow permanently red solely because its initial guard found repairable drift
- [ ] #2 The bot-authored repaired SHA receives one authoritative PR-visible verdict after lock guard, configured CI, and sandbox publish validation complete
- [x] #3 A failed repair, failed validation, fork pull request, or missing permission fails closed and reports an actionable reason
- [x] #4 The design avoids duplicate publish/relock loops and preserves the existing rule that dispatch occurs only after a real lock commit
- [x] #5 Renderer tests cover clean, repaired, failed, and fork paths; the generated workflow remains actionlint-clean and init regeneration remains byte-identical
- [ ] #6 A consumer pull request proves the repaired head has a usable check rollup; record the PR and run links in this task and issue #92
<!-- AC:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-05 — implemented the local generated-workflow slice test-first. After a real
`pixi-sandbox[bot]` lock commit, the relock job creates an in-progress
`pixi-sandbox relock validation` Check Run directly on `git-auto-commit-action`'s immutable
`commit_hash`; it never treats the stale pull-request SHA as the verdict target. It then verifies
the repaired Pixi/Cargo locks locally, dispatches the configured CI and publisher plus one
input-marked relock validation run, and grants `checks: write` only to the jobs that call the
Checks API.

The input-marked relock run reruns the lock guard on the repaired branch, then uses the pinned
`actions/github-script` v8.0.0 commit (`ed597411d8f924073f98dfc5c65a23a2325f34cd`, resolved
from the upstream tag) to poll the exact CI and publisher `workflow_dispatch` runs for that SHA.
It completes the named Check Run only when both succeed; lock-guard, API/dispatch, downstream,
and one-hour timeout failures complete it as failure with a remediation. The observer checks out
no contributor code, has no contents-write permission, and cannot dispatch another repair, so a
fork remains loudly refused and the validation path cannot loop.

The renderer now carries its own configured relock-workflow path instead of assuming
`relock.yml`, preserving custom `init --relock-workflow-path` output. External renderer tests
cover clean, repaired, failed, and fork contracts; existing fixture coverage holds the locked
Cargo behavior. `pixi run --frozen fmt`, `pixi run --frozen lint` (including actionlint and the
generated-workflow check), and `pixi run --frozen test` passed at 634 passing / 1 skipped.

AC#1, #2, and #6 remain open pending a connected consumer pull request: GitHub must demonstrate
that the final Check Run appears in the repaired head's PR rollup after the dispatched guard, CI,
and publisher all complete.

2026-10-05 — the pinned Checks-API client has moved, so the SHA named above is history rather than
the current pin. Dependabot opened a bump to `actions/github-script` v9.0.0 straight into the
committed `.github/workflows/relock.yml`, which `check-repository` check 10 correctly rejected —
that file is a render, not a source. PR #107 moved the bump into the template's four `uses:` sites
(`crates/pixi-sandbox/src/generated/relock_workflow.rs`) and re-rendered, so the committed
workflow is now the generator's current output at
`3a2844b7e9c422d3c10d287c895573f7108da1b3 # v9.0.0`. The same PR fixed the reason a bump could
never pass CI from this task's side:
`a_repaired_commit_starts_an_authoritative_pr_visible_check_run` asserted the literal v8.0.0 SHA,
so every Dependabot bump turned its pull request red for a reason no renderer edit could fix. It
now asserts the pin's *shape* — 40 lowercase hex characters plus a release label — which is the
property `check-repository` check 4 already enforces over the committed render, so a reviewed bump
needs no test edit and a mutable tag still cannot slip through. The immutability guarantee this
criterion was making is unchanged; only the SHA it named is stale.

2026-10-06 — the observer is now deployed in a real consumer, which removes the last obstacle to
AC#1, #2 and #6 without satisfying any of them. Castellan was bootstrapped onto the 0.6.0
templates by hand (commit 8193c58); its `.github/workflows/relock.yml` went from the 0.5.3 render
to the current one, picking up `checks: write`, the `repaired_sha` dispatch input and the
long-running observer job — 252 added lines, verified in the pushed diff. Until that commit the
proof was not merely unperformed but unreachable: the 0.5.3 render Castellan was running carries
no Check Run code at all, so no pull request against it could ever have produced the rollup this
task is about.

What is still missing is the trigger, not the machinery. `relock.yml` is `pull_request` /
`workflow_dispatch` only, and 8193c58 was a push to main, so no relock run exists for it. These
three criteria need one consumer pull request that carries *repairable lockfile drift* — a
manifest edit without a matching lock refresh — so the guard fires, the bot repairs, and the
observer can attach the verdict to the repaired head. That is cheap to arrange deliberately and
should be done as its own PR rather than waited for, since ordinary Castellan PRs are usually
already locked. Record the PR and run links here and on issue #92 (already closed) when it runs.
<!-- SECTION:NOTES:END -->
