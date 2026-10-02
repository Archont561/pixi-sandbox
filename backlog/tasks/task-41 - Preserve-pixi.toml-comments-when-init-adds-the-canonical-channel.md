---
id: TASK-41
title: Preserve pixi.toml comments when init adds the canonical channel
status: To Do
assignee: []
created_date: '2026-10-02 09:51'
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
- [ ] #1 Running pixi-sandbox init on a commented pixi.toml preserves every unrelated byte, comment, table ordering choice, and trailing newline while adding the canonical channel.
- [ ] #2 Existing channel forms covered by init remain correctly recognised and normalised, and repeated init is byte-for-byte idempotent.
- [ ] #3 Tests cover inline and standalone comments around the channels array, multiline and single-line arrays, no channels key, and an already-canonical manifest.
- [ ] #4 Malformed manifests and unsafe channel configurations retain their current explicit errors; the implementation never silently falls back to destructive whole-file serialisation.
- [ ] #5 Pixi-only fmt, lint, and test gates pass, and the task is completed in house format.
<!-- AC:END -->
