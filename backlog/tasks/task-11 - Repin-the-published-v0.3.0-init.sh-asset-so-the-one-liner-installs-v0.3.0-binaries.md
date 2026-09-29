---
id: TASK-11
title: >-
  Repin the published v0.3.0 init.sh asset so the one-liner installs v0.3.0
  binaries
status: To Do
assignee: []
created_date: '2026-09-29 14:30'
labels:
  - release
  - docs
dependencies: []
priority: medium
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The v0.3.0 release asset init.sh was cut before task-2 repinned the installer, so the published one-liner (curl .../releases/download/v0.3.0/init.sh) still defaults to PIXI_SANDBOX_VERSION=v0.2.0 and installs v0.2.0 binaries. scripts/init.sh on main is correct (VERSION default v0.3.0); only the released copy is stale. Functional but misleading: a user who pins the v0.3.0 URL silently gets the previous binaries. Recorded as a caveat in the task-2 final summary; tracked here so it is not lost. Cannot be fixed from the airlock - uploads.github.com and the release-asset hosts are unreachable, so this needs CI or an online agent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The init.sh asset on the v0.3.0 release defaults to v0.3.0 binaries,Downloading and running the published one-liner installs a v0.3.0 pixi-sandbox and the version is asserted not assumed,release.yml cannot regress this again - the shipped asset is the same bytes as scripts/init.sh at the tag
<!-- AC:END -->
