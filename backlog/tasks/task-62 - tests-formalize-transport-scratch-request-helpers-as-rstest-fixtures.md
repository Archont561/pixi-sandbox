---
id: TASK-62
title: 'tests: formalize transport, scratch, request helpers as rstest fixtures'
status: To Do
assignee: []
created_date: '2026-10-03 09:27'
updated_date: '2026-10-03 09:27'
labels:
  - testing
  - rstest
dependencies: []
priority: low
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Convert the manual builder functions in pixi-sandbox-core tests verify.rs transport, pixi-sandbox tests self_update.rs scratch and request, and pixi-sandbox-git tests publish.rs remaining hand-built setup into fixture functions, and convert the near-duplicate test bodies they feed into rstest plus case tables. See doc-9 section B.3.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 verify.rs transport, self_update.rs scratch and request become #[fixture] functions
- [ ] #2 Tests that called them directly are converted to #[rstest] consuming the fixture
- [ ] #3 Near-duplicate test bodies become #[case] tables where applicable
- [ ] #4 pixi run --frozen test passes with count >= 516/1
<!-- AC:END -->
