---
id: TASK-91
title: Put both GitHub adapters under one github module
status: To Do
assignee: []
created_date: '2026-10-11 11:51'
labels:
  - refactor
  - github-api
  - coverage
dependencies:
  - TASK-90
references:
  - crates/pixi-sandbox/src/release/github.rs
  - crates/pixi-sandbox/src/upgrade/github.rs
  - crates/pixi-sandbox/src/release/mod.rs
  - crates/pixi-sandbox/src/upgrade/mod.rs
  - crates/package.json
  - package.json
  - AGENTS.md
priority: medium
type: refactor
ordinal: 91000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two files are the only socket-touching code in the tree, and they duplicate the same request shape:
`crates/pixi-sandbox/src/release/github.rs` (93 lines, `GitHubReleaseSource`) and
`crates/pixi-sandbox/src/upgrade/github.rs` (74 lines, `GitHubPullRequestSource`). Each one
independently spells `Accept: application/vnd.github+json`, `User-Agent: pixi-sandbox` and the
bearer header, and each carries the same "keep it thin, decisions live in `mod.rs`" comment that has
to be kept true twice. Consolidate both adapters into one `github` module inside `pixi-sandbox`.
There are four call sites, all in `commands/`: `fetch_release.rs:21`, `self_update.rs:23`,
`tools/update.rs:44`, `upgrade.rs:52`. This is a move plus one shared request helper — no dependency
is added, no crate boundary is introduced, and request semantics (retries, timeouts, status
classification) are out of scope unless TASK-90's outcome already brings them along.

**Do not extract a `pixi-sandbox-github` crate yet.** `pixi-sandbox-git` is not the precedent it
looks like: it is 1,686 lines of domain logic with two consumers (CLI and xtask), while this is two
HTTP adapters with one consumer — the CLI. Extraction would also break couplings that no check
catches if they are missed:

- The coverage exclusion is a **filename regex** at `crates/package.json:11`
  (`'release/github\.rs$|upgrade/github\.rs$'`). A new file name that the regex does not match starts
  counting socket code in the number; two old names left in it silently exclude nothing.
- The root `package.json` carries explicit per-crate lists: `lint:rust` (line 10) enumerates
  `@repo/pixi-sandbox-core`, `@repo/pixi-sandbox-git`, `@repo/pixi-sandbox`, `@repo/xtask`, and
  `lint:toml` (line 14) enumerates every `Cargo.toml` by path. A new crate missing from either list
  is silently un-linted.
- The traits stay where the tests reach them — `ReleaseSource` in `release/mod.rs`,
  `PullRequestSource` in `upgrade/mod.rs` — never inside the file the coverage config excludes.
  Moving a trait into the excluded file takes its tested decisions out of the number.
- `coverage_guard_requires_a_test_route_for_every_production_module`
  (`crates/pixi-sandbox/tests/fixtures.rs:422`) matches on the file **stem** for everything but
  `mod.rs`/`lib.rs`, so the new file's name has to appear in the test sources or the guard fails.
- AGENTS.md's repo-map row for `crates/pixi-sandbox/src/release/` describes *two files* and the
  division of labour between them; TASK-84 brought the map back to the tree, so this change keeps it
  there.

**Promotion trigger to record when this lands:** a second consumer (xtask — e.g. TASK-73's
owner-operated GitHub App work) or a third GitHub API call. Until one of those exists, the adapters
stay in-crate and the module stays small.

**Sequence after TASK-90.** If the `self_update` crate is adopted, `release/github.rs` largely
disappears and this task shrinks to one adapter; if it is rejected, both move as written. The
dependency above is that sequencing, not a build dependency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 One `github` module inside `pixi-sandbox` owns both HTTP adapters; `release/github.rs` and `upgrade/github.rs` are gone, the four `commands/` call sites import the new module, and no new crate is created.
- [ ] #2 `ReleaseSource` and `PullRequestSource` still live in their `mod.rs` homes and remain coverage-counted; the ignore-filename regex in `crates/package.json:11` names the new file and nothing else that carries tested logic.
- [ ] #3 No behaviour change: every existing test passes unchanged and `pixi run --frozen test` is at or above its pre-change count; the diff moves code and shares the request helper rather than rewriting response handling.
- [ ] #4 The couplings are updated in the same change and verified: AGENTS.md's repo map describes the new layout, and the coverage guard still passes (`pixi run --frozen test` includes it) — no check silently stops applying.
- [ ] #5 The promotion trigger — a second consumer (xtask / TASK-73) or a third API call — is recorded in this task's Implementation Notes with the condition that would revisit the crate decision.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 11:51
<!-- COMMENTS:END -->
