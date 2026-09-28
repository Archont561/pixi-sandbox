---
id: TASK-2
title: Point the install one-liner at a release that exists (v0.3.0 vs v0.2.0)
status: To Do
assignee: []
created_date: '2026-09-28 22:05'
labels:
  - docs
  - release
dependencies: []
references:
  - README.md
  - docs/src/content/docs/quickstart.mdx
  - docs/src/content/docs/guides/using-in-your-project.mdx
priority: high
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
README.md, the docs quickstart and using-in-your-project all curl scripts/init.sh from the v0.3.0 tag, but the latest published release is v0.2.0, so the flagship install one-liner 404s for every new user. Either cut the v0.3.0 release (preferably after the osx-arm64 proof in task-1 lands) or repin every reference to v0.2.0.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every documented install URL resolves with HTTP 200 for the pinned tag
- [ ] #2 The pin is consistent across README.md and every docs page that references a version
- [ ] #3 A v0.3.0 release, if cut, ships init.sh plus the five static binaries and SHA256SUMS matching the release workflow contract
<!-- AC:END -->
