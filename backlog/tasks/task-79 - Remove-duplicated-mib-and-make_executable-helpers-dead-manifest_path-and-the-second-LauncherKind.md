---
id: TASK-79
title: >-
  Remove duplicated mib and make_executable helpers, dead manifest_path, and the
  second LauncherKind
status: To Do
assignee: []
created_date: '2026-10-08 18:31'
labels:
  - cleanup
  - naming
dependencies: []
priority: low
type: chore
ordinal: 79000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Audit finding, production duplication and naming collisions in crates/pixi-sandbox.
- `fn mib` is defined three times with identical bodies: commands/doctor.rs:690 and commands/publish.rs:124 (private copies) and commands/support.rs:312 (pub(crate), the shared one).
- `make_executable` exists in commands/support.rs:131 and user_tools.rs:268 with the same chmod-or-add-0o111 logic. self_update/replace.rs:197 is a different policy (exact 0755, cfg-split), so keep it separate or document why.
- publish.rs:131 `manifest_path` is `#[allow(dead_code)]` and has no caller, even though its comment calls it part of the published contract.
- Two different enums are both named `LauncherKind`: commands/init.rs:23 (Posix, PowerShell; private) and user_tools.rs:44 (Posix, Windows; pub). They model the same question (which launcher flavour to write) with different variant names, which makes the code hard to read and easy to confuse.

Intended behaviour: one helper per concept, no dead function (remove it, or make its contract reachable and tested), and distinct names for the two launcher concepts (or one shared type).

Non-goals: no change to generated launcher bytes and no change to the replace.rs policy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 mib is defined once and used by the doctor, publish and support callers; output strings are unchanged and existing tests stay green.
- [ ] #2 The user_tools.rs and support.rs make_executable copies are one function; replace.rs keeps its exact-0755 policy.
- [ ] #3 publish.rs manifest_path is removed, or kept only if it is reachable and tested; the #[allow(dead_code)] attribute is gone.
- [ ] #4 The two LauncherKind types are renamed or merged so each name means one concept; generated launcher output is byte-identical, shown by existing tests.
- [ ] #5 Each change is a separate commit, the test count does not drop, and clippy with -D warnings stays clean.
<!-- AC:END -->
