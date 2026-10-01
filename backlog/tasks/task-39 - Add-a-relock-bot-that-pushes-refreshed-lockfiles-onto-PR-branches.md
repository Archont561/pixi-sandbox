---
id: TASK-39
title: Add a relock bot that pushes refreshed lockfiles onto PR branches
status: Done
assignee: []
created_date: '2026-10-01 18:36'
updated_date: '2026-10-01 21:55'
labels:
  - ci
  - tooling
milestone: m-0
dependencies: []
references:
  - .github/workflows/relock.yml
  - .github/workflows/ci.yml
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/xtask/src/repo_checks.rs
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
- [x] #1 .github/workflows/relock.yml is the committed render of a new pixi-sandbox generator (render_relock_workflow, beside the publisher template that pixi-sandbox init already writes), and check-repository fails when the committed file and a fresh render differ, with a remedy naming the command that re-renders it; the render obeys the house step rule unaided — every step is a SHA-pinned uses: or a one-line run:, with pixi lock as a bare one-liner rather than pixi run <task> (a pixi task would first have to solve the very environment whose lock is stale) — so unlike the publisher it needs no multiline-run-allowed exemption; it stamps the pixi version from the embedded tools lock, so the bot cannot write a lockfile the pixi a transport carries is unable to read; it triggers on pull_request and workflow_dispatch, and the relock job is gated on the guard job's verdict rather than on a paths list, so a stale lock arriving by any route is caught
- [x] #2 a PR that changes a manifest and leaves the lock unsatisfied gets one commit pushed onto its branch refreshing pixi.lock and Cargo.lock (cargo fetch for the Cargo side), authored as pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com> (the github-actions app's noreply address, so the commit renders with the bot avatar) with a conventional chore(lock) message so convco accepts it when it lands on main; a PR whose lock already satisfies the manifest gets no commit, and a per-PR concurrency group with cancel-in-progress prevents a raced push
- [x] #3 after pushing, the bot dispatches ci.yml on the PR head (gh workflow run) — the GITHUB_TOKEN push itself triggers nothing, so the explicit dispatch is the only red or green signal the lock commit ever gets
- [x] #4 on a fork PR the token cannot push to the head branch; the workflow fails loudly with an error naming that limitation rather than silently leaving a stale lock
- [x] #5 the lock guard is a job of its own in relock.yml, running pixi lock --check on every pull request behind a setup-pixi with run-install: false, so it reports before any environment is installed and a stale lock fails with a message about the manifest rather than setup-pixi's message about installation; being a separate job is what buys that ordering, so ci.yml itself stays unchanged; verified once by pushing a throwaway branch that edits a manifest without its lock
- [x] #6 the dependency lane is documented once for contributors: edit the manifest, push, let the bot relock, merge — and a new dependency is still not usable on an airlocked host until a transport that carries it is packed and published
- [x] #7 pixi run lint and pixi run test are green, and one real PR demonstrates the three paths: lock refreshed and pushed, no-op on an already-satisfied lock, and the explicit ci.yml dispatch
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
Built as a *generated* artifact rather than a hand-written workflow, at the owner's direction:
`render_relock_workflow` sits beside the publisher template, `pixi-sandbox init` writes it
(`--relock-workflow-path`, `--relock-ci-workflow`), and this repository commits its own render,
held byte-equal by `check-repository` check 10 with `xtask render-relock` as the only way to
change it. The blocker that made this cheap: unlike the publisher — whose bootstrap download and
pack loop are multi-line shell by necessity, and which therefore still cannot be committed until
check 9 learns a generated-file exemption — the relock lane is one-line steps throughout, so the
render satisfies check 9 unaided.

Two values come from the tree instead of from flags. The pixi version is stamped from the
embedded tools lock (0.81.0), because the bot must not write a lockfile the pixi *inside a
transport* cannot read and a restored host cannot upgrade to catch up; and the `cargo fetch`
half plus `file_pattern` follow the reviewed publish plan, so a conda-only consumer gets neither
a cargo step nor a remedy message mentioning cargo.

Deviations from the plan table, all deliberate. The action's inputs are underscored
(`commit_message`, `commit_user_name`, `commit_user_email`, `file_pattern`) and `commit_author`
had to be set as well — name and email only set the committer, while AC#2 asks for authorship.
The `paths:` filter was dropped: the guard job's verdict is the trigger (`needs.guard.result ==
'failure'`), so drift arriving by any route is caught rather than only drift under a configured
path. The branch name reaches `gh workflow run` through a `HEAD_REF` env var, because actionlint
rejects inline `${{ github.head_ref }}` in a run as script injection. And the bot dispatches two
workflows, not one: `ci.yml`, and `relock.yml` itself, so the guard reports green on the very
commit that fixed the drift (the dispatched run cannot loop — the relock job also requires
`github.event_name == 'pull_request'`).

One bug found by the live demonstration, worth keeping: a job-level `permissions:` block is a
*replacement*, not an addition. With `contents: write` alone the push succeeded and then
`gh workflow run` exited 1 with no `actions: write` (run 36927489932) — the worst possible
ordering, since the lock commit lands and its dispatch does not. Both scopes are now named in
the template and asserted by a renderer test.

AC#7 evidence, all on real runners in PR #52: **no-op path** — run 36926500261, guard pass in
5s, relock `skipping`, and again on the cleaned-up branch (36928161711). **Refresh-and-push
path** — run 36927874085, guard red in 5s on the deliberately stale lock, relock green in 32s,
commit `chore(lock): refresh lockfiles for #52` authored *and* committed as
`pixi-sandbox[bot] <41898282+github-actions[bot]@users.noreply.github.com>`, touching exactly
pixi.lock (+65 lines for the conda dependency) and Cargo.lock (+1 for the Rust dev-dependency
edge). **Dispatch path** — ci.yml run 36927951143 (`workflow_dispatch`, triggering actor
`github-actions[bot]`) green in 1m15s, and relock.yml run 36927954638 green on the bot's commit.
The demonstration edits were reverted before merge, so main's manifests and lockfiles are
untouched.

Two corroborations fell out of the same runs. The bot's push produced a `pull_request` run stuck
in `action_required` and never executed (36927958169) — the GITHUB_TOKEN suppression this design
exists to work around, observed rather than assumed. And on the stale-lock commit the guard
failed in 5s with a message about the manifest while `ci` failed in 22s inside `setup-pixi`
(run 36927873842) — exactly the installation-flavoured failure AC#5 exists to pre-empt, side by
side with the manifest-flavoured one.

AC#4 is proven by construction, not by a live fork pull request: this repository has no fork to
open one from. What *was* observed is the gate evaluating — the `Refuse a fork pull request
loudly` step reports `skipped` on a same-repo PR — and a renderer test holds the condition and
the error text, whose remedy names this project's actual lock commands.

Deliberately not built: the scheduled `pixi update` flavour (it needs the opens-its-own-PR shape,
a PR of its own), and any change to the airlock lane — a merged lock changes nothing on a
restored host until a transport carrying it is packed and published.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
A dependency can now be declared from inside the airlock. The manifest edit is the human's;
the solve belongs to a bot on the connected side, and it is generated rather than hand-written,
so every consumer of `pixi-sandbox init` gets the same lane this repository runs — provably the
same, byte for byte, through `check-repository` check 10.

`relock.yml` is two jobs: a guard that runs `pixi lock --check` on every pull request ahead of
any environment install, so a stale lock fails with a message about the manifest rather than
`setup-pixi`'s message about installation; and a relock job gated on that guard's verdict, which
refreshes the lockfiles onto the branch as `pixi-sandbox[bot]` with a conventional
`chore(lock):` commit and then dispatches CI explicitly, because a `GITHUB_TOKEN` push triggers
nothing. It pins the pixi it solves with to the embedded tools-lock version, so the lane cannot
write a lockfile an airlocked host is unable to read.

Suite 259 → 274 passing / 1 skipped (+15: 12 renderer tests, 3 for check 10). `pixi run lint` is
unchanged at nine gates, now with actionlint over both generated renders and the committed one.
task-38 is unblocked: adding `rstest` is an ordinary manifest edit from here — followed by a
transport repack before an airlocked host can build against it.
<!-- SECTION:SUMMARY:END -->
