---
id: TASK-12
title: >-
  Fix init github: the generated publish workflow cannot run (matrix shape +
  relative action uses)
status: To Do
assignee: []
created_date: '2026-09-29 22:15'
updated_date: '2026-09-29 22:15'
labels:
  - ci
  - init
  - bug
dependencies: []
priority: high
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
GitHub issue #37: `pixi-sandbox init github` (v0.3.1) generates a publish-sandbox.yml with two independent GitHub Actions failures. Both are still present on main at 26e1349, so this is a live bug in the generator, not only in the cut release.

1. Matrix shape. github_workflow() in crates/pixi-sandbox/src/commands/init.rs (line ~130) emits `matrix: ${{ fromJSON(needs.plan.outputs.matrix).include }}`, which hands strategy.matrix the include array. Actions expects an object there, so the plan job succeeds and the publish matrix job is never instantiated. Reproduced in Archont561/qgis-rs run 36636577488. The generator must nest it: `matrix:` then `include: ${{ fromJSON(needs.plan.outputs.matrix).include }}`. The issue cites commit e1b3755 as carrying the fix, but that object does not exist in this repository, so it has to be written here, not cherry-picked.

2. Relative uses in the remote action shims. setup/action.yml and publish/action.yml are thin shims whose only step is `uses: ../action.yml`. Actions does not resolve a relative uses from a remote action, so a generated workflow calling Archont561/pixi-sandbox/setup@<sha> fails during job setup with "Expected format {org}/{repo}[/path]@ref. Actual '../action.yml'". Reproduced in qgis-rs run 36636090062. Two candidate fixes: make setup/ and publish/ self-contained composite actions, or have init github reference the repository-root action at the pinned SHA and pass subpath: setup / subpath: publish.

Checked from the restored airlock before filing: `pixi-sandbox plan --config .pixi-sandbox.toml --json` emits {"schema":1,"include":[{bundle,environments,platform,runner,branch,cargo_vendor}]}, and the generated workflow reads matrix.runner / matrix.environments / matrix.platform / matrix.branch / matrix.cargo_vendor. The plan payload is therefore fine; only the two wrappers above are broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 init github emits an object-shaped matrix (include: nested under matrix:) and a test over the generated workflow text asserts it
- [ ] #2 No published action path contains a relative uses of ../action.yml: setup/ and publish/ resolve from a remote ref, and a test or a scripts/lint-repo-consistency.sh check fails if one reappears
- [ ] #3 A regression test pins plan --json against the generated workflow: every matrix.<key> the workflow reads exists in the include entries
- [ ] #4 actionlint validates the workflow that init github generates, not only the workflows committed in this repo
- [ ] #5 pixi run lint and pixi run test pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Source: https://github.com/Archont561/pixi-sandbox/issues/37 (opened 2026-09-29). Repro runs referenced there: plan-only publish https://github.com/Archont561/qgis-rs/actions/runs/36636577488 and shim failure https://github.com/Archont561/qgis-rs/actions/runs/36636090062. Both defects confirmed by reading main at 26e1349 - init.rs still emits the unwrapped matrix and both setup/action.yml and publish/action.yml still carry uses: ../action.yml. Fix verification cannot end at CI in this repo: nothing here consumes the generated workflow, which is why AC#3 and AC#4 ask for cover on the generated text itself.
<!-- SECTION:NOTES:END -->
