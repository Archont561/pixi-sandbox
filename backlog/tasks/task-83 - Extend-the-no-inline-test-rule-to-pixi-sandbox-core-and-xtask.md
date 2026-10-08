---
id: TASK-83
title: Extend the no-inline-test rule to pixi-sandbox-core and xtask
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
labels:
  - testing
  - policy
dependencies: []
priority: medium
type: enhancement
ordinal: 83000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding, test policy. AGENTS.md (Test conventions) says pixi-sandbox-core and xtask "still carry inline tests and are not yet enforced; splitting them is backlog work". No open backlog task tracks that work. Measured: 27 source files contain #[cfg(test)] modules, 3,077 lines in total (core: 4 files, 326 lines; xtask: 23 files, 2,751 lines). pixi-sandbox-git has none. The binary crate is guarded by production_sources_carry_no_inline_test_modules in crates/pixi-sandbox/tests/fixtures.rs, with a LEGACY list that may only shrink.

Intended behaviour: move each inline module to tests/ through the crate's public API. Extend the guard to pixi-sandbox-core and xtask with a shrinking LEGACY list, so the rule is enforced rather than promised.

Seam to agree before tests are written: per module, the public API, exercised from tests/<module>.rs. Private helpers that need testing are exported #[doc(hidden)] pub, as AGENTS.md already prescribes.

Non-goals: no behaviour change, no new test framework, and no weakening of assertions to make moved tests pass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each inline module in pixi-sandbox-core and xtask moves to tests/; no test is lost (names kept, or a recorded mapping), and the test count does not drop.
- [ ] #2 A guard test in each crate's tests/ fails on any production source containing #[cfg(test)] that is not in a LEGACY list, and fails on stale LEGACY entries (mirroring fixtures.rs).
- [ ] #3 The LEGACY list is recorded in the guard test when it lands and shrinks over successive commits; the final list is empty.
- [ ] #4 AGENTS.md's test-convention paragraph is updated to match the enforced state.
- [ ] #5 No test points at this repository or a real HOME (D10).
<!-- AC:END -->
