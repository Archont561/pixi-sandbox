---
id: TASK-33
title: Persist restored Pixi tools on the user PATH
status: To Do
assignee: []
created_date: '2026-10-01 16:30'
labels:
  - restore
  - cli
  - shell
  - platform
milestone: m-0
dependencies:
  - TASK-21
references:
  - crates/pixi-sandbox/src/commands/restore.rs
  - crates/pixi-sandbox/src/commands/init.rs
  - scripts/restore.sh
  - docs/src/content/docs/restore.mdx
priority: high
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
After a transport has been fully verified and restored, make its bundled `pixi` and `pixi-sandbox` tools permanently discoverable for the user. Pixi needs both commands on `PATH`: `pixi` is the default bundled tool, and Pixi resolves `pixi sandbox` by searching `PATH` for `pixi-sandbox`. The persistent registration must point at the manifest-verified tool copies, be idempotent, and avoid requiring `source .pixi/sandbox-env.sh` for normal `pixi` or `pixi sandbox` use.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A successful restore registers both `pixi` and `pixi-sandbox` in a persistent per-user bin location by default, only after branch and restored-tree verification have passed
- [ ] #2 POSIX uses managed links or launchers to the restored tools and Windows uses an equivalent user-level mechanism that does not require administrator privileges; the bin location is configurable for airlock policy
- [ ] #3 The user bin directory is added idempotently to the detected shell's persistent PATH (and Windows user PATH), with a clear notice that an already-running parent shell cannot be mutated and a new shell sees the change
- [ ] #4 Re-restoring the same or another project safely retargets only pixi-sandbox-managed entries, refuses unrelated existing `pixi` or `pixi-sandbox` commands unless explicitly forced, and documents that the most recently registered restore becomes the user-level tool source
- [ ] #5 An explicit opt-out disables all home/profile changes for CI, shared accounts, and locked-down airlocks without changing restore verification or project output
- [ ] #6 Fixture-backed tests use isolated HOME, profile, PATH, and user-bin directories; they cover first install, idempotent repeat, managed retarget, collision refusal/force, opt-out, missing HOME, and discovery of `pixi sandbox`
- [ ] #7 Documentation distinguishes persistent tool discovery from full environment activation: `pixi` and `pixi sandbox` work directly, environment commands should use `pixi run`, while `.pixi/sandbox-env.sh` remains available when direct `cargo`, `rustc`, or `bun` commands are wanted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Model user-tool registration as a filesystem/profile service with an explicit root so tests never touch the developer's home. Register both tool names because Pixi extension discovery is PATH-based. Use recognizable managed entries rather than copying untracked binaries, update persistent PATH with a marker block or native user-environment API, and perform the operation only after restore verification. Add force and opt-out controls to the Rust CLI and generated launchers, then update airlock documentation and CI callers to choose the intended policy explicitly.
<!-- SECTION:PLAN:END -->
