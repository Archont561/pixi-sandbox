---
id: TASK-82
title: Move library-grade logic out of commands/pack.rs into promoted library modules
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-09 19:36'
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
- [x] #1 Vendor policy, tool fetch and verification, and branch-docs rendering move to library modules promoted in lib.rs, each with its own tests/ file; the inline tests in commands/pack.rs are removed.
- [x] #2 commands/pack.rs keeps argument handling and orchestration only; its production line count drops below 600 (currently 1,254).
- [x] #3 The LEGACY entry for commands/pack.rs is removed from crates/pixi-sandbox/tests/fixtures.rs in the same commit, and the guard test still passes.
- [x] #4 No behaviour change: pack's manifest, branch docs and vendor tree are byte-identical on the fixture project before and after, shown by a test or a recorded checksum.
- [x] #5 The promoted modules satisfy the coverage_guard route requirement under tests/.
- [x] #6 Work lands in slices, one module per commit, pack first; pixi run --frozen test does not drop below its last count.
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

Slice 4 (`branch_docs`): pure `render(Manifest, VendorInfo)` plus a separate writer preserves
both guide files, their write order and their error context. Nine integration cases cover the
three reference variants, unknown provenance, empty host sets, ordering, and both write failures.
`commands/pack.rs` is now **495 production lines**, down from **1,254**; its only functions are
`run` and the four planning/orchestration methods. The remaining manifest assembly, tool selection
and self-bin probe calls are phase wiring, not copies of library policy.

**Final AC evidence (all local; no push, PR or merge performed for TASK-82):**

1. `lib.rs` promotes `pack`, `vendor`, `tool_fetch` and `branch_docs`, with corresponding test
   roots: **20 + 14 + 25 + 9 = 68** cases. All eleven former inline pack tests have public
   library routes; the combined unsafe/duplicate selection test became two named cases.
2. `wc -l crates/pixi-sandbox/src/commands/pack.rs` is **495**, and no inline-test block remains.
3. LEGACY shrank **6 → 5** in the tool-fetch slice that removed the last inline cases;
   `production_sources_carry_no_inline_test_modules` passes, including its stale-entry check.
4. The original binary and the final binary produced four identical complete transport
   snapshots: Linux/no-vendor (**8 files**), Linux/loose + host requirements (**11**), Linux/tarballs
   + host requirements (**10**), Windows/loose (**11**). Only the owner-approved `created_at` and
   README Built timestamp were normalized; every other file byte and executable bit matched.
   The raw per-crate tar archive matches too, sha256
   `c8aebc22e0c9fafc0be24ecbaa40813bd9d90aec35dabb523c0257bbed42004f`.
   The committed references and three end-to-end comparison cases keep this proof executable.
5. `coverage_guard_requires_a_test_route_for_every_production_module` and the shared-helper
   guard pass; each promoted concern is called through its public API in its own tests/ file.
6. One extraction commit per module, pack first: `f0933ce9` (pack), `29962105` (vendor),
   `087321d0` (tool_fetch), then `refactor(pack): promote branch documentation rendering`.
   Every slice passed fmt/lint/test; suite counts rose **816 → 827 → 841 → 864 → 873**, with
   **1 skipped** throughout. Final gates: fmt --check, lint **11/11**, test **4/4**; the full
   suite is **873 passing / 1 skipped** (27 git + 209 core + 482 pixi-sandbox + 155 xtask).

`git diff --exit-code d8f14cf8` over all three lockfiles, core/git source, generated workflow
source, xtask source and the BDD feature files is empty: no dependency, schema, transport format,
workflow-generator or BDD-sentence change. AGENTS.md names the implemented library surface and
`xtask check-repository` passes. Status Done means all TASK-82 acceptance criteria are complete
locally; the new commits remain unpushed on the session branch, not merged into main.

**2026-10-09 integration update.** The owner has now explicitly authorized creating and
merging the combined TASK-87/TASK-82 PR. The no-push statements above describe the prior
local hand-off, not the current sanction boundary. All six criteria remain complete;
required PR checks and the merge-triggered main runs are to be verified before the final
integration report. No additional product or fixture changes are included in this closure.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
Pack is now thin wiring: **495 production lines** instead of 1,254. Reusable preflight and the
D13 file oracle live in `pack`; Cargo collision/storage policy in `vendor`; reviewed pin selection,
verified helper caching and embedding in `tool_fetch`; and pure guide rendering plus ordered
writes in `branch_docs`. All four are promoted through lib.rs and have public-API integration
roots. Shared process/filesystem helpers were moved once and re-exported, not duplicated.

All eleven inline tests left commands/pack.rs, and its LEGACY exception left in the same slice
as the last test. The four new roots run 68 cases, raising the suite **816 → 873 passing / 1
skipped**. Mutation proofs catch bypassing the unpack copy, weakening duplicate-source refusal,
bypassing download checksums, and guide-rendering drift; every mutation was reverted.

Four fixture transports match the pre-extraction binary byte-for-byte (raw tar included), apart
from the explicitly approved generated timestamp normalization. No new dependency, lockfile,
schema, rendered-output or generated-workflow change. Gates: fmt --check, lint 11/11, test 4/4,
repository consistency and architectural guards. Implemented in four module-sized commits on
`arena/49e36983-pixi-sandbox`; TASK-82 is locally complete and not pushed or merged.
<!-- SECTION:SUMMARY:END -->
