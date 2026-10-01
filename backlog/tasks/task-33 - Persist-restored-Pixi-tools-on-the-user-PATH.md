---
id: TASK-33
title: Persist restored Pixi tools on the user PATH
status: Done
assignee: []
created_date: '2026-10-01 16:30'
updated_date: '2026-10-01 17:55'
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
documentation:
  - backlog/docs/specifications/transport-and-restore/doc-2
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
- [x] #1 A successful restore registers both `pixi` and `pixi-sandbox` in a persistent per-user bin location by default, only after branch and restored-tree verification have passed
- [x] #2 POSIX uses managed links or launchers to the restored tools and Windows uses an equivalent user-level mechanism that does not require administrator privileges; the bin location is configurable for airlock policy
- [x] #3 The user bin directory is added idempotently to the detected shell's persistent PATH (and Windows user PATH), with a clear notice that an already-running parent shell cannot be mutated and a new shell sees the change
- [x] #4 Re-restoring the same or another project safely retargets only pixi-sandbox-managed entries, refuses unrelated existing `pixi` or `pixi-sandbox` commands unless explicitly forced, and documents that the most recently registered restore becomes the user-level tool source
- [x] #5 An explicit opt-out disables all home/profile changes for CI, shared accounts, and locked-down airlocks without changing restore verification or project output
- [x] #6 Fixture-backed tests use isolated HOME, profile, PATH, and user-bin directories; they cover first install, idempotent repeat, managed retarget, collision refusal/force, opt-out, missing HOME, and discovery of `pixi sandbox`
- [x] #7 Documentation distinguishes persistent tool discovery from full environment activation: `pixi` and `pixi sandbox` work directly, environment commands should use `pixi run`, while `.pixi/sandbox-env.sh` remains available when direct `cargo`, `rustc`, or `bun` commands are wanted
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Model user-tool registration as a filesystem/profile service with an explicit root so tests never touch the developer's home. Register both tool names because Pixi extension discovery is PATH-based. Use recognizable managed entries rather than copying untracked binaries, update persistent PATH with a marker block or native user-environment API, and perform the operation only after restore verification. Add force and opt-out controls to the Rust CLI and generated launchers, then update airlock documentation and CI callers to choose the intended policy explicitly.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
New service module `crates/pixi-sandbox/src/user_tools.rs`: every root (bin dir, profile file) is an explicit input — the module never reads the environment, so tests pass tempdirs and no code path can reach a developer's real home. POSIX launchers are `#!/bin/sh` scripts carrying the `managed by pixi-sandbox` marker that `exec` the manifest-verified copy under `.pixi/tools/<platform>/` (decision-2 §4.1: the `tools` entry is the canonical executable, never an environment-embedded copy — symlinks were rejected because Windows needs no-privilege creation and copies drift from the verified bytes); Windows uses `.cmd` launchers plus the HKCU user `Path` via a PowerShell script that reads the raw value with `DoNotExpandEnvironmentNames` and refuses rather than baking `%…%` references open. The profile edit is a marker-delimited PATH block (`case`-guarded so sourcing twice adds one entry) replaced as a unit: idempotent repeat, changed `--user-bin`, and stale/duplicated blocks all collapse to one block. Shell detection: bash prefers an existing `~/.bash_profile`, zsh uses `~/.zshrc`, fish and undetectable shells fall back to `~/.profile` with a notice. Default bin: `~/.local/bin`; Windows `%USERPROFILE%\.pixi-sandbox\bin` (not `~/.pixi/bin`, which belongs to `pixi global install` trampolines).

`restore` gained the gate the task demands: after installing everything it now runs the restored-tree oracle check itself (`verify::verify_restored`, the same code `doctor --verify-restored` runs) and only then registers — a mismatch fails the restore before the work dir is cleaned (evidence rule), and schema-1 envs proceed on branch verification alone with an `unverifiable` notice, per D13. CLI: `--user-tools register|skip` (default register, env `PIXI_SANDBOX_USER_TOOLS`, `overrides_with` itself so the launchers' explicit flag yields to an operator's later one), `--user-bin DIR` (env `PIXI_SANDBOX_USER_BIN`), and `--force` now also covers unmanaged collisions. Refusals are hard errors that state the restore itself completed and name the remedies; a missing HOME is a hard error naming `--user-tools skip` (exit 0 means "registered, or you opted out"). The generated `restore.sh`/`restore.ps1`, `scripts/restore.sh` (Bash 3.2-safe), and the airlock workflow's CI restore all select the policy explicitly — `register` for the human-facing launchers (overridable via env or a trailing argument), `--user-tools skip` on the shared CI runner.

Tests: 12 unit tests in the service module (launcher shape, first/repeat/retarget, collision refusal/force, block collapse, unsafe-bin refusal, PS script shape, shell detection, sourcing idempotency) plus 10 integration tests in `tests/user_tools.rs` (first install, `pixi sandbox` PATH discovery through the fixture's pixi stub which now resolves `pixi-<command>` through PATH like the real pixi, idempotent repeat, cross-project retarget, collision refusal/force, opt-out, env-var policy + custom `--user-bin`, missing HOME, duplicate-flag override, and a full no-stub run of the real generated launcher against a branch carrying the real binary). Every restoring test — including the pre-existing ones in `cli.rs`/`e2e.rs` — now isolates HOME/USERPROFILE/SHELL in tempdirs; `restore_materialises_the_fixture_vendor_tree…` was simplified to use the fixture's genuine unpacker because its hand-rolled fake can no longer satisfy the internal oracle check (the check caught that on the first run). Suite: 206 passing, 1 skipped (was 184+1).

Docs: `restore.mdx` gained the persistent-discovery vs full-activation distinction (registration defaults, retarget semantics, the running-shell notice, `--user-tools skip`/`--user-bin`), the CLI reference documents the new flags and the updated restore order, README's airlock section shows both PATH kinds, doc-2 §4 records the new steps 8–11 (sandbox-env, restored-tree verify, cleanup, registration), and AGENTS.md/README repo-map lines mention the explicit policy.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
A verified restore now leaves `pixi` and `pixi sandbox` working in any new shell: managed launchers in a configurable per-user bin directory, a managed PATH block in the detected shell's profile (registry-backed on Windows, no admin rights), both pointing at the manifest-verified tool copies. Restore itself verifies the tree it produced against the manifest's per-file oracle before registering, refuses unmanaged collisions unless forced, retargets managed entries across projects with the most-recent-restore rule stated out loud, and offers a hard `--user-tools skip` opt-out that CI and the generated launchers select explicitly.
<!-- SECTION:SUMMARY:END -->
