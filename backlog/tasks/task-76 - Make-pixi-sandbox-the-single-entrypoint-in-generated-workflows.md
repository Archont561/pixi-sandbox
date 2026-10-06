---
id: TASK-76
title: Make pixi-sandbox the single entrypoint in generated workflows
status: To Do
assignee: []
created_date: '2026-10-06 20:11'
updated_date: '2026-10-06 20:11'
labels:
  - ci
  - github-actions
  - generated-workflow
  - refactor
  - cli
dependencies:
  - TASK-36
  - TASK-71
references:
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox/src/generated/relock_workflow.rs
  - crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml
  - crates/xtask/src/repo_checks/workflow_shape.rs
  - .github/workflows/ci.yml
priority: high
type: chore
ordinal: 76000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The workflows that pixi-sandbox init writes into consumer repositories still carry their logic as embedded shell. The golden render at crates/pixi-sandbox/tests/fixtures/generated/publish-sandbox.yml is 420 lines, and 218 of its non-comment lines sit inside 9 multi-line `run:` blocks written in two dialects (10 `shell: bash`, 2 `shell: pwsh`), the largest a single 62-line block. None of that shell is type-checked, formatted, linted, or unit-tested, and the per-platform duplication is the same failure mode that killed the v0.3.6 release on macOS.

This repository already proves the shape we want. `.github/workflows/ci.yml` contains 6 `run:` lines and every one of them is a single `pixi run <task>`: the workflow decides when, the tool decides how. The pattern is proven inside the generated surface too, because the generated `relock.yml` renders 0 multi-line `run:` blocks and keeps its logic in `actions/github-script` steps.

Refactor crates/pixi-sandbox/src/generated/github_workflow.rs so pixi-sandbox itself is the entrypoint of the generated publisher, exactly as `pixi run <task>` is the entrypoint in ci.yml. Every step that carries logic becomes one `pixi-sandbox <verb>` invocation, and the absorbed behaviour moves into Rust where the existing test suite reviews it, both platforms share it, and it is versioned with the binary. The crisp end state is that check 9 no longer needs to exempt generated artifacts.

Sequenced after TASK-71 so the upgrade-delivery shell is not rewritten before its outstanding acceptance criterion has been proven on a real consumer run.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every logic-bearing step of the generated publisher is a single command: one `pixi-sandbox <verb>` invocation, or one `actions/github-script` step as relock.yml already does, replacing all 9 multi-line `run:` blocks in the current render.
- [ ] #2 The behaviour absorbed from those blocks lives in Rust behind named subcommands covered by the existing test suite instead of in YAML, including the pack-verify-publish wrapper with its per-step outcome recording, the pinned-download plus self-update plus regenerate plus check-owned-files upgrade sequence, and the upgrade-patch delivery with its step-summary and artifact fallback.
- [ ] #3 The generated publisher no longer forks on runner OS to express logic: each bash and pwsh twin collapses into one cross-platform step, and `shell: pwsh` survives only inside the bootstrap exception.
- [ ] #4 The steps that download the pixi-sandbox binary are the only exception, because they cannot use that binary as their own entrypoint; each stays at most one checksum-verified command per platform and is annotated in the render as the bootstrap exception.
- [ ] #5 Every subcommand and flag the generator emits exists in the binary that the same init run pins as PIXI_SANDBOX_VERSION, enforced by a test that fails when the generator emits a verb the current CLI does not accept.
- [ ] #6 Check 9 in crates/xtask/src/repo_checks/workflow_shape.rs stops exempting generated artifacts through GENERATED_MARKER, so the one-command-per-step rule applies to rendered consumer workflows, and any surviving exception is allowlisted by name with a recorded reason.
- [ ] #7 Golden fixtures under crates/pixi-sandbox/tests/fixtures/generated are regenerated, init output stays byte-identical to the committed renders, actionlint reports no findings, and non-comment embedded shell in publish-sandbox.yml falls from 218 lines to the bootstrap exception alone.
- [ ] #8 A real consumer repository runs the refactored publisher end to end on a released binary and reaches the same transport result as the pre-refactor lane, so the change is proven outside fixtures.
- [ ] #9 scripts/restore.sh is untouched and stays shell-only and version-agnostic per TASK-47 AC #9, because the airlock bootstrap has to work before any binary exists.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->
