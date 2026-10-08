---
id: TASK-79
title: >-
  Remove duplicated mib and make_executable helpers, dead manifest_path, and the
  second LauncherKind
status: Done
assignee: []
created_date: '2026-10-08 18:31'
updated_date: '2026-10-08 21:31'
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
- [x] #1 mib is defined once and used by the doctor, publish and support callers; output strings are unchanged and existing tests stay green.
- [x] #2 The user_tools.rs and support.rs make_executable copies are one function; replace.rs keeps its exact-0755 policy.
- [x] #3 publish.rs manifest_path is removed, or kept only if it is reachable and tested; the #[allow(dead_code)] attribute is gone.
- [x] #4 The two LauncherKind types are renamed or merged so each name means one concept; generated launcher output is byte-identical, shown by existing tests.
- [x] #5 Each change is a separate commit, the test count does not drop, and clippy with -D warnings stays clean.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->

Landed on the session branch as four commits, one per slice: the `mib` deduplication, the
`make_executable` consolidation, the dead `manifest_path` removal, and the LauncherKind
clarification.

Evidence, all local:

- AC#1 one `fn mib` remains (`commands/support.rs`); `doctor` and `publish` call `support::mib`
  the way pack, restore and unpack already did. The three bodies were byte-identical, and the
  suite pins every printed string green.
- AC#2 one `make_executable`: `user_tools` owns the chmod-or-keep implementation (promoted to
  `pub`, so `tests/` reaches it), `commands/support.rs` re-exports it for the command call
  sites. `self_update/replace.rs` keeps its exact-0755 policy — documented in place as
  deliberate: a staged self-update must land with the mode it was built with (task-37 AC#3) —
  and `tests/support/mod.rs` keeps its own helper per invariant 8 (an oracle must not share
  code with the thing it judges).
- AC#3 `publish.rs::manifest_path` is gone with its `#[allow(dead_code)]`, its comment and the
  now-unused `Path` import; the contract it claimed lives in `Manifest::path_in`, which
  `publish` itself calls and the core tests cover.
- AC#4 the private `commands/init.rs` enum is now `LauncherShell` (Posix/PowerShell — which
  launcher *script* to generate); `user_tools::LauncherKind` (Posix/Windows, pub) keeps its
  name and now means one concept (which flavour the *restoring host* runs). Generated output
  is byte-identical: a tempdir render diff of `init`'s whole output tree (restore.sh, both
  workflows, the seeded config, the round-tripped pixi.toml) is empty before versus after,
  and the byte-pinning init/user_tools/restore_script tests stay green. Alternative weighed
  and rejected: merging the two into one shared type — the variant sets answer different
  questions (script flavour vs host flavour), and the merge would churn the pub API and the
  generator tables for no behavioural gain.
- AC#5 four separate commits, in the required order; suite 767 → 767 passed / 1 skipped after
  every slice (count held), lint 11 gates green (clippy with `-D warnings` among them).

Baselines for the next session: 767 passed / 1 skipped.

<!-- SECTION:NOTES:END -->
