---
id: ADR-7
title: "Verify before write; a join is atomic; never a half-populated environment"
status: accepted
tags:
  - decision
---

# ADR-7 — Verify before write; a join is atomic; never a half-populated environment

**Decision.** No file reaches the user's tree before its sha256 matches the manifest.
`join_parts` writes a sibling temp file, verifies the parts *and* the reassembled blob, and
renames into place only then; on any failure the temp file is removed. `restore` stages whole
environments in the work dir and renames them in. A staged environment points at the staging
path until the moment before the rename, so every valid-UTF-8, NUL-free text file has that path
replaced by the final prefix first; a NUL byte means a fixed-width binary payload, which is
left exactly as it is. Once everything is in place the scratch this restore created is removed,
and a failed restore keeps it.

**Why.** A half-written environment is worse than a failed restore: pixi and cargo will happily
use a prefix that is 99 % correct and fail hours later in a way nobody connects to the
transport. Failure must be loud, complete and non-destructive. The same reasoning runs backwards
for scratch: an environment that still names the staging path works only until the work dir goes
away, so a leftover `.restore-work` is a silent time bomb (#18), while leftover scratch from a
*failed* restore is the evidence an operator needs.

**Evidence ✅.** The property test plus two concrete bugs caught by it (parts resolved relative
to the destination; a half-written destination left behind after a corrupted part) — both are
now covered by tests in `crates/pixi-sandbox-core/tests/shard.rs` and by the tamper case in
`research/EVIDENCE.md` §4.

**What would change it.** Nothing we can think of; this is the invariant the whole tool exists
to provide.
