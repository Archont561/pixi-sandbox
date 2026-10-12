---
id: TASK-110
title: 'Add docs link, snippet, and accessibility checks to CI'
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - ci
  - accessibility
  - testing
dependencies:
  - TASK-96
references:
  - .github/workflows/docs.yml
  - docs/src/components/
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: low
type: docs
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The audit accessibility findings are desktop-only observations, not a WCAG audit — validate before and after the refactor. Wire checks into the docs workflow: internal link integrity after the re-homing, command-snippet tests where practical, keyboard operation of navigation/search/theme/copy buttons, visible focus in both themes, contrast of badges/diagram labels/callouts, and responsive behavior of the transport diagram and wide tables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An internal link check runs in docs.yml and fails on a dead link after the IA re-home.
- [ ] #2 Command snippets are tested where practical — at minimum the commands the first-bundle guide tells users to run.
- [ ] #3 Keyboard/focus/contrast/responsive checks are documented and run, covering nav groups, search, theme control, copy buttons, badges, diagram labels, and callouts in both themes.
- [ ] #4 Print/no-script usability of the operator checklist is verified.
<!-- AC:END -->
