---
id: TASK-100
title: Publish troubleshooting and incident response
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - troubleshooting
  - incident-response
  - airlock
dependencies: []
references:
  - docs/src/content/docs/restore.mdx
  - docs/src/content/docs/reference/cli.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 100000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Operators under pressure search for symptoms, not commands. Add a symptom-organized page covering the eight audited failure modes: restore attempted network access; doctor found a digest mismatch; sandbox branch not in the transfer bundle; restore cannot execute the embedded binary; Git object larger than GitHub limit; host lacks GUI/display/font capability; CI publish over budget; vendored Cargo build still fetches. Each entry gets symptom/error text, likely cause, safe immediate action, evidence to collect (--log-file, doctor --verify, branch/ref checks), recovery action, and escalation/reproducibility details. Adapts Playwright running/debugging model to this safety-sensitive domain.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 All eight audited failure modes have entries in the shape above.
- [ ] #2 Every entry names the evidence to collect using the tool actual diagnostics (--log-file, doctor --verify, branch/ref checks).
- [ ] #3 The restore page and the CLI diagnostics sections link to the matching symptom entry.
<!-- AC:END -->
