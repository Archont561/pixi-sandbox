---
id: TASK-68
title: >-
  workflow: surface publish failures outside Actions logs and preserve the last
  healthy transport
status: To Do
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - generated-workflow
  - diagnostics
  - publish
  - airlock
dependencies:
  - TASK-67
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/93'
  - crates/pixi-sandbox/src/generated/github_workflow.rs
  - crates/pixi-sandbox-git
  - docs/src/content/docs/guides/ci-publishing.mdx
priority: high
type: enhancement
ordinal: 68000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #93 asks the generated publisher to make failures diagnosable when the Actions log CDN is unavailable and to avoid replacing a healthy transport with incomplete output. Use the durable pipeline diagnostics from task-67, publish bounded failure evidence through a second GitHub-visible channel, and prove the publish boundary preserves the previous healthy orphan snapshot on every pre-publish or rejected-push failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The generated install → pack → doctor → publish lane writes one bounded diagnostic log and uploads it as an artifact on success or failure
- [ ] #2 On failure, a second GitHub-visible surface independent of the Actions log CDN identifies the failed phase and exposes or links the bounded log without leaking secrets
- [ ] #3 Workflow permissions and fork/event behavior are explicit; an unavailable comment or check API does not hide the original pipeline failure
- [ ] #4 Pack or doctor failure never invokes publish, and any failed/rejected publish leaves the previously healthy transport branch byte-identical and fetchable
- [ ] #5 The failure log is not committed into or published over the healthy transport branch; retention and size limits are documented
- [ ] #6 Renderer and tempdir/bare-remote tests cover phase failures and transport preservation, generated output remains actionlint-clean and byte-stable under init --check
- [ ] #7 A connected consumer run demonstrates the secondary evidence path when the primary workflow step fails; record it in this task and issue #93
<!-- AC:END -->
