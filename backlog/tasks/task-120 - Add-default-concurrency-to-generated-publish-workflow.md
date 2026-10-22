---
id: TASK-120
title: Add default concurrency to generated publish workflow
status: Done
assignee: []
created_date: '2026-10-22 07:34'
updated_date: '2026-10-22 07:49'
labels:
  - ci
  - workflow
  - optimization
dependencies: []
priority: medium
type: enhancement
ordinal: 120000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #179 section 3: missing concurrency in publish-sandbox.yml

When multiple commits land on main in rapid succession (e.g. merging consecutive PRs), each push triggers full packaging and branch-pushing run across all runners. Without concurrency, these runs overlap and contend on force-push to same transport branch.

Current state:
- This repository own publish-sandbox.yml HAS concurrency (group publish-sandbox-${{ github.repository }}, cancel_in_progress false) configured via [workflow.concurrency] in .pixi-sandbox.toml (TASK-53, ADR-18)
- Generated consumer template DEFAULT has NO concurrency unless consumer configures [workflow.concurrency] explicitly
- Issue proposes:
  concurrency:
    group: publish-sandbox-${{ github.ref }}
    cancel-in-progress: true

Evaluate:
- Should default template have concurrency even without config? ADR-18 says presence of [workflow] table moves push_paths and setup_pixi_cache to new defaults; concurrency is per-field opt-in. Should we make concurrency default-on?
- cancel_in_progress true vs false: true cancels superseded runs, saving minutes but may cancel pending repack that would have published. False prevents aborting mid-flight force-push (current choice for this repo). Which is safer for consumers?
- Group naming: publish-sandbox-${{ github.ref }} vs publish-sandbox-${{ github.repository }}. Former is per-ref, latter repository-wide. For force-pushed transport branch, repository-wide is more correct (same branch regardless of ref), but per-ref is what issue proposes.
- Check existing behavior: this repo uses repository-wide + false, and documents that false does not guarantee every push publishes, but prevents aborted force-push.

Reference: https://github.com/Archont561/pixi-sandbox/issues/179 section 3
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document current concurrency behavior: optional via config, default none, this repo own config uses repository-wide false
- [ ] #2 Decide default concurrency group and cancel policy for generated consumer workflow
- [ ] #3 Implement default concurrency in template or via ADR-18 default change, with tests
- [ ] #4 Update golden file and docs, verify actionlint and check-repository pass
- [ ] #5 Measure minutes saved when 2 consecutive PRs merge
<!-- AC:END -->
