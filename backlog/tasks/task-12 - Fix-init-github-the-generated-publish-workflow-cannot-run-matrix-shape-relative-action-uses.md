---
id: TASK-12
title: >-
  Fix init github: the generated publish workflow cannot run (matrix shape +
  relative action uses)
status: Done
assignee:
  - '@agent'
created_date: '2026-09-29 22:15'
updated_date: '2026-09-29 22:35'
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
- [x] #1 init github emits an object-shaped matrix (include: nested under matrix:) and a test over the generated workflow text asserts it
- [x] #2 No published action path contains a relative uses of ../action.yml: setup/ and publish/ resolve from a remote ref, and a test or a scripts/lint-repo-consistency.sh check fails if one reappears
- [x] #3 A regression test pins plan --json against the generated workflow: every matrix.<key> the workflow reads exists in the include entries
- [x] #4 actionlint validates the workflow that init github generates, not only the workflows committed in this repo
- [x] #5 pixi run lint and pixi run test pass
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. init.rs: nest include: under matrix: in the generated workflow.
2. Make setup/ and publish/ self-contained: a relative uses: inside a REMOTE composite action resolves against the caller's workspace (or fails), so no cross-file reference can work - render both shims from the single root action.yml with scripts/render-action-shims.sh, exactly as templates/install.sh is rendered for the release asset.
3. lint-repo-consistency.sh check 7: re-render both shims and diff, and fail on any relative uses: in a published action path.
4. scripts/lint-generated-workflow.sh: generate a workflow with init github into a tempdir and run actionlint over it; wire into pixi run lint.
5. cli.rs tests: object-shaped matrix asserted on the generated text, and every matrix.<key> the generated workflow reads is a key of every plan --json include entry.
6. Prove the shim on a real runner: push a throwaway workflow that uses Archont561/pixi-sandbox/setup at the fixed commit and read the verdict.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Source: https://github.com/Archont561/pixi-sandbox/issues/37 (opened 2026-09-29). Repro runs referenced there: plan-only publish https://github.com/Archont561/qgis-rs/actions/runs/36636577488 and shim failure https://github.com/Archont561/qgis-rs/actions/runs/36636090062. Both defects confirmed by reading main at 26e1349 - init.rs still emits the unwrapped matrix and both setup/action.yml and publish/action.yml still carry uses: ../action.yml. Fix verification cannot end at CI in this repo: nothing here consumes the generated workflow, which is why AC#3 and AC#4 ask for cover on the generated text itself.

Proof on a real runner, not reasoning about the docs. Two throwaway workflows on this branch (since deleted; the runs stay as evidence).

Positive, run https://github.com/Archont561/pixi-sandbox/actions/runs/36639757694 pinned at 2735887: job 'plan' resolved uses: Archont561/pixi-sandbox/setup@2735887 and ran pixi-sandbox plan, and the matrix job was INSTANTIATED - GitHub named it 'publish (developer, default, linux-64, ubuntu-latest, sandbox/developer-linux-64, true)', i.e. the include entry expanded. The same job also ran publish/action.yml (subpath: setup mode), so both published action paths were exercised from a remote ref. Reproduced green a second time in run 36639860666.

Negative control, run https://github.com/Archont561/pixi-sandbox/actions/runs/36639860753: the identical shapes on the pre-fix code fail exactly as reported. Job 'plan-old' (uses: .../setup@26e1349, the old ../action.yml shim) has ONE step, 'Set up job', and it is red; the runner annotation is 'Failed to load Archont561/pixi-sandbox/26e13491.../setup/action.yml' followed by a System.FormatException - the runner cannot even format its own 'Expected format {org}/{repo}[/path]@ref' message because of the braces. Job 'publish-old' (matrix fed the include array) does not appear in the run's job list at all: not failed, never created. That is the silent mode this task is about, and the control shows both halves side by side in one run.

Opened as PR #38 (https://github.com/Archont561/pixi-sandbox/pull/38). All three required checks green on GitHub's runners, not only in the airlock: 'ci (lint · test · coverage)', 'validate airlock plan', and 'airlock linux-64' (the end-to-end pack/publish/offline-restore proof).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Both defects in issue #37 are fixed, and both fixes are verified on a real runner rather than by reading the YAML.

1. Matrix shape. init.rs now emits 'matrix:' with 'include:' nested under it. The regression cover is a text assertion in tests/cli.rs, not the linter: actionlint accepts the broken form (measured - it exits 0 on a workflow built to carry exactly that bug), because it will not evaluate the expression.
2. Published action paths. setup/ and publish/ are no longer shims. A relative uses: inside a REMOTE composite action resolves against the caller's workspace, and ../ is rejected by the reference parser, so no cross-file reference could ever have worked; ./ would be worse (it silently runs the consumer's own file), the proposed same-repo $/ syntax has not shipped, and hardcoding owner/repo@sha inside the shim would override the ref the caller pinned. Each path is therefore a complete composite action, rendered from the single root action.yml by scripts/render-action-shims.sh, with check 7 of lint-repo-consistency.sh re-rendering it, failing on drift and failing on any relative uses: - the same anti-drift shape task-2 used for the install.sh asset.

New gates: pixi run lint-generated-workflow (actionlint over a project produced by init github, wired into pixi run lint and ci.yml), lint check 7, and two cli.rs tests - the matrix shape, and 'every matrix.<key> the generated workflow reads is a key plan --json emits'. Both new tests were watched failing on the pre-fix generator before being accepted.

Evidence: run 36639757694 (fixed, pinned at 2735887) resolved setup@<sha> and instantiated the matrix job, which GitHub named 'publish (developer, default, linux-64, ubuntu-latest, sandbox/developer-linux-64, true)'; run 36639860753 (control) shows plan-old red at 'Set up job' with 'Failed to load .../26e13491.../setup/action.yml', and publish-old absent from the job list entirely. The proof workflows were deleted afterwards; the runs remain.

Drive-by, because it made the gate unrunnable in the airlock: lint check 4 matched SHA pins with /^[0-9a-f]{40}$/, and mawk (default awk on Debian-family hosts) has no interval expressions, so every correctly pinned SHA was reported as mutable and pixi run lint could not pass locally. Also fixed two docs examples that declared 'with:' twice in one step.

Not covered here: re-releasing. The fix lands on main; users pinning v0.3.1 keep the broken shims until the next tag.
<!-- SECTION:FINAL_SUMMARY:END -->
