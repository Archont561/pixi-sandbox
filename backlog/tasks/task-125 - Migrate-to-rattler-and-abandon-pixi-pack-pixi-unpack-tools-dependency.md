---
id: TASK-125
title: Migrate to rattler and abandon pixi-pack/pixi-unpack tools dependency
status: In Progress
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

## Implementation session (2026-10-22)

- User selected TASK-125 and approved the CLI pack/restore/unpack seams plus the public
  local package-install boundary, using real Conda/wheel fixtures. Baseline after restoring
  source commit 890158f: 1,188 passing / 2 skipped.
- User explicitly approved narrow runtime unmaintained-YAML advisory exceptions and the
  specific bzip2 license required by rattler. This does not authorize ignoring vulnerabilities.
  Apply advisory-specific reasons only when the dependency actually ships.
- User explicitly authorized a draft PR on this branch and a temporary connected source-
  provisioning job. It verifies original .crate archives against registry lock checksums,
  stores them as unreferenced GitHub blobs, and lets the airlocked developer retrieve them
  through authenticated GitHub API. No generated dependency tree enters source history;
  no release, merge or published transport is authorized by this provisioning approval.
- Candidate minimal graph: rattler 0.48.7, conda_types 0.52.1, package_streaming 0.27.5
  with TLS features off; rattler_lock 0.32.6 for the real lock parser. Use direct local
  extraction/linking rather than the installer's network-capable fallback. Reuse ureq on
  the connected packing side; reqwest remains an unavoidable transitive dependency of
  rattler even with no TLS enabled, so AC#1's empty-reqwest goal cannot be promised.
- Wheel evaluation: uv-install-wheel 0.0.57 is the low-level installer used by the existing
  pixi-pack 0.7.11, without uv's HTTP/resolution/build layers. It requires Rust 1.94, unlike
  rattler's 1.88 floor. Any adoption must explicitly document/validate this MSRV impact,
  not retain a false rust-version claim. The installed development toolchain is 1.98.1.
- Existing helper implementation remains intact until replacement behavior is proven. This
  temporary preparation is not completion of any migration acceptance criterion.

### Dependency evaluation correction

Connected source resolution shows rattler 0.48.7 / package_streaming 0.27.5 force zip 8.6.0;
that line has no satisfactory patched candidate for the reported advisories. It is not an
acceptable product dependency even with TLS disabled. The provisioning candidate therefore
moves to rattler 0.54.2, conda_types 0.57.1, lock 0.34.8 and package_streaming 0.27.15,
whose ZIP dependency uses the patched 9.x line. No ZIP advisory exception is authorized.
The first real-fixture CLI test was run red: `unpack` with PATH empty fails because the
current implementation requires pixi-unpack. Its red log is retained locally.

### Native bare-pack checkpoint

Implemented `native_pack::{verify, install}` and bare format-1 `unpack`'s default library path:
local Conda extraction/linking plus UV wheel overlay, no helper or interpreter process. Explicit
helper override and existing transport/restore behavior remain unchanged. Output replacement is
staged and rolls back if promotion fails. Fifteen real-fixture CLI/public-boundary tests pass,
including independent failing tests for corrupt-input preservation, inconsistent metadata,
unsafe wheel paths, and unsafe environment names. Full workspace runs passed with six and then
fifteen new tests. No migration AC is marked complete on this partial implementation.

Owner subsequently approved Rust 1.94 as the new minimum and exact-version 0BSD license
exceptions for mailparse 0.16.1 / quoted_printable 0.5.2. The selected runtime graph is rattler
0.54.2 / types 0.57.1 / streaming 0.27.15 (defaults off), UV 0.0.57 and locked ZIP 9.0.5.
reqwest remains transitive; host normal-tree inspection finds no AWS-LC edge. A Rust 1.94
workspace/all-targets CI gate is added; its actual result, not the newer local compiler, must
validate the floor. Remaining implementation and compatibility evidence are enumerated in
CONTEXT.md's dated TASK-125 checkpoint. PR #187 remains draft; TASK-125 remains In Progress.
