---
id: TASK-113
title: Refresh Rustls for the 2026 cargo-deny advisories
status: To Do
assignee: []
created_date: '2026-10-15 10:10'
labels:
  - security
  - dependencies
dependencies: []
references:
  - Cargo.lock
  - deny.toml
  - .pixi-sandbox/vendor
  - >-
    backlog/tasks/task-112 -
    Add-a-local-pre-push-check-for-direct-main-pushes.md
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0346'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0347'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0348'
  - 'https://rustsec.org/advisories/RUSTSEC-2026-0349'
priority: high
type: chore
ordinal: 113000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As of 2026-10-15, `pixi run --frozen lint` fails at cargo-deny because the unchanged Cargo.lock resolves rustls 0.23.45, flagged by RUSTSEC-2026-0346, -0347, -0348, and -0349. The configured local Cargo home and `.pixi-sandbox/vendor` contain only 0.23.45; crates.io is not reachable from the current offline sandbox. Upgrade the transitive Rustls dependency to at least 0.23.46, refresh the checked-in vendor tree from the lockfile, and prove that the repository still builds offline. Do not suppress the advisories or expand this into an unrelated dependency refresh. This blocks TASK-112 AC#5 and its full lint DoD.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cargo.lock resolves rustls at version 0.23.46 or newer, with `.pixi-sandbox/vendor` regenerated from that same lockfile and no unrelated dependency churn.
- [ ] #2 cargo-deny no longer reports RUSTSEC-2026-0346, RUSTSEC-2026-0347, RUSTSEC-2026-0348, or RUSTSEC-2026-0349; no advisory-ignore exception is added to achieve this.
- [ ] #3 The refreshed lock and vendor tree support offline workspace build and test execution without fetching crates.
- [ ] #4 The repository lint and test gates pass, unblocking TASK-112 AC#5.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
- [ ] #4 pixi run --frozen -- convco check HEAD~1..HEAD
<!-- DOD:END -->
