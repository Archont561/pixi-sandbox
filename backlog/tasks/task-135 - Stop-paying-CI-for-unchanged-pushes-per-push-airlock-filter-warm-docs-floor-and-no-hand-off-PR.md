---
id: TASK-135
title: 'Stop paying CI for unchanged pushes: per-push airlock filter, the warm docs-only floor, and no hand-off PR'
status: Done
assignee: []
created_date: '2026-10-24 22:50'
updated_date: '2026-10-25 14:30'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
references:
  - .github/workflows/airlock.yml
  - .github/workflows/ci.yml
  - crates/xtask/src/workflow_gates.rs
  - .agents/skills/session/SKILL.md
  - .agents/skills/session/standup-template.md
  - AGENTS.md
documentation:
  - >-
    backlog/docs/plans/doc-18 - Sprint-Train-and-the-Milestone-Release-Gate.md
priority: high
type: enhancement
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Follow-up hygiene to TASK-126/TASK-127 (both still In Progress; their landed slices came via
PR #202, #204, #205 on 2026-10-24 — do not re-litigate them). Three measured CI costs from the
2026-10-24 session, each with a solution already identified. The theme is one rule: **a push that
cannot change a gate's verdict must not pay for that gate.** This is non-blocking stage-C
hygiene for milestone m-1, not part of its release exit criteria.

### Finding 1 — the airlock re-runs its full proof on markdown-only pushes

`airlock.yml` filters at the trigger (`on.pull_request.paths:`), and GitHub evaluates trigger
paths against the **whole PR diff**, not the pushing commit. Once a PR has touched `crates/**`
or `airlock.yml`, every later push re-runs the complete proof regardless of what that push
contains. In PR #202 the 18:17 and 18:28 pushes were markdown-only measurement commits and still
bought full proofs:

| Run | Push content | Duration |
| --- | --- | --- |
| `40116802649` | code (first push) | 682 s |
| `40120014109` | markdown-only push, same PR | 435 s |
| `40120881120` | markdown-only push, same PR | 415 s |

A current proof costs plan 81 s + linux-64 proof 229 s (run `40129282990`); the two markdown
pushes above therefore spent roughly 13 runner-minutes proving bytes the push could not affect.

**Solution.** Move the decision from trigger paths to a job-level per-push filter, the same
pattern `ci.yml`'s `changes` job already uses. Keep the workflow triggering on every
`pull_request` activity (a paths-ignored trigger leaves required checks Pending forever — the
`ci.yml` comment documents this), add a pinned `dorny/paths-filter` step to the `plan` job that
on `synchronize` evaluates **only the push range** (`github.event.before`..`github.event.after`)
against the five existing airlock path globs, and gate both jobs' `if:` on its output combined
with the existing non-draft condition. On `opened`/`reopened`/`ready_for_review` the filter sees
the full diff and the proof runs as today; `schedule`, `workflow_call` and `workflow_dispatch`
are unaffected and always run.

### Finding 2 — the docs-only CI floor is a 61 s cold compile

On the markdown-only PR #206 (run `40130578805`) the `ci (lint · test · coverage)` job took
87 s, of which `pixi run xtask check-ci-builds` alone was **61 s**: the check only parses
YAML/JSON/TOML, but the xtask binary compiles cold because the `Swatinem/rust-cache` step sits
behind the same `code != 'false'` gate as the Rust builds, so a docs PR restores no `target/`
cache. The same step is 0 s on a warm code PR (run `40129282917`).

**Solution (recommended: A).** Make the rust-cache step restore unconditionally and add
`save-if: ${{ steps.changes.outputs.code != 'false' }}` — a restore is a download when warm and
a no-op when cold, and the airlock `plan` job already uses `save-if` for the same reason. The
xtask gate `check_ci_test_route` asserts only the cache's `cmd-format` and `key`, not its `if`,
so the gate stays green; prove that rather than assume it.
**Alternative B:** gate the three xtask checks behind `code != 'false'`. Every file they judge is
"code" by the filter, so a docs PR cannot break them, and on `push` events the skipped filter
leaves the output empty, which still satisfies `!= 'false'` (main runs stay full). B removes a
self-check from docs PRs; A removes nothing, so prefer A and only fall back with a measured
reason.

### Finding 3 — the standalone hand-off PR is a CI cycle with no product content

The session skill's old §5b made the `CONTEXT.md` scratchpad entry its own commit, which under
invariant 12 (main moves only by PR) became a post-merge hand-off PR. PR #206 was exactly that:
one markdown file costing PR ci runs `40130578805` (87 s) and `40130911798` (119 s), merge-push
ci `40131167563` (107 s) and docs `40131303749` (28 s) — about 341 s of runners for a prompt.

**Solution (drafted, not yet merged).** The skill now prints the session report and next-session
opening prompt in the final chat message after the merge; the scratchpad entry rides the work PR
before it merges, and the only tree-stamping escape hatch is invariant 12's single hand-made
direct-push commit, documented as an exception. This task lands that wording and proves the next
session close follows it without opening a follow-up `docs(context)` PR.

### Constraints — what must not change

- ADR-29 concurrency semantics: PR runs remain cancellable per ref; main runs stay grouped by
  commit SHA and are never cancelled.
- The airlock's two-tier proof, the `AIRLOCK_SKIP_CARGO_BUILD` PR/schedule split (wiring asserted
  on PRs, offline compile on the weekly cron and dispatches), the self-bin cache and the two
  deliberately distinct rust-cache identities (plan vs proof).
- The draft-skip + `ready_for_review` behaviour of the airlock and its `workflow_call` inputs.
- The relock and publish-sandbox trigger path filters — measured correct this session
  (publish-sandbox properly skipped the #206 merge). The `docs.yml` SHA-bound trust chain and
  the `changes`/`ci` duplicated filter that lets msrv and the Windows job start early.
- Required checks must never be left Pending: skip jobs, never narrow a trigger to silence them.
- The one-line-step rule and pinned-action policy (`xtask check-repository` / workflow-shape
  checks). `airlock.yml` and `ci.yml` are hand-maintained, not generated renders — no generator
  or golden-file work is involved.
- No production release, transport publish, tag push or workflow dispatch merely to benchmark;
  evidence comes from ordinary PR runs. Anything an xtask gate enforces gets red/green tests;
  YAML-only behaviour is proven on real runners, because this airlocked machine cannot simulate
  Actions job-level evaluation.

### Explicitly out of scope (already recorded, do not fold in)

Vendor filtering (~350–400 MiB off the transport, measured but unapproved), per-step budgets
(T4), the turbo cache's SHA-keyed primary key (always a cold primary), and `incremental = false`
— all live in `CONTEXT.md` § Session scratchpad (2026-10-24, ninth session) and deserve their
own proposals.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A before/after ledger is attached: run ids, commit SHAs, runner OS, cache hit/miss state and job/step timings for a docs-only push and a code push, with wall time and aggregate runner minutes kept separate. Failed runs and estimates are never presented as measured savings.
- [x] #2 A markdown-only (or otherwise non-airlock-path) `synchronize` push on a PR that previously touched an airlock path skips both airlock jobs while the required checks report green; an `opened`/`reopened`/`ready_for_review` run and any push touching one of the five airlock globs still runs the full two-tier proof, and draft PRs still skip it. The filter evaluates the push range (`before`..`after`) on synchronize, never the whole PR diff. Runner-proven in PR #207: skip in run `40136460735` (10 s, plan/proof skipped, check green — the skip itself proves local push-range mode, since API mode would have returned the whole PR), full proofs in `40134414615` and `40135970226`, and again on the 2026-10-25 PRs #209/#211 whose airlock runs ran the full proof and passed. One sub-clause stays gate-pinned rather than runner-exercised: no draft PR was opened, so the draft skip is asserted by the `airlock_filter.rs` refusal tests, not a runner.
- [x] #3 `schedule`, `workflow_dispatch` and `workflow_call` airlock runs behave exactly as before — full proof, no filter short-circuit — and the concurrency group, `AIRLOCK_SKIP_CARGO_BUILD` split, self-bin cache and separate rust-cache identities are untouched. Evidence boundary, stated plainly: none of the three triggers was re-run after merge — dispatches require explicit owner sanction and none was given, and the weekly cron next fires 2026-10-26 (last scheduled run, `84e8010a` on 2026-10-19, predates this change). The behaviour is instead pinned structurally: the filter step is `pull_request`-only so its output is unset on those events, the `if:` expressions default to "run", and `xtask check-airlock-filter` (red/green refusal tests, executed in every ci) pins both. The concurrency group, cargo-build split, self-bin cache and rust-cache identities are byte-verified unchanged by the same gate.
- [x] #4 A docs-only PR run restores the Rust cache without saving it (`save-if` bound to the code filter; option A), the `check-ci-builds` step shows a measured drop from the 61 s cold floor toward the warm 0 s observed on code PRs, and every previously gated check still executes on code PRs and on every main push. Option A shipped and is pinned by `check_ci_test_route`; the warm 5 s vs. cold 61 s measurement came from run `40136460732`. The push-event fallback (filter skipped, every gate runs) is now runner-proven on all four main pushes after the #207 merge — ci `401364…`, 22:55, 23:58 on 2026-10-24 and the 2026-10-25 13:53 push for PR #211, all green. One comparison remains unobserved: no PR whose *whole* diff is markdown has run since the change, so the full floor-to-floor docs-only PR picture rests on the 5 s warm datapoint plus the gate, not a second complete run.
- [x] #5 Any change to `crates/xtask/src/workflow_gates.rs` (or a new gate enforcing the filters) lands with red/green tests at the existing public seam, including the refusal path; the current `check_ci_builds` assertions still pass against the edited workflows.
- [x] #6 The session-skill wording (SKILL.md §5b steps 2/4/5, standup-template preamble, AGENTS.md intro) is merged: the scratchpad entry rides the work PR before merge, the report and next opening prompt are printed in chat after merge, no standalone `docs(context)` PR is opened, invariants 10 and 12 hold, and the direct-push escape hatch is documented as an exception. Merged in PR #207 (all four files are in its merge list); the retired standalone hand-off PR pattern last occurred as #206 before #207 — every PR since (#208, #209, #211) has been a work PR, and the current wording is present in SKILL.md §5b, the standup template and AGENTS.md.
- [x] #7 fmt, lint, the full suite (baseline 1,223 passing / 2 skipped — it must not fall), actionlint and the repository checks are green, and the task stays open until the real-run evidence for AC#2–#4 exists; a local green run alone does not close a CI-behaviour criterion. The suite has only risen since — 1,277 passing / 0 failed / 4 ignored on the 2026-10-25 main tip — with fmt, clippy, actionlint (inside ci) and every xtask gate green across #209 and #211; the runner-proven portions of AC#2–#4 are cited above, and the gate-pinned remainder is named rather than implied.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 `pixi run --frozen fmt --check`, `pixi run --frozen lint` and `pixi run --frozen test` green, with the before/after suite counts recorded.
- [x] #2 Before/after measurements link real workflow runs and state cache hit/miss; no unapproved dispatch, release, tag or transport publish was performed. All measurements above cite run ids with cache states (the 15 s warm restore, 5 s vs. 61 s xtask step, the 10 s skipped job); no dispatch, release, tag or transport publish was performed at any point.
- [x] #3 The workflow evidence shows skipped jobs satisfying required checks (never Pending) and a code-bearing push still running the full airlock proof and full ci.
- [x] #4 The task file is completed in house format only once every AC has evidence; anything provable solely on a native runner is left open and named, not checked. Closed 2026-10-25 after the #207 merge and four green main pushes; the three clauses without runner evidence (draft-PR skip, schedule/dispatch/call short-circuit, the pure-markdown whole-PR floor comparison) are named inside AC#2–#4 above with the gate tests that pin them, rather than presented as runner-proven. The 2026-10-26 scheduled airlock run is the next opportunity to confirm AC#3's cron path without an authorized dispatch.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Vertical slices, biggest measured waste first; agree the filter expression before editing YAML.

1. Re-baseline (AC#1): capture current timings on one fresh docs-only push and one code push,
   including the per-step split and cache state, so every later number is comparable.
2. Airlock per-push filter (AC#2, AC#3): add the push-range paths-filter to the `plan` job and
   combine its output with the existing draft `if:`; let `proof` inherit the skip through
   `needs`. Verify with a markdown-only push and then a code push on the same PR.
3. Warm the docs-only floor (AC#4): unconditional rust-cache restore plus `save-if`; run the xtask
   gate locally first, and add/extend a gate test (AC#5) if the workflow assertions move.
4. Land the session-skill change (AC#6) — the wording is already drafted on the working branch;
   merge it as part of a work PR, never as its own hand-off PR.
5. Full gates and the evidence ledger (AC#7), then close only the ACs whose real-run proof exists.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-24 scoping (from the CI-history review, no implementation yet):

- Evidence runs — airlock waste on PR #202: `40120014109` (435 s, markdown push),
  `40120881120` (415 s, markdown push) vs code push `40116802649` (682 s); current proof shape
  `40129282990` (plan 81 s, proof 229 s). Docs floor: `40130578805` — check-ci-builds 61 s of an
  87 s job; warm counterpart `40129282917` — same step 0 s. Hand-off PR #206 total:
  `40130578805` 87 s + `40130911798` 119 s + push ci `40131167563` 107 s + docs
  `40131303749` 28 s ≈ 341 s.
- Gate fact verified in `workflow_gates.rs::check_ci_test_route`: it pins the rust-cache
  `cmd-format`/`key` and the test gates' `if:` strings, but does not assert the cache step's own
  `if`, so the AC#4 change is compatible; keep it that way or update the test deliberately.
- On `push` events the ci paths-filter step is skipped, so `steps.changes.outputs.code` is an
  empty string and every `!= 'false'` condition stays true — main-push behaviour must remain
  full; add a test that pins this if the conditions move.
- No hard dependencies: TASK-126/TASK-127 remain In Progress but this task neither blocks nor
  requires them; it deliberately avoids the paths they already optimized.

2026-10-24 implementation (slides 1–3; real-run ACs still open):

- **Finding 1 — per-push airlock filter.** `airlock.yml` gains a `changes` job (mirroring
  ci.yml's architecture): pinned `dorny/paths-filter`, PR-only, whose `base`/`ref` select
  `github.event.before..after` on `synchronize` and `pull_request.base.sha..head.sha` on every
  other activity. Its `run-airlock` output defaults to "run" outside pull requests, so
  schedule/dispatch/workflow_call are untouched. `plan` now `needs: changes` and `proof` needs
  `[changes, plan]`; both `if:` expressions combine the output with the existing draft guard.
  The trigger-level `on.pull_request.paths` is unchanged — `xtask check-main-push`
  (ADR-29 guard) text-parses that exact block, and a paths-ignored trigger would leave required
  checks Pending whereas skipped jobs post a conclusion (precedent: ci.yml's msrv/windows jobs).
- **New gate `xtask check-airlock-filter`** (`workflow_gates.rs`, wired through `main.rs`, run
  unconditionally in ci.yml next to the other xtask checks). It parses airlock.yml and fails
  closed when: the `changes` job/output/step is missing, the filter's five globs diverge from
  the trigger `paths`, the base/ref lose the synchronize push range, or either proof job drops
  the `needs: changes` gate or the draft condition. Tests: `tests/airlock_filter.rs` — reference
  shape passes, nine refusal mutations, missing-job and missing-file cases (4 tests).
- **Finding 2 — warm docs-only floor.** ci.yml's `Swatinem/rust-cache` step now restores
  unconditionally and carries `save-if: ${{ steps.changes.outputs.code != 'false' }}`; the
  `check_ci_test_route` gate pins both properties (no step `if`; the exact `save-if`), with
  three new refusal mutations in `tests/ci_builds.rs` and the ci-builds fixture updated in
  lockstep. The gate already asserted only `cmd-format`/`key`, so the extension is additive.
- **Finding 3 — hand-off in chat.** The session-skill wording (SKILL.md §5b steps 2/4/5,
  standup-template preamble, AGENTS.md intro) rides this same PR; no standalone
  `docs(context)` PR is opened, per the rule it introduces.
- **Local evidence (this airlocked machine).** `fmt --check` clean; `lint` 7/7 green incl.
  clippy workspace, `check-repository` (all 12 checks, incl. one-line-step and permissions),
  actionlint clean on both edited workflows; `xtask check-airlock-filter`, `check-ci-builds`,
  `check-repository`, `check-main-push` all pass against the real workflows. Suite:
  **1,228 passing / 2 skipped** (593 product, 315 core, 59 git, 261 xtask) against the
  1,223/2 pre-implementation baseline — exactly five new tests, no drops.
- **Still open by design (needs GitHub runners, cannot be proven locally).** AC#1's after
  numbers; AC#2's skipped-job verdict on a markdown-only synchronize push and a full proof on
  the next code push; AC#3's unchanged schedule/dispatch/call behaviour; AC#4's measured
  check-ci-builds step time on a warm docs-only PR; AC#6's merge; AC#7's closure. No dispatch,
  release, tag or transport publish was performed.

2026-10-24 real-runner evidence (PR #207):

- **First-push fault caught and fixed on the runner.** The opening push ran the `changes`
  job with `predicate-quantifier: every` (copied from ci.yml's "all files are code" filter):
  run `40134136518` — changes success, plan/proof **skipped**, because a mixed code+docs push
  does not have every file under a watched path. Fixed in `d3bafe54` with `some` semantics
  ("at least one watched path touched"), pinned in the gate and a refusal test.
- **Positive path proven.** Run `40134414615` on the fix push (which touches
  `.github/workflows/airlock.yml` and `crates/xtask/**`): changes → plan → linux-64 proof all
  ran and succeeded; the changes job's real work is checkout 1 s + paths-filter 1 s against
  ~390–490 s of proof it gates. ci run `40134414604` green on the same push.
- **Second runner fault, root-caused to the action contract.** The markdown-only
  synchronize push that followed nevertheless ran the full proof (run `40135163133`).
  dorny/paths-filter's pinned source (`src/main.ts`, commit `ceb8a2b`) shows that on
  `pull_request` events a non-empty `token` (the default `${{ github.token }}`) makes it
  ignore `base`/`ref` and read the whole-PR file list through the API. Fix `34ffc421` sets
  `token: ''`, routing to local git diff where the explicit `before..after` range applies;
  verified locally that `git diff --name-status d3bafe54..d087805a` is exactly the one
  markdown file and matches no watched glob. The gate now pins the empty token and the
  `some` quantifier, each with a refusal mutation.
- **Local-mode positive path.** Run `40135970226` (push touching `airlock.yml` and
  `crates/xtask/**`): changes → plan → linux-64 proof all green under local git mode; ci run
  `40135970225` green.
- **Skip proven on a real runner (AC#2 clauses 1 and 4).** The markdown-only synchronize
  push `a6630f50` produced airlock run `40136460735`: **10 s wall time, `changes` success,
  plan and linux-64 proof skipped, run conclusion success** — the required check is
  satisfied, not Pending. Because token-mode API diffing would have returned the whole PR
  (which contains `crates/**`) and run the proof, the skip itself demonstrates that local
  `before..after` range evaluation is in effect. Runner cost: one ~10 s job instead of
  plan 491 s + proof 386 s measured on run `40135970226` (ubuntu-latest throughout).
- **Warm docs floor measured (AC#4 core claim).** On that same push the ci job
  (`40136460732`) now runs the rust-cache step on the docs path — restore 15 s where the
  step used to be gated off entirely — and `xtask check-ci-builds` takes **5 s warm versus
  61 s cold** on PR #206's run `40130578805`; the new `check-airlock-filter` step is
  sub-second. Caveat, stated plainly: ci.yml's own paths-filter is deliberately PR-wide, so
  this code-carrying PR still ran its full Rust gates; a PR whose *whole* diff is markdown
  (the #206 shape) is what shows the full floor-to-floor comparison, and no such PR has run
  since the change. Main-push behaviour likewise only becomes observable after merge.
- **Gaps still open, named rather than checked.** AC#2's draft-skip clause was not
  re-exercised (no draft PR was opened; the expression and the gate test pin the clause).
  AC#3's `schedule`/`workflow_dispatch`/`workflow_call` paths were deliberately not
  re-run — no dispatch is authorized for benchmarking; the output defaults to "run"
  outside `pull_request` events and the gate pins that expression. AC#6 awaits the merge.
  No release, tag, transport publish or workflow dispatch was performed at any point.

2026-10-25 closure (owner-directed, pre-0.9.0 board hygiene):

- PR #207 merged 2026-10-24 22:10 with every implementation file and the AC#6 session-skill
  wording. Main has since taken four pushes (22:10, 22:55, 23:58, and PR #211 at 13:53),
  each running full `ci` green — runner evidence that the push-event fallback keeps every
  gate live when the PR-only filter does not evaluate. No standalone `docs(context)` PR
  exists after #207; #208/#209/#211 are all work PRs, so the chat hand-off rule is being
  followed. The suite rose from 1,228 to 1,277 passing with zero failures.
- Checked now: #2 (skip and full-proof paths runner-proven; draft clause gate-pinned),
  #3 (structurally pinned by the PR-only filter plus `check-airlock-filter`; no
  dispatch authorized, cron next fires 2026-10-26), #4 (warm-floor datapoint plus four
  full main pushes; pure-markdown whole-PR comparison still unobserved), #6 (merged and
  observed in practice), #7, and DoD #1–#4. The three never-runner-exercised clauses are
  recorded inline above, not claimed as runner evidence. This task is non-blocking stage-C
  hygiene and is not one of m-1's release exit criteria.
<!-- SECTION:NOTES:END -->
