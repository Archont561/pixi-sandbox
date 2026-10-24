---
id: doc-18
title: 'Sprint train and the milestone release gate: 0.9.0'
type: plan
created_date: '2026-10-24 18:20'
updated_date: '2026-10-24 18:20'
tags:
  - planning
  - release
  - sprint
---

# Sprint train and the milestone release gate: 0.9.0

**Status: planning only.** This document changes no task status, claims no implementation and
adds no scope to any task. It defines how a session, a sprint and a milestone relate in this
repository, proposes the sprint that targets milestone [m-1](../../milestones/m-1%20-%200.9.0-nested-workspaces-in-one-transport.md),
and lists what must exist before that sprint can start — including two capabilities this
repository does not have today.

**Baseline measured 2026-10-24** at `b246bf6c` (main, after PR #201): latest published release
`v0.8.0` (2026-10-22T19:27:27Z); the airlock run that proved PR #201 is green on both tiers; the
last full local run on this tree reported **1,218 passed / 2 skipped** (593 product, 315 core,
54 git, 256 xtask). Nothing else here was re-measured for this document.

## 1. The working model this document sets up

The repository has been running **task-long** sessions: propose one task, work it, merge it, hand
over. That is a fine cadence for a bug, and it is the wrong one for shipping. This document
switches the default to **sprint-long**, and the four nouns get fixed meanings:

| Noun | Meaning | Lives in |
| --- | --- | --- |
| **Sprint** | One session's whole scope, from proposal to release. The unit a session is named after. | this document |
| **Milestone** | Exactly one version, plus the exit criteria that make that version releasable. | `backlog/milestones/m-N` |
| **Version** | The tag a sprint produces. One sprint → one milestone → one tag. | `.versionrc`, `xtask prepare-release` |
| **Task** | A unit of work *inside* a sprint. Four to eight per sprint, no more. | `backlog/tasks/` |

A sprint has four stages, always in this order:

1. **Spike** — the one unknown that could invalidate the rest. Timeboxed, and its output is
   evidence (notes on the task, or an amendment to a design note), not a feature.
2. **Build** — the milestone's tasks on top of what the spike proved.
3. **Gate** — the proofs this repository actually trusts: `fmt`, `lint`, the full suite, and
   `airlock linux-64` with both tiers on the release branch.
4. **Release** — version bump, release PR, merge, **milestone tag**, watched release runs.

The sprint is not finished when the PR merges. It is finished when the milestone tag has run the
release lane and the milestone file is closed — that is the same "a merge is not the end of a
session" rule the session skill already carries, moved up one level.

**Task-long stays available.** When the user asks for a single task, or when the sprint's proof
needs many CI rounds, native runners or a maintainer's click (the airlock and release lanes both
qualify), the session runs task-long and says so in its proposal. The default, though, is
sprint-long: one spike, one version, one merge, one release.

## 2. Where the board is, measured

Open work at `b246bf6c`:

| Task | Status | Acceptance criteria | Subject |
| --- | --- | --- | --- |
| TASK-125 | In Progress | 9 done, 0 open | rattler migration; shipped through v0.8.0 |
| TASK-126 | In Progress | 4 done, 4 open | CI minutes |
| TASK-127 | In Progress | 10 done, 3 open | supporting-workflow latency |
| TASK-129 | In Progress | 7 done, 4 open | explicitly declared nested Pixi workspaces |
| TASK-130 | To Do | 0 done, 10 open | opt-in Go, NuGet and Maven vendoring |
| TASK-131 | To Do | 0 done, 7 open | per-workspace Cargo vendor trees |
| TASK-73 | Deprecated | — | owner-operated GitHub App delivery |

Two facts about milestones:

- `m-0` ("v1 standalone cross-platform workflow") is effectively complete: 20 tasks carry
  `milestone: m-0` and every one of them is `Done`.
- **No open task carries a milestone at all.** Milestones here are hand-authored directories under
  `backlog/milestones/`, and the documented `backlog` command surface (`.agents/skills/backlog/REFERENCE.md`) has no
  milestone command at all — no create, no list, no assign — so attaching a task is a hand edit
  of its frontmatter.

## 3. Milestone m-1 — 0.9.0

[m-1](../../milestones/m-1%20-%200.9.0-nested-workspaces-in-one-transport.md) proposes **0.9.0 —
"Nested workspaces in one transport"**: TASK-129 finished, then TASK-131, with TASK-126/127 as
non-blocking hygiene, closed by an airlock-green release branch.

Why this and not TASK-130: TASK-130 is new-ecosystem work with ten untouched criteria and no
design note; it needs a spike of its own and becomes **m-2**. TASK-129 is already seven of eleven
criteria in, and it is the only open task that another open task depends on (TASK-131 carries the
nested-workspace context), so finishing it is what makes the next milestone startable rather than
merely larger.

### Sprint 1 of m-1 — the sprint this document creates

| Stage | Work | Exit |
| --- | --- | --- |
| A — spike | TASK-129's remaining criteria, starting with the one it cannot close offline: a **pixi-generated lock** for the nested fixture | A recorded answer: either the lock is produced and the nested transport verifies against real observables, or the limitation is written into the task with the evidence that produced it |
| B — build | The rest of TASK-129, then TASK-131 | Both tasks closed in house format |
| C — gate | TASK-126/127 as capacity allows; `fmt`, `lint`, full suite, `airlock linux-64` on the release branch | Green gates, airlock proving a transport that carries the nested workspace |
| D — release | `prepare-release`, the release PR, the merge, the milestone tag, the watched `release` + `publish sandbox` runs | `v0.9.0` published; m-1 closed |

Out of scope: TASK-130, TASK-73, and anything that is not needed to make 0.9.0 releasable.

## 4. What the release lane does today

Verified against the workflow files at `b246bf6c`:

- **`auto-release.yml`** — triggers on `workflow_dispatch` and `workflow_call`, **nothing else**.
  It runs `xtask prepare-release` (selector `auto | major | minor | patch | vX.Y.Z`), which bumps
  the workspace version, refreshes `Cargo.lock`, regenerates `CHANGELOG.md` and restamps the
  committed relock render; then `xtask commit-release <tag>` commits exactly those files, creates
  the annotated tag and pushes both. It requires `RELEASE_PUSH_TOKEN` for a real cut, because
  `GITHUB_TOKEN` may not push `.github/workflows/**`.
- **`release.yml`** — triggers on `push: tags: v*.*.*` (and dispatch). Builds the five static
  tier-1 binaries, the conda packages on native runners, and publishes the GitHub Release with
  `SHA256SUMS`.
- **`publish-sandbox.yml`** — triggers on `push: branches: [main]` and repacks the transport.
- The tag push and the merge push start both consumers themselves; there is deliberately no
  dispatch step between them.

So the lane can build and publish a version. What it cannot do is *start by itself*: nothing in
this repository reacts to a milestone.

## 5. What is needed before the sprint can start

Seven items, in dependency order. The first is delivered with this document; the rest are the
gap between "a session that merges a PR" and "a sprint that ships a version".

### 5.1 A sprint-long session skill

**Delivered with this document** (`.agents/skills/session/SKILL.md` and its templates): the
default session proposes a *sprint* — milestone, stage plan, decisions needed — and stops; tasks
are worked inside it; the close is the release plus the report. Task-long remains available on
request or when the proof needs many CI runs.

### 5.2 A milestone tag that starts the release lane

**Gap, verified:** `auto-release.yml` has no tag trigger, and no file under `.github/workflows/`
mentions a milestone at all (`grep -ril milestone .github/workflows/` returns nothing). The pattern that works elsewhere in this account is
`Archont561/qgis-sdk`: its `autorelease.yml` triggers on `push: tags: ["milestone-v*",
"milestone/*", "m-v*"]`, decides the bump from conventional commits, and its `release.yml`
explicitly excludes those globs so a milestone tag never builds artifacts directly.

**What closing it means here:** add a `push: tags:` trigger to `auto-release.yml` and a step that
resolves the milestone tag to a version (`milestone-v0.9.0` → `--version v0.9.0`, or `bump=auto`
when the tag carries no version), then verify three things that this repository has already been
bitten by:

- `release.yml`'s `v*.*.*` glob does **not** match `milestone-v0.9.0` (it does not — the pattern
  anchors on a leading `v`), so no second build starts.
- The tag must be pushed with `RELEASE_PUSH_TOKEN`, not `GITHUB_TOKEN`: GitHub suppresses
  workflow triggering for events created with `GITHUB_TOKEN`, which is the same reason the
  release commit itself needs that token.
- `xtask check-repository`'s "no workflow pins a literal release tag" check must still pass — a
  glob is not a literal, but the new step must not hardcode `v0.9.0`.

### 5.3 A release-gate task per milestone

**Gap:** this board has no task whose acceptance criteria are "the version is published". qgis-sdk
models it as a per-milestone task (`TASK-28 — Prepare and qualify 0.2.0 release`) that carries the
release's criteria and closes the milestone. Without one, a milestone closes by assertion.

The proposed task for m-1 carries: the airlock green on the release branch; `fmt`/`lint`/suite
green; `CHANGELOG.md` section generated by `prepare-release`; tag pushed; `release.yml` green on
all five platforms; conda packages published; `publish sandbox` green; the milestone file closed;
the next sprint document written.

### 5.4 Milestone attachment on the tasks

**Gap:** no open task carries a milestone. Closing it means adding `milestone: m-1` to the
frontmatter of TASK-129 and TASK-131 (and to TASK-126/127 if they are in the sprint), by hand,
since the CLI has no milestone command. This is a board change, so it is proposed here rather
than applied with the document.

### 5.5 Board hygiene before the sprint opens

- **TASK-125** has 9 done and 0 open criteria but is still `In Progress`. Either its final summary
  is written and it is closed, or the reason it stays open is recorded — a milestone cannot close
  over a task that is finished but not closed.
- **TASK-129's** remaining four criteria include the one Stage A exists to answer.
- **TASK-126/127** are hygiene: they may ship in this sprint, but they do not gate it.

### 5.6 The spike's own obstacle, named

TASK-129 cannot close its observables criterion without a **pixi-generated lock**: the packer
fixture's hand-built `pixi.lock` is rejected by pixi (`missing field 'subdir'`), so
`pixi install --frozen --offline` cannot be asserted on fixture transports. This is recorded in
TASK-129's notes and is the reason Stage A starts there. If a pixi-generated lock cannot be
produced inside the airlocked environment, the sprint's proposal must say so before Stage B
begins, and 0.9.0 ships without that criterion or waits.

### 5.7 An airlock that actually runs on the release branch

**Verified:** `airlock.yml` triggers on pull requests touching `crates/**`, `pixi.toml`,
`scripts/restore.sh`, `.pixi-sandbox.toml` or `.github/workflows/airlock.yml`. A release PR that
touches only version files and documents will therefore merge **without** the proof the milestone
claims. Either Stage C includes a deliberate dispatch (`gh workflow run airlock.yml`, which needs
the `workflow` scope some tokens lack) or the release commit's paths are checked before the gate
is claimed. A green `ci` is not an airlock.

## 6. The loop, once 5.2 and 5.3 exist

```
sprint proposal ─► spike ─► build ─► gate ─► prepare-release
                                                  │
                                                  ▼
                                    release PR ─► merge to main
                                                  │
                                    milestone tag ─► auto-release (bump, commit, tag)
                                                  │
                        ┌─────────────────────────┴──────────────────────────┐
                        ▼                                                    ▼
              release.yml (5 binaries,                          publish-sandbox.yml
              conda, GitHub Release)                            (repack the transport)
                        └─────────────────────────┬──────────────────────────┘
                                                  ▼
                          release-gate task closed ─► m-N closed
                                                  │
                                                  ▼
                          next sprint document (doc-19) + opening prompt
```

## 7. Decisions the owner has to make

These change the shape of the work, so they are asked rather than assumed:

1. **Tag vocabulary.** `milestone-v0.9.0` (qgis-sdk's spelling, carries the version),
   `m-1` (carries the milestone id and forces the bump to be computed), or `release/0.9.0`.
   *Recommendation: `milestone-v0.9.0`* — the version is visible in the tag, and the workflow can
   pass it straight to `prepare-release` without a second source of truth.
2. **What opens `m-1`'s sprint.** TASK-129 plus TASK-131 (the proposal here), or TASK-129 alone
   with TASK-131 in m-2. *Recommendation: both* — 131 is what makes the nested transport useful
   to a real repository with several Cargo workspaces.
3. **Whether the milestone tag is pushed by a human or by the merge.** A tag pushed by the merge
   needs a workflow with a `workflow`-scoped token; a human-pushed tag needs one `git push`
   command after the merge. *Recommendation: the human command first* (one line, reviewable, no
   new token surface), automation second.
4. **Whether 5.2 is in this sprint.** Wiring the tag trigger is a workflow change with its own
   proof burden; the sprint can ship 0.9.0 by pushing the tag after `prepare-release` by hand and
   land 5.2 in m-2. *Recommendation: land 5.2 first*, because a sprint whose release depends on a
   human remembering a command is not the cadence this document sets up.
