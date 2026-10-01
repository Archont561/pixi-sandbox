---
id: TASK-34
title: Add the Archont561 prefix namespace during init
status: To Do
assignee: []
created_date: '2026-10-01 17:30'
labels:
  - init
  - pixi
  - configuration
  - channels
milestone: m-0
dependencies:
  - TASK-21
  - TASK-22
references:
  - crates/pixi-sandbox/src/commands/init.rs
  - crates/pixi-sandbox/tests/cli.rs
  - docs/src/content/docs/reference/configuration.mdx
  - docs/src/content/docs/guides/using-in-your-project.mdx
priority: high
type: enhancement
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Make `pixi-sandbox init` add the fixed Archont561 prefix.dev namespace root to the project's Pixi channel configuration. Init should register `https://prefix.dev/archont561`, not only the package-specific `https://prefix.dev/archont561/pixi-sandbox` channel used to install the CLI. This makes other channels and packages published under the Archont561 namespace easier for users to consume while preserving the project's existing channels and channel priority. The namespace comes from pixi-sandbox's publisher, not from the owner of the user's Git repository.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `pixi-sandbox init` adds `https://prefix.dev/archont561` to the project-local Pixi channel configuration regardless of the owner or host of the user's Git repository
- [ ] #2 The configured channel is the Archont561 namespace root rather than the package-specific `https://prefix.dev/archont561/pixi-sandbox` channel, so users can consume other Archont561 channels and packages
- [ ] #3 Init preserves all existing channels and their order, inserts the Archont561 namespace deterministically, and is idempotent across repeated runs
- [ ] #4 An existing equivalent Archont561 namespace entry is recognized without duplication, including normalized URL spelling and casing where supported by prefix.dev
- [ ] #5 Init updates configuration through a structured Pixi/TOML mechanism and produces an actionable error if the project configuration cannot be safely updated
- [ ] #6 Tests cover existing channel preservation, deterministic insertion, equivalent-entry normalization, duplicate avoidance, repeated init, and a user repository owned by an unrelated organization
- [ ] #7 Documentation explains that init adds the Archont561 namespace for project dependencies and distinguishes it from the package-specific channel used to install `pixi-sandbox`
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Define the canonical Archont561 namespace URL once and have init add it to project-local Pixi configuration through a structured TOML edit or Pixi's supported configuration interface rather than text appending. Retain existing channel priority and comments where practical, normalize equivalent entries before insertion, add fixture-backed CLI tests, and document the namespace-root versus package-specific channel distinction. Do not derive this namespace from the user's Git remote.
<!-- SECTION:PLAN:END -->
