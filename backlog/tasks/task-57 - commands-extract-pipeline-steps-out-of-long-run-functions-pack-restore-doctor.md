---
id: TASK-57
title: >-
  commands: extract pipeline steps out of long run() functions (pack, restore,
  doctor)
status: To Do
assignee: []
created_date: '2026-10-03 09:26'
updated_date: '2026-10-03 09:27'
labels:
  - refactor
  - kiss
dependencies: []
priority: medium
ordinal: 58000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
pack::run (332 lines), restore::run + register_user_tools (142+139 lines), and doctor::print_human/as_json (163+101 lines) mix validation/I-O/business logic inline. Extract named steps (e.g. a PackPlan with ensure_fresh_output/resolve_tools/build_files_oracle/write_manifest_and_docs) per the sketch in doc-9 section A3. Do one command per sub-task; land pack first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pack::run is decomposed into named steps (validation, tool resolution, files oracle, manifest and docs write)
- [ ] #2 Each extracted step has its own focused unit test
- [ ] #3 No behavior change: existing pack/restore/doctor integration tests pass unchanged
- [ ] #4 pixi run --frozen test passes with count >= 516/1
<!-- AC:END -->
