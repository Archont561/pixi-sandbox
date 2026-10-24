---
id: TASK-125
title: Migrate to rattler and abandon pixi-pack/pixi-unpack tools dependency
status: Done
assignee: []
created_date: '2026-10-22'
updated_date: '2026-10-25'
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

Historical starting point below; ADR-31 and the accepted-scope reconciliation in
`backlog/docs/plans/task-125-release-readiness.md` supersede helper acquisition and directory-layout assumptions.

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
- [x] #1 Approved runtime graph evaluated/documented: rattler defaults/TLS off, unavoidable transitive reqwest disclosed, no host-normal AWS-LC edge, scoped dependency-policy exceptions and actual Rust 1.94 CI validation (ADR-31).
- [x] #2 New `pack` uses native rattler/UV with v7 lock preflight; original Conda archive/wheel bytes remain unchanged and relocated/oracle-verified installed behavior is preserved (ADR-31 replaces helper-directory byte identity).
- [x] #3 Format-1 restore/unpack under schemas 1–3 install natively with markers and staged-oracle verification; explicit override and legacy opaque reader remain only for compatibility (ADR-31).
- [x] #4 Environment-helper catalogue/updater/modules, `tools` CLI and helper-management flags removed; no pixi-pack/pixi-unpack install required. Canonical ecosystem package or standalone binary suffices; independent payload builder remains (ADR-28/31).
- [x] #5 New schema-3 manifests have no implicit helpers; local Pixi/bootstrap selection is explicit with actual byte hashes and unchanged size/linkage ceilings. Published native manifest has only pixi/pixi-sandbox; historical size docs are labelled, not treated as current measurements.
- [x] #6 Full test suite passes: `pixi run --frozen test`, `test-payload-roundtrip`, `consumer-proof`, airlock restore; docs updated with new transport mechanism
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
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


### Native producer and restore continuation

PR #187 was merged; it was a partial checkpoint, not task completion. The subsequent implementation
now packs genuine compact version-7 locks natively, emits schema 3, restores format-1 packs under
schemas 1–3 without helpers, and checks the staged file oracle before rollback-protected promotion.
A hash-valid but incorrect oracle cannot retire an existing restore or raw-unpack output. Explicit
local tools now record their actual byte SHA256 even without upstream catalogue provenance.

Removed the tools CLI/catalogue/updater and helper flags, source Pixi helper dependencies, and
approved temporary provisioning files. The independent payload builder remains on-demand under
`--build-cache`. Publisher/consumer callers explicitly select Pixi; the airlock release adapter
alone supports the immutable pre-migration release CLI. Opaque legacy transports retain their
recorded unpacker compatibility reader; that is not a new-producer dependency (ADR-31).

The owner approved obsolete helper-interface/synthetic producer test retirement: previous baseline
1,203 passes / 2 skipped; latest full workspace inventory 1,154 passes / 2 skipped, with 26 native
CLI/library cases. Original Conda archive and wheel bytes are independently compared unchanged.
Full lint passed 11/11; fmt passed. Final full-task test/lint and connected payload, built-artifact
consumer and airlock evidence remain pending, so AC/DoD completion is deliberately not asserted.

Final local gate reruns: `pixi run --frozen fmt` passed; `pixi run --frozen lint` passed 11/11;
`pixi run --frozen test` passed all four package tasks (1,154 tests / 2 intentionally skipped).
The last oracle-option refactor also passed pack/native/embedding checks. Connected platform
proofs remain outstanding; task stays In Progress until their actual outcomes are known.


2026-10-22 publisher incident follow-up: post-merge publisher 39813564308 compiled successfully
but pack rejected valid v7 records without `build_number`. Public fixture-backed pack/restore/unpack
reproduced that exact error before fixing the v7 trailing-digit/default-zero rule. Explicit values
remain authoritative; omitted noarch kind follows v7 inference too. Six genuine local Conda archive
cases validate the resulting channel metadata (numeric/nonzero/non-trailing/default-zero builds,
generic/Python kinds). Focused native suite: 36 pass. No new lock-format dependency or policy
exception, and no production publication/release was triggered. Live publisher success and the
existing native release obligations remain unproven; this does not close TASK-125.


### 2026-10-22 — Live evidence reconciled; documentation delivery in review

The runtime/connected evidence listed as pending in the historical notes above is now available:
CI 39824086495 (including payload/offline consumer, Rust 1.94 and Windows), publisher 39824086542,
five-platform artifacts-only release 39826041383 including fresh Linux binary consumer proof,
and Pages 39825855879 all succeeded at merge 0d15c4d8. Auto-release 39828722470 was a successful
**dry run**: no commit/tag/push or production release. Published schema-3 manifest source is
0d15c4d with only pixi/pixi-sandbox tools. This session also restored it with zero blob/file failures.

AC1–5 are reconciled to the owner-approved ADR-31 scope above; the historical description is not
an instruction to reintroduce helpers or promise an empty reqwest graph. AC6 and local DoD1–3
are now checked after successful full lint, ordinary and instrumented
1,186-pass/2-skipped suites, doctests and the complete docs build/quality checks on this cleanup.
The public CLI diagnostic regression was observed red then green; no installer algorithm changed.
No fresh native runtime failure is known. New-native offline proof is Linux, while all five platforms passed packaging
and smoke; those are deliberately not conflated. See the release-readiness ledger for links,
current counts, version recommendation and scoped limitations. Status stays In Progress until
this final documentation delivery is merged; previous "pending runtime evidence" notes are history.

The task remains In Progress solely for delivery/review/merge. The new cleanup still needs normal
PR CI before merge; the linked connected/release evidence above belongs to source 0d15c4d8,
not to an unbuilt claim about every future commit. Controlled billing evidence remains with
TASK126/127 and is not needed to reinterpret these successful runtime gates.

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-25 closure: the delivery/merge this task waited on has shipped. The native transport is
the released format as of v0.8.0 — published manifests declare schema 4 with only
`pixi`/`pixi-sandbox` tools (no pixi-pack/pixi-unpack), `pixi-sandbox-unpack` no longer exists,
and the packed transport restores through rattler/UV native install paths (ADR-31). All six
acceptance criteria and all three DoD items are checked above; the connected evidence named in
the historical notes (CI `39824086495`, publisher `39824086542`, five-platform artifacts-only
release `39826041383`, Pages `39825855879`) is merged history, and subsequent airlock runs
exercise the native restore on every PR plus the weekly scheduled full offline compile.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
TASK-125 delivered the rattler migration: pack resolves and installs Conda archives plus UV
wheels through native libraries instead of pixi-pack/pixi-unpack subprocesses, restore and the
format-1 compatibility reader need no helper binaries, the tools CLI/catalogue/updater and all
helper-management flags are removed, and manifests carry explicit, hash-verified tool
selections. It shipped incrementally through PR #187 and the following native-transport series,
released as v0.8.0 under ADR-31's accepted scope (rattler 0.54.2 line, TLS off; reqwest
accepted as unavoidable transitive; Rust 1.94 floor; scoped 0BSD/mailparse license
exceptions). Closing now that every criterion has merged, released evidence behind it.
<!-- SECTION:FINAL_SUMMARY:END -->
