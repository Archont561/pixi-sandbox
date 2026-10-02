---
id: TASK-40
title: >-
  Wire restored Cargo vendor through Pixi activation when project cargo config
  exists
status: Done
assignee: []
created_date: '2026-10-02 09:04'
updated_date: '2026-10-02 09:11'
labels:
  - restore
  - cargo
  - airlock
milestone: m-0
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/55'
documentation:
  - backlog/docs/specifications/transport-and-restore/doc-2
priority: high
type: bug
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub issue #55 reports that restore materialises `.pixi-sandbox/vendor/` but cannot safely wire Cargo when a project already tracks `.cargo/config.toml`: `auto` leaves the vendor tree unused, while `write` clobbers project-owned Cargo configuration. It also reports that generated launchers fail when a connected clone has not fetched the sandbox branch. Implement a default restore path that keeps Cargo source replacement in the sandbox-owned namespace, makes `pixi run -- cargo ... --offline` use it without resurrecting `.pixi/sandbox-env.sh`, and lets generated launchers fetch the selected sandbox branch when it is absent locally.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Default restore with vendored Cargo writes the source-replacement config under .pixi-sandbox/cargo-home and does not create or modify .cargo/config.toml.
- [x] #2 Every restored environment gets Pixi/Conda activation hooks that set CARGO_HOME to that sandbox-owned cargo home and put its bin directory on PATH, so `pixi run -- cargo ... --offline` uses the restored vendor tree while preserving project-owned Cargo config.
- [x] #3 Explicit --cargo-config write keeps the destructive legacy behaviour; --cargo-config print and none stay non-mutating and their final output does not claim Cargo was wired.
- [x] #4 Restore output and CLI help say what actually happened and no longer mention sandbox-env.sh or a cargo-sandbox wrapper.
- [x] #5 Fixture tests cover a project with an existing .cargo/config.toml and prove it survives byte-for-byte while the sandbox cargo-home config is written.
- [x] #6 Generated `restore.sh` / `restore.ps1` fetch the selected sandbox branch when neither the local nor remote-tracking ref exists, and `PIXI_SANDBOX_FETCH=skip` keeps a fully local airlock from attempting network access.
- [x] #7 Issue #55 is referenced from the task and the task is marked Done only after local targeted validation; PR checks are watched after push and reported in handoff.
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Replace restore auto wiring with sandbox-owned cargo-home config plus per-env activation scripts; keep write/print/none explicit.
2. Extend restore tests to cover existing project Cargo config preservation and cargo-home activation files.
3. Teach generated launchers to fetch the selected sandbox branch only when no local ref is present, with a `PIXI_SANDBOX_FETCH=skip` opt-out.
4. Update CLI/help/docs/backlog references, run targeted validation, commit, and push the session branch.
<!-- SECTION:PLAN:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Default restore now writes Cargo source replacement to `.pixi-sandbox/cargo-home/config.toml` and installs per-environment Pixi/Conda activation hooks so `pixi run -- cargo ... --offline` sees the restored vendor tree without touching project `.cargo/config.toml`. Explicit `--cargo-config write` remains the destructive project-config mode, while `print` and `none` stay non-mutating and no longer print an offline Cargo build recipe. Generated restore launchers now fetch the selected sandbox branch when it is absent locally, with `PIXI_SANDBOX_FETCH=skip` for fully local airlocks. Tests and docs cover the issue #55 behaviours.
<!-- SECTION:FINAL_SUMMARY:END -->
