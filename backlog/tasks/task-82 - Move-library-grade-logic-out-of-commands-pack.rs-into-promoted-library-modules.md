---
id: TASK-82
title: Move library-grade logic out of commands/pack.rs into promoted library modules
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
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
