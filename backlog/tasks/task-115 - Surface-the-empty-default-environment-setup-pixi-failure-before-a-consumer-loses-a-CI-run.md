---
id: TASK-115
title: >-
  Surface the empty-default-environment setup-pixi failure before a consumer
  loses a CI run
status: To Do
assignee: []
created_date: '2026-10-15 17:48'
labels:
  - docs
  - ci
  - init
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/158'
priority: high
ordinal: 115000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Consumers whose pixi.toml declares every environment explicitly, with no dependencies in the implicit default feature, meet the error No packages found in default environment for linux-64 platform from the setup-pixi step and never reach their own CI. setup-pixi runs pixi info, then pixi install --locked, then pixi list --locked, and the empty default environment makes the last one exit 1 before any consumer command runs. This project own examples produce that shape and nothing documents it, so the failure names neither the environment nor the failed command. See issue 158 and polybridge PR 1. Scope is to document the caution where consumers configure CI, and to decide whether init --check or plan should report an all-explicit workspace so the pitfall is visible before a CI run is lost. The two upstream defects, pixi list exiting non-zero for an empty environment and setup-pixi not tolerating it, are out of scope and tracked upstream.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The consumer-facing CI or workflow-policy guide warns that when the workspace default environment is empty, setup-pixi must be given an explicit environments value, and shows the naming workaround.
- [ ] #2 A check reachable from the CLI (init --check or plan, whichever the design lands) reports an all-explicit workspace whose implicit default environment is empty, or a recorded decision explains why no check is added.
- [ ] #3 The detection, or the documented caution when no check is added, is covered by a test at the appropriate boundary.
- [ ] #4 The upstream pixi and setup-pixi defects are recorded as out of scope with a pointer, so the repository does not silently absorb them.
<!-- AC:END -->
