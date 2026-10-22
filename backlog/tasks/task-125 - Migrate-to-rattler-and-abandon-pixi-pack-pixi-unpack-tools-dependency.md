---
id: TASK-125
title: Migrate to rattler and abandon pixi-pack/pixi-unpack tools dependency
status: To Do
assignee: []
created_date: '2026-10-22'
updated_date: '2026-10-22'
labels:
  - rattler
  - tools
  - refactor
  - pack
  - restore
dependencies:
  - TASK-9
  - TASK-124
priority: high
type: feature
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Current transport relies on external helper binaries `pixi-pack` and `pixi-unpack` pinned via `crates/pixi-sandbox-core/assets/tools.lock.json` (embedded via `include_str!` in `tools_lock.rs:45`). The flow:

- `tools.lock.json` contains `pixi`, `pixi-pack`, `pixi-unpack` with `url_template` `{version}/{target}`, `version`, `platforms: { target, sha256, linkage }`
- `pixi-sandbox tools list` / `tools update` refreshes pins from same upstream (`github_repo()` parsed from template), verifies `sha256` cross-checked against `sha256.sum` when published, verifies `linkage:static` (ADR-4)
- `pack --fetch-tools` → `tool_fetch::fetch_tool()` downloads via `Download`, verifies, caches, embeds `pixi`+`pixi-unpack` into `payload/tools/<platform>/` with `ToolEntry { url, pinned_sha256 }`
- `pack` spawns `pixi-pack --directory-only` (ADR-2) per env, builds oracle by unpacking again with same pinned `pixi-unpack` (ADR-3)
- `restore` spawns embedded `pixi-unpack`

Goal: make `pixi-sandbox` depend on `rattler` crate directly (crates.io `rattler` 0.48.0, BSD-3-Clause, used by `pixi` and `rattler-build`, see https://github.com/conda/rattler) so only `pixi-sandbox` binary from conda-forge is needed – no `pixi global install pixi-pack`, no `--fetch-tools`.

This is the implementation of TASK-9 spike (v2 spike: replace pixi-pack/pixi-unpack subprocesses with rattler, currently Done as spike with 2/2 AC). ADR-2, ADR-3, ADR-4, ADR-24 currently mandate retaining helpers; they need revisiting.

**Work:**
- Evaluate `rattler` main crate + subcrates `rattler_conda_types`, `rattler_package_streaming`, `rattler_cache`, `rattler_networking`, `rattler_solve` – choose minimal feature set (no reqwest if possible, reuse ureq/rustls)
- Replace `pack` subprocess with rattler library: resolve `pixi.lock` envs, install packages into `payload/envs/<name>/` directly, handle PyPI wheels (uv reinstall path that `pixi-pack` currently handles)
- Replace `restore` subprocess with rattler: install from payload blobs, write markers
- Remove `tools_lock.rs`, `assets/tools.lock.json`, `tool_fetch.rs`, `tools_update.rs`, `commands/tools/*`, `--fetch-tools`, `--tools-lock`, `--tools-cache` flags
- Update `manifest.rs` ToolEntry – no longer needs `pixi`/`pixi-unpack` with url/pinned_sha256
- Update `SandboxConfig` validation that currently requires `pixi`, `pixi-pack`, `pixi-unpack` in tools lock
- Update `xtask` and generated workflow templates that reference fetch-tools

Reference: https://crates.io/crates/rattler, https://github.com/conda/rattler, TASK-9, ADR-2/3/4/24
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `rattler` crate evaluated: minimal feature set documented, `cargo tree -i reqwest` still empty if possible, MSRV impact assessed (rattler 0.48 declares 1.88+ already satisfied by TASK-92)
- [ ] #2 `pack` no longer spawns `pixi-pack`: implements packing via rattler library, produces byte-identical `payload/envs/<name>/` for existing fixtures, PyPI wheel handling preserved (uv reinstall or rattler equivalent)
- [ ] #3 `restore` no longer spawns `pixi-unpack`: implements unpack via rattler, restores same file list as before, markers written correctly
- [ ] #4 `tools.lock.json`, `tools_lock.rs`, `tool_fetch.rs`, `tools_update.rs`, `tools` CLI subcommand, `--fetch-tools` / `--tools-lock` / `--tools-cache` flags removed; `pixi-sandbox` binary from conda-forge is sufficient, no user install of pixi-pack needed
- [ ] #5 `manifest.json` no longer embeds `pixi`/`pixi-unpack` tools with url/pinned_sha256, or embeds only when explicitly requested; transport size budgets (TASK-72) updated
- [ ] #6 Full test suite passes: `pixi run --frozen test`, `test-payload-roundtrip`, `consumer-proof`, airlock restore; docs updated with new transport mechanism
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

## Dependency scaffold follow-up (2026-10-22, PR #186)

The unused workspace/CLI `rattler` dependency was removed while repairing CI. No Rust
code imported it; pack/restore still use the existing pinned tool subprocesses. The scaffold
activated AWS-LC (airlock's debug link failed on `__isoc23_sscanf`) and vulnerable archive/YAML
packages without implementing this task. Reintroduce a maintained, minimal-feature dependency
with the actual migration, feature-tree/MSRV analysis, and all acceptance tests above. This
removal does not complete TASK-125 or change the existing transport format/tool behavior.
