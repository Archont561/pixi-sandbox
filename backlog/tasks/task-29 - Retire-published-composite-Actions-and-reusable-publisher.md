---
id: TASK-29
title: Retire published composite Actions and reusable publisher
status: To Do
assignee: []
created_date: '2026-10-01 14:10'
updated_date: '2026-10-01 17:45'
labels:
  - actions
  - cleanup
  - release
  - breaking-change
milestone: m-0
dependencies:
  - TASK-32
references:
  - action.yml
  - setup/action.yml
  - publish/action.yml
  - .github/workflows/publish-sandbox.yml
  - .github/workflows/airlock.yml
  - scripts/render-action-shims.sh
  - scripts/lint-repo-consistency.sh
  - scripts/release-refs.sh
  - README.md
priority: high
type: enhancement
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Retire the public composite Action surfaces and reusable publisher after the native Pixi packages and direct CLI workflow are the supported path. Remove the repository-root `action.yml`, the generated `setup/` and `publish/` Actions, and `.github/workflows/publish-sandbox.yml`. Preserve repository CI and release workflows, but migrate their Action-specific tests and local invocations to direct `pixi-sandbox` CLI coverage. Ship the removal as an announced breaking release; immutable older tags and commit pins continue to expose their historical Action files.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 TASK-23's five native package variants have been published and smoke-tested, TASK-22's generated direct CLI publisher is validated, and TASK-32 makes channel installation canonical before removal begins
- [ ] #2 `action.yml`, `setup/`, `publish/`, `.github/workflows/publish-sandbox.yml`, and `scripts/render-action-shims.sh` are removed with no current workflow or generated artifact referring to them
- [ ] #3 `.github/workflows/airlock.yml` and other repository workflows retain equivalent native pack, checksum, doctor, publish, and restore coverage through direct CLI commands rather than local or remote pixi-sandbox Actions
- [ ] #4 Repository-consistency and release-reference tooling contains no generated-Action drift policy and no longer treats `action.yml` as the canonical repository identity source
- [ ] #5 README and active documentation remove Action and reusable-workflow installation examples, describe the generated direct CLI workflow as the supported replacement, and include a migration note for users upgrading from an Action-based release
- [ ] #6 The breaking release notes state that `Archont561/pixi-sandbox@…`, `/setup@…`, `/publish@…`, and the reusable `publish-sandbox.yml` are absent from new refs while immutable old tags and commit pins remain available
- [ ] #7 Actionlint, repository consistency, release preparation, generated-workflow tests, and the full enabled test suite pass after removal
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Wait for the canonical channel/package gate in TASK-32, then inventory every Action reference in workflows, scripts, docs, and tests. Move any still-useful hermetic coverage from the composite Action into direct CLI workflow steps before deleting the public files. Replace `action.yml` as the identity source in release tooling with an explicit canonical repository constant or structured package metadata. Remove obsolete rendering and drift checks, update documentation and release notes as a breaking migration, and verify that no current ref advertises the retired interfaces. Do not delete CI and release workflows that build, test, or publish the channel packages and standalone bootstrap assets.
<!-- SECTION:PLAN:END -->
