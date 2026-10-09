---
id: TASK-82
title: Move library-grade logic out of commands/pack.rs into promoted library modules
status: In Progress
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-09'
labels:
  - architecture
  - pack
dependencies: []
priority: medium
type: enhancement
ordinal: 82000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding, architecture. AGENTS.md's repo map says `crates/pixi-sandbox/src/commands/*` is "thin wiring" and that logic belongs in the library, where it can be tested from tests/. Measured: commands/ holds 4723 production lines across 12 files (pack.rs 1,254, restore.rs 945, doctor.rs 699, init.rs 624). Inside pack.rs, much of it is library-grade policy: validate_vendorable_lockfile, reject_duplicate_crate_sources, crate_directories and vendor_tree (cargo vendoring policy, roughly 250 lines); fetch_tool, tools_cache, reported_version and embed_tool (tool pinning and verification); write_branch_docs (186 lines of rendered documentation); and build_files_oracle. commands/ is binary-private, so that logic is only testable black-box through the CLI. pack.rs is one of six LEGACY files that still carry inline tests (crates/pixi-sandbox/tests/fixtures.rs:356).

This is distinct from TASK-57, which extracted steps inside run(). It must be sequenced with TASK-8 (duplicate crate sources) and TASK-54 (self-bin refusal), which both change pack.rs.

Seam to agree before tests are written: the public functions of the promoted modules, one module per concern (for example vendor, tool_fetch, branch_docs), exposed from lib.rs as pub mod and tested from tests/<module>.rs.

Non-goals: no change to the transport format, manifest schema or rendered output; no new dependency; no rewrite of render_github_workflow (task-76).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Vendor policy, tool fetch and verification, and branch-docs rendering move to library modules promoted in lib.rs, each with its own tests/ file; the inline tests in commands/pack.rs are removed.
- [ ] #2 commands/pack.rs keeps argument handling and orchestration only; its production line count drops below 600 (currently 1,254).
- [ ] #3 The LEGACY entry for commands/pack.rs is removed from crates/pixi-sandbox/tests/fixtures.rs in the same commit, and the guard test still passes.
- [ ] #4 No behaviour change: pack's manifest, branch docs and vendor tree are byte-identical on the fixture project before and after, shown by a test or a recorded checksum.
- [ ] #5 The promoted modules satisfy the coverage_guard route requirement under tests/.
- [ ] #6 Work lands in slices, one module per commit, pack first; pixi run --frozen test does not drop below its last count.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-09 — the owner approved four public boundaries before new tests: `pack` (preflight and
per-file oracle), `vendor` (lockfile policy and the vendor tree), `tool_fetch` (pinned fetch,
cache verification and embedding), and `branch_docs` (rendering and writing). One extraction
commit per module, `pack` first; no new dependency, transport/schema/rendering change, or
workflow-generator rewrite. Push/PR/merge remain reserved.

AC#4's byte comparison has one explicit owner-approved exception: only `manifest.created_at`
and the corresponding README `Built … from commit` timestamp are normalized. Every other
byte is compared against pre-extraction output. References were captured with the original
binary at `d8f14cf8`, sha256 `a73cdd6a540e51f6755f9ef8d80b9a9c1a239650615685ff96d57e5bb9e68fa9`,
from temporary copies of the demo project and synthetic external-tool programs, not the
repository as a project. Linux/no-vendor, Linux/loose with host requirements, and Windows/loose
references are committed under `tests/fixtures/pack-reference/`; Linux/tarballs is retained
for a raw before/after comparison on this runner because tar headers carry native UID/GID.

Slice 1 (`pack`): public layout preflight, host-requirement resolution and file-oracle assembly;
shared process/filesystem primitives moved once and re-exported by commands. Nine old inline
cases moved to `tests/pack.rs` (the combined environment-name case became two named cases),
plus shard-limit/failure-path and reference tests. Deliberate mutations were seen red and
reverted: bypassing the unpack copy changes the source pack, and changing the README headline
fails every golden case. `tests/pack.rs`: 20 passing; full suite: **827 passing / 1 skipped**;
fmt, lint (11/11), and test (4/4) green.

Slice 2 (`vendor`): duplicate-source preflight and both storage modes now live in the library;
`VendorMode` is independent of clap and the process boundary accepts explicit cargo/rustc paths.
Fourteen integration cases cover same-source repeats, different versions, path members,
malformed and missing locks, refusal before output, loose bytes, tar contents and cleanup,
empty output, and honest unknown toolchain labels. Weakening the collision filter was seen
red and reverted. All four pre/post transports match, including the raw tar archive
sha256 `c8aebc22e0c9fafc0be24ecbaa40813bd9d90aec35dabb523c0257bbed42004f`; only the two
approved timestamp locations were normalized. Full suite: **841 passing / 1 skipped**;
fmt, lint (11/11), and test (4/4) green.

Slice 3 (`tool_fetch`): reviewed pin selection, cache placement, streaming download verification,
reported versions and embedding now live in the library. The HTTP adapter is a thin byte-stream
boundary; no pin policy is delegated to the fake. HOME/USERPROFILE selection stays in the CLI.
Twenty-five integration cases cover verified hits, corrupt-cache replacement, refused/failed/
interrupted downloads, cleanup, absent pins, version mismatches, Windows names, override paths,
cache selection, embedded metadata, oversize/dynamic/pin refusals and version diagnostics.
Bypassing the checksum check was seen red and reverted. The last two inline pack tests moved
here and `commands/pack.rs` left LEGACY in this same commit; both architectural guards pass.
Reference projects have an unborn local Git repository so even a caller's checkout-local TMPDIR
cannot leak an ancestor commit into provenance. The expected reference's build-version inputs
follow the package version; actual outputs are normalized only at the two approved timestamp
sites. Full suite: **864 passing / 1 skipped**; fmt, lint (11/11), and test (4/4) green.
<!-- SECTION:NOTES:END -->
