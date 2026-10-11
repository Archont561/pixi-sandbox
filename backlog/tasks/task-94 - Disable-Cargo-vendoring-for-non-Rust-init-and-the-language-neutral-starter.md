---
id: TASK-94
title: Disable Cargo vendoring for non-Rust init and the language-neutral starter
status: In Progress
assignee: []
created_date: '2026-10-11 18:44'
updated_date: '2026-10-11 18:45'
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
- [ ] #4 A corrected published starter completes its real publisher workflow; record the successful run before closing.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented after user confirmation of CLI init and public starter scaffold/verify seams. Red: init_seeds_vendoring_for_the_project_kind::case_1_non_cargo observed Some(true), expected Some(false); stale starter scaffold kept cargo_vendor=true; starter verification returned no findings for an unusable Cargo policy. Green: first-render relock policy follows the seeded config, Cargo.toml enables vendoring even before lock creation, and existing config survives init/force/check. Scaffold now owns pixi-sandbox.toml and rewrites stale starter policy to false; verifier parses the plan and rejects effective root or bundle Cargo vendoring and malformed config. The fixture no longer overwrites the real scaffold config with schema-only text. Review after green kept the change local; no global schema default changed and no reviewed consumer config is rewritten. Reproduced pack refusal offline with the published starter pixi.lock and no Cargo.lock: --cargo-vendor needs a Cargo.lock. GitHub job annotations confirm the pack phase failed; raw logs redirect to a blocked host, so the exact remote stderr was not read. Local gates: fmt passed, lint 11/11, tests 992 passing / 1 skipped (was 986/1). AC#4 remains open: no remote starter mutation, workflow dispatch, or successful republish has been performed. Existing starter users can set cargo_vendor=false and regenerate their owned files; the root README documents the published-version workaround.
<!-- SECTION:NOTES:END -->
