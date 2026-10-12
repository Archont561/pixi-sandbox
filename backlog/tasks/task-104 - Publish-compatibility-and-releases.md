---
id: TASK-104
title: Publish compatibility and releases
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - releases
  - compatibility
dependencies: []
references:
  - docs/astro.config.mjs
  - docs/src/content/docs/installation.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: medium
type: docs
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Readers cannot currently discover the support policy: supported pixi-sandbox versions, compatible Pixi versions, Rust/Cargo/OS support, generated-workflow compatibility, retired interfaces, and security fixes. Add a "Compatibility and releases" page carrying the current release and documentation version, a support matrix (OS, platform, Pixi, Git, Rust where relevant), an upgrade checklist, the generated-workflow upgrade policy, a migration table for retired composite Actions/reusable publisher paths, and a link to the GitHub releases feed — plus a compact version/release badge in the header or footer. A full version switcher is explicitly out of scope until multiple docs versions are maintained.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The page names the current release and the documentation version, with a support matrix covering OS, platform, Pixi, Git, and Rust requirements.
- [ ] #2 It carries an upgrade checklist and the generated-workflow upgrade policy.
- [ ] #3 A compact version/release badge appears in the header or footer and links to the releases feed.
- [ ] #4 No version switcher is introduced.
<!-- AC:END -->
