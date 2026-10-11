---
id: TASK-91
title: Put both GitHub adapters under one github module
status: Done
assignee: []
created_date: '2026-10-11 11:51'
updated_date: '2026-10-11 18:15'
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
- [x] #1 One `github` module inside `pixi-sandbox` owns both HTTP adapters; `release/github.rs` and `upgrade/github.rs` are gone, the four `commands/` call sites import the new module, and no new crate is created.
- [x] #2 `ReleaseSource` and `PullRequestSource` still live in their `mod.rs` homes and remain coverage-counted; the ignore-filename regex in `crates/package.json:11` names the new file and nothing else that carries tested logic.
- [x] #3 No behaviour change: every existing test passes unchanged and `pixi run --frozen test` is at or above its pre-change count; the diff moves code and shares the request helper rather than rewriting response handling.
- [x] #4 The couplings are updated in the same change and verified: AGENTS.md's repo map describes the new layout, and the coverage guard still passes (`pixi run --frozen test` includes it) — no check silently stops applying.
- [x] #5 The promotion trigger — a second consumer (xtask / TASK-73) or a third API call — is recorded in this task's Implementation Notes with the condition that would revisit the crate decision.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Created `crates/pixi-sandbox/src/github.rs` as the single in-crate socket boundary. It owns
`GitHubReleaseSource` and `GitHubPullRequestSource`; the former adapter files are deleted and all
four command call sites now import the consolidated module. `github_request` centralises only the
shared `Accept`, `User-Agent`, and optional bearer-auth setup. Release still derives its optional
non-empty `GITHUB_TOKEN` itself, upgrade still receives its caller-supplied token, and the existing
URLs, status/error handling, empty-body refusal, checksum-404 behaviour, payload, and response
parsing were moved unchanged.

`ReleaseSource` and `PullRequestSource` remain in their original trait modules. The public
`PIXI_CHECKSUM_MANIFEST` now lives in `release/mod.rs`, preserving `tools_update`'s existing import
and keeping that release decision surface coverage-counted. The coverage command now excludes only the
exact `pixi-sandbox/src/github.rs` filename; `coverage_excludes_only_the_socket_adapter` parses
`crates/package.json` and rejects the deleted paths, so the filename coupling cannot silently
regress. AGENTS.md records both the new repo-map layout and the socket-only testing exception.

**Promotion trigger:** retain this small adapter module inside `pixi-sandbox` until `xtask` becomes
a second consumer (for example through TASK-73) **or** a third distinct GitHub API call is added.
Either condition reopens the crate-extraction decision; neither exists now.

Validation: the focused coverage-config regression test passed; the pre-change `pixi-sandbox`
count was 525 passed / 1 skipped and the final full suite reports 526 passed / 1 skipped for that
crate. `pixi run --frozen fmt`, `pixi run --frozen lint`, and `pixi run --frozen test` all passed;
the full suite includes 42 `pixi-sandbox-git`, 222 `pixi-sandbox-core`, 526 `pixi-sandbox`, and
196 `xtask` tests (plus the one documented skip).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Both GitHub HTTP adapters now share one thin, in-crate `github` module and one request-header/auth
helper. The offline-tested release and upgrade traits stay in their existing modules, the exact
coverage exclusion follows the moved socket boundary, and AGENTS.md documents the new layout and
future extraction trigger. No dependency or crate was added, and the workspace gates are green.
<!-- SECTION:FINAL_SUMMARY:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 11:51

2026-10-11 18:15 — completed; all acceptance criteria and required local gates are green.
<!-- COMMENTS:END -->
