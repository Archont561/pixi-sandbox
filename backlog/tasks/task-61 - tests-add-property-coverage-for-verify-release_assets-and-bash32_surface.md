---
id: TASK-61
title: 'tests: add property coverage for verify, release_assets, and bash32_surface'
status: Done
assignee: []
created_date: '2026-10-03 09:27'
updated_date: '2026-10-03 09:54'
labels:
  - testing
  - proptest
dependencies: []
priority: medium
ordinal: 62000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add proptest properties. First, pixi-sandbox-core verify::verify: flipping any single byte in any blob of a valid transport is always reported as a failure, an untouched transport never is. Second, xtask release_assets.rs staged_name and binary_name: asset name always starts with pixi-sandbox-, ends with .exe only when the triple contains windows. Third, xtask repo_checks.rs bash32_surface regex: any injected forbidden token always fires, allow-listed vocabulary never does. Keep the existing example-based regression tests alongside; do not delete the v0.3.6-incident cases. See doc-9 section B.1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A property test for verify() asserts any single-byte flip in any blob of a valid transport is always a reported failure
- [x] #2 A property test for release_assets staged_name/binary_name holds over arbitrary target triples
- [x] #3 A property test for bash32_surface holds for injected forbidden tokens and for allow-listed vocabulary
- [x] #4 Existing example-based regression tests are kept, not replaced
- [x] #5 Each new proptest has a committed proptest-regressions file and a bounded case count
<!-- AC:END -->
