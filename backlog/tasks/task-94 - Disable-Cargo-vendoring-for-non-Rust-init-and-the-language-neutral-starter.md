---
id: TASK-94
title: Disable Cargo vendoring for non-Rust init and the language-neutral starter
status: Done
assignee: []
created_date: '2026-10-11 18:44'
updated_date: '2026-10-15 21:10'
labels:
  - bug
  - starter
  - init
dependencies: []
priority: high
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The v0.6.2 starter publisher run 38164114312 fails during pack. The published language-neutral tree has no Cargo.lock, but pixi-sandbox.toml requests cargo_vendor=true. init hardcodes that default, and starter generation only proved the dev task and file shape, not packability. Confirmed test seams: black-box init CLI and public starter scaffold/verify APIs. Preserve existing consumer config; correct starter-owned config explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 New non-Cargo projects default to cargo_vendor=false; Cargo projects retain vendoring; first init passes init --check.
- [x] #2 Existing reviewed vendoring policy is unchanged by init, force, or check.
- [x] #3 Starter scaffolding replaces stale Cargo policy and verification rejects effective Cargo vendoring or invalid config.
- [x] #4 A corrected published starter completes its real publisher workflow; record the successful run before closing.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented after user confirmation of CLI init and public starter scaffold/verify seams. Red: init_seeds_vendoring_for_the_project_kind::case_1_non_cargo observed Some(true), expected Some(false); stale starter scaffold kept cargo_vendor=true; starter verification returned no findings for an unusable Cargo policy. Green: first-render relock policy follows the seeded config, Cargo.toml enables vendoring even before lock creation, and existing config survives init/force/check. Scaffold now owns pixi-sandbox.toml and rewrites stale starter policy to false; verifier parses the plan and rejects effective root or bundle Cargo vendoring and malformed config. The fixture no longer overwrites the real scaffold config with schema-only text. Review after green kept the change local; no global schema default changed and no reviewed consumer config is rewritten. Reproduced pack refusal offline with the published starter pixi.lock and no Cargo.lock: --cargo-vendor needs a Cargo.lock. GitHub job annotations confirm the pack phase failed; raw logs redirect to a blocked host, so the exact remote stderr was not read. Local gates: fmt passed, lint 11/11, tests 992 passing / 1 skipped (was 986/1). AC#4 remains open: no remote starter mutation, workflow dispatch, or successful republish has been performed. Existing starter users can set cargo_vendor=false and regenerate their owned files; the root README documents the published-version workaround.

2026-10-15 — AC#4 proven; task closed.

Starter run https://github.com/Archont561/pixi-sandbox/actions/runs/38804692103 regenerated and published the language-neutral starter from release v0.7.0 (source commit d54ae974f487e1946d059ee6707dc018724ee32b). It became revision 66704220209df2294655bc61570a8aa77eda40df, tag pixi-sandbox-v0.7.0. This is the first corrected starter the lane itself produced: its `pixi-sandbox.toml` carries `cargo_vendor = false`, and the verifier, which now rejects effective Cargo vendoring, passed.

The push started the starter's own publisher, run https://github.com/Archont561/pixi-sandbox-starter/actions/runs/38804893239, which succeeded: the plan, then the `developer`/`default`/`linux-64` leg with Cargo vendoring `false` (fetch the pinned release, then pack, verify and publish). Its transport, `sandbox/developer-linux-64` at ddc086e4c6362c0eddce9c628e3d58f09e61123a, has a schema-2 manifest naming source commit 6670422, a `lock_sha256` equal to the starter `pixi.lock` digest, and static pixi-sandbox 0.7.0.

Earlier runs were never this criterion's proof. The v0.6.2 starter's publisher run 38164114312 failed in pack (`--cargo-vendor needs a Cargo.lock`). The green 2026-10-11 run 38168357510 ran on the owner's hand edit c971ef773ddf8490b29942c332a3d70819820ef7, not on lane output.

The root README's v0.6.2 caution is reworded in the same pull request. It still tells projects created from v0.6.2 to set `cargo_vendor = false`, and no longer claims the failed run is unreplaced.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Non-Cargo projects now start with Cargo vendoring disabled, reviewed vendoring policy survives init, force and check, and the language-neutral starter owns an explicit non-Cargo plan that its verifier enforces. AC#1–#3 were proven locally. AC#4 is proven live: the v0.7.0 starter (revision 66704220, run 38804692103) carries `cargo_vendor = false`, and its own publisher, run 38804893239, packed, verified and published its transport. Task closed.
<!-- SECTION:FINAL_SUMMARY:END -->
