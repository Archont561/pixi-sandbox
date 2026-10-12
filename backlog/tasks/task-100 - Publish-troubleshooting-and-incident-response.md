---
id: TASK-100
title: Publish troubleshooting and incident response
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 08:47'
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
- [x] #1 All eight audited failure modes have entries in the shape above.
- [x] #2 Every entry names the evidence to collect using the tool actual diagnostics (--log-file, doctor --verify, branch/ref checks).
- [x] #3 The restore page and the CLI diagnostics sections link to the matching symptom entry.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: troubleshooting.mdx published under 'For airlock operators' and wired into the sidebar. All eight audited symptoms in the six-field shape (symptom, likely cause, safe immediate action, evidence, recovery, escalation), grounded in documented behavior: the launcher fetch and PIXI_SANDBOX_FETCH=skip, the trampoline trap (issue #81) and pack's empty-environment probe, the doctor --budget-config gate running before publish, host-requirement verdict semantics (unknown never fails), and the vendor/Cargo.lock drift path. Shared evidence kit (--log-file with the no-overwrite rule, doctor --verify, refs, publish-diagnostics artifact, redaction) opens the page; restore.mdx links the matching symptom entries by fragment; a filing section lists the reproducibility fields. Fragment links verified by the link checker. Build 22 pages zero warnings; biome 12 files green; ledger reduced to the task-102/103 forward links.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
All eight audited failure modes now have symptom-first entries with safe immediate actions, evidence collection, recovery, and escalation criteria, reachable from the restore page by fragment links.
<!-- SECTION:FINAL_SUMMARY:END -->
