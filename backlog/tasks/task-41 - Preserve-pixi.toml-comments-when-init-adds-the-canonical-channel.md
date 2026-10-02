---
id: TASK-41
title: Preserve pixi.toml comments when init adds the canonical channel
status: Done
assignee: []
created_date: '2026-10-02 09:51'
updated_date: '2026-10-02 10:38'
labels:
  - init
  - cli
  - tooling
dependencies: []
references:
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/tests/cli.rs
documentation:
  - CONTEXT.md
priority: high
type: bug
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
pixi-sandbox init currently parses pixi.toml into toml::Table and rewrites it with toml::to_string_pretty when adding the canonical Archont561 channel. That turns a surgical one-line change into a whole-file rewrite, removes comments, and reorders hand-maintained tables. Preserve consumer formatting and comments while adding or normalising the channel, preferably with toml_edit or a narrowly scoped textual edit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Running pixi-sandbox init on a commented pixi.toml preserves every unrelated byte, comment, table ordering choice, and trailing newline while adding the canonical channel.
- [x] #2 Existing channel forms covered by init remain correctly recognised and normalised, and repeated init is byte-for-byte idempotent.
- [x] #3 Tests cover inline and standalone comments around the channels array, multiline and single-line arrays, no channels key, and an already-canonical manifest.
- [x] #4 Malformed manifests and unsafe channel configurations retain their current explicit errors; the implementation never silently falls back to destructive whole-file serialisation.
- [x] #5 Pixi-only fmt, lint, and test gates pass, and the task is completed in house format.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Replaced `toml::Table` serialisation in `ensure_archont561_channel` with a `toml_edit::DocumentMut` mutation that changes only `workspace.channels`. The multiline append path keeps a final entry's comment attached to that entry while retaining the array's closing whitespace and indentation. `toml_edit` was already present in the restored vendored dependency set; the lockfile now records it as a direct dependency.

Added byte-exact CLI coverage for commented multiline and single-line arrays, a missing channels key, normalised canonical channels, repeated-init idempotence, malformed TOML, and unsafe channel shapes. `pixi run --frozen fmt`, `pixi run --frozen lint`, and `pixi run --frozen test` pass (289 passed / 1 skipped).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
`pixi-sandbox init` now adds `https://prefix.dev/archont561` without rewriting consumer-maintained `pixi.toml` files. Comments, unrelated bytes, table order, and trailing newlines remain intact; explicit validation errors remain non-destructive.
<!-- SECTION:SUMMARY:END -->
