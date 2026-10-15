---
id: TASK-112
title: Add a local pre-push check for direct main pushes
status: In Progress
assignee: []
created_date: '2026-10-15 08:06'
updated_date: '2026-10-15 10:10'
labels:
  - git-hooks
  - ci
dependencies:
  - TASK-113
references:
  - lefthook.yml
  - crates/xtask/src/main_push.rs
  - crates/xtask/tests/main_push.rs
  - AGENTS.md
  - ADR-29
  - .agents/skills/session/SKILL.md
  - >-
    backlog/tasks/task-113 -
    Refresh-Rustls-for-the-2026-cargo-deny-advisories.md
priority: low
type: enhancement
ordinal: 112000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
ADR-29 keeps the authoritative main-integration verdict in CI, but a contributor with the repository's local hooks installed can get the same fast feedback before pushing. Add a local pre-push convenience that checks only updates targeting refs/heads/main and reuses the existing main-push judgment. This check may stop an invalid push in a clone where the hook is installed, but it is not repository-wide enforcement; CI remains authoritative. Do not duplicate the main-push policy or affect pushes to other refs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A Git push targeting refs/heads/main is judged using its actual remote and local object IDs and the existing ADR-29 rules; an unsanctioned update fails with the established remediation.
- [x] #2 A push targeting another remote ref does not invoke the main-integration judgment or fail merely because the local branch is not main.
- [x] #3 The local pre-push hook runs the check through Pixi and reuses the existing policy; any local rejection is only a convenience, not a substitute for CI or branch protection.
- [x] #4 Regression coverage at the agreed public seam proves a rejected main update, a sanctioned main update, and a non-main update that is ignored; tests use temporary Git fixtures, never this checkout.
- [ ] #5 The Lefthook configuration validates, and repository format, lint, and test gates pass.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
- [x] #4 pixi run --frozen -- lefthook validate
- [x] #5 pixi run --frozen -- convco check HEAD~1..HEAD
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Use Git pre-push input at the xtask CLI seam and test against temporary Git fixtures. Validate Lefthook configuration separately. Run refactor review after green.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `xtask check-main-push-hook`, which reads Git's pre-push ref-update protocol and delegates only `refs/heads/main` updates to the existing ADR-29 check. Main deletion is explicitly refused; blank input and non-main remote refs are no-ops. Lefthook's `main-integration` pre-push job invokes the command through Pixi with `use_stdin: true`. Updated AGENTS.md and the session skill to describe the local-only job and preserve CI as the repository-wide authority.

CLI tests use temporary Git fixtures and cover a rejected main update, a sanctioned main update, a feature remote ref even with local ref `refs/heads/main`, empty stdin, and deleting main. The focused `xtask::main_push` target passes 20/20; the full `pixi run --frozen test` suite passes. `pixi run --frozen fmt`, `pixi run --frozen -- lefthook validate`, and `pixi run --frozen -- cargo clippy --offline --workspace --all-targets -- -D warnings` pass. The real Lefthook job was separately exercised with pre-push input: it accepted a no-op main update and ignored a feature-ref update.

The full `pixi run --frozen lint` gate is currently blocked by the unchanged Cargo.lock pin `rustls 0.23.45`, which `cargo deny` flags for RUSTSEC-2026-0346, -0347, -0348, and -0349 (the advisory output recommends `rustls >=0.23.46`). Dependency remediation is outside this task; therefore AC#5 and DOD#2 remain open rather than recording a false green.

Dependency remediation is now tracked as TASK-113. TASK-112 depends on that follow-up for the repository-wide lint gate; this task remains In Progress until the linked remediation makes full lint green.
<!-- SECTION:NOTES:END -->
