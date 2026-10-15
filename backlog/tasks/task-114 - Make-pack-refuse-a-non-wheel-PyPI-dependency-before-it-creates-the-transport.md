---
id: TASK-114
title: Make pack refuse a non-wheel PyPI dependency before it creates the transport
status: Done
assignee:
  - '@me'
created_date: '2026-10-15 17:17'
updated_date: '2026-10-15 17:35'
labels:
  - pack
  - bug
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/157'
priority: high
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
pack validates the lock, paths, environment names and --cargo-vendor before it creates any output, but never checks that each selected environment PyPI dependencies are transportable. pixi-pack carries a PyPI dependency only as a wheel and otherwise bails late, after the install phase, from a subprocess, naming neither the environment nor what to change. See issue 157, found in polybridge PR 1. Read the same decision out of pixi.lock in the pack preflight.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 pack refuses before creating any output directory when a selected environment locks a PyPI dependency whose location is not a wheel, mirroring the rule pixi-pack applies: a location whose file name ends in whl.
- [x] #2 The refusal names every offending dependency - environment, package name and locked location - across all selected environments, not only the first.
- [x] #3 The check is scoped to the selected envs and platform: a non-wheel dependency in an unselected environment or another platform is not refused.
- [x] #4 pipeline inherits the refusal because it drives pack with envs, so the dependency is named before pixi-pack runs rather than after the install phase.
- [x] #5 A wheel location is accepted, including one carrying a sha256 fragment.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Add a global pixi.lock scanner module and a refuse helper. Wire the helper into PackPlan::from_args so pack refuses before any output exists. Test the pure scanner and the refusal message under tests/pixi_lock.rs, and the pack refusal black-box under tests/cli.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added crates/pixi-sandbox/src/pixi_lock.rs: a small indentation scanner over pixi.lock that reads environments.<env>.packages.<platform> PyPI locations and the top-level packages name map, plus non_wheel_pypi and refuse_non_wheel_pypi. Mirrors the pixi-pack rule exactly: a location is transportable iff its file name, after a # or ? fragment is stripped, ends in whl. Wired into PackPlan::from_args beside the other preflight guards, before any output directory is created. Tests: tests/pixi_lock.rs covers a path dep, wheels with and without a sha256 fragment, a direct+ wheel URL, a git source and an sdist, multi-environment ordering, platform scoping, name fallback and an unreadable lock; tests/cli.rs adds a black-box pack refusal asserting the message names environment, package and location and that no output directory is created. The pure scanner avoids adding a YAML dependency to the airlocked vendor tree.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
pack now refuses a non-wheel PyPI dependency before creating the transport. A new pixi_lock module scans the lock with a small indentation reader and reuses the exact pixi-pack wheel rule, and PackPlan::from_args calls it with the selected envs and platform, so pack and the pipeline fail early naming every offending environment, package and location instead of letting pixi-pack bail after the install phase. 11 new tests; fmt, lint 11/11 and the full test suite are green.
<!-- SECTION:FINAL_SUMMARY:END -->
