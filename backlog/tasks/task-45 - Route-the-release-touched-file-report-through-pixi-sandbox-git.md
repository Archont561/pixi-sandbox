---
id: TASK-45
title: Route the release touched-file report through pixi-sandbox-git
status: Done
assignee:
  - '@agent'
created_date: '2026-10-02 17:30'
updated_date: '2026-10-02 17:40'
labels:
  - architecture
  - git
  - testing
dependencies:
  - TASK-43
references:
  - crates/xtask/src/prepare_release.rs
  - crates/pixi-sandbox-git/src/shell.rs
  - AGENTS.md
priority: medium
type: refactor
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Task-43 moved pack provenance inside the Git boundary, but `prepare-release` still runs one
bare `git status --porcelain` subprocess (prepare_release.rs `write_touched_report`) — the
last production git call outside `pixi-sandbox-git` (invariant 8 / D9). Move it behind a
`ShellGit` primitive built on the existing Runner command model, make the report path an
explicit argument so the report becomes tempdir-fixture testable (it had none: the path was
read from the environment inside the function), and state invariant 8's test-fixture carve-out
in AGENTS.md so the `#[cfg(test)]` oracle builders that legitimately run real git stop being
re-flagged every audit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `prepare_release.rs` spawns no git subprocess; `ShellGit::changed_paths` owns the query through the Runner command model with identical porcelain parsing
- [x] #2 the touched-file report takes an explicit path argument (the `RELEASE_TOUCHED_FILE` override is resolved once at the entrypoint, the house args-with-defaults shape)
- [x] #3 tempdir-fixture tests cover the report against a real git: modified and staged tracked files listed, untracked excluded, clean tree writes an empty report
- [x] #4 invariant 8 in AGENTS.md states its scope: production git only through pixi-sandbox-git; `#[cfg(test)]` fixture builders may run real git as the oracle that must not share code with the thing it judges
- [x] #5 fmt, lint and the full suite pass with the test count above the 403/1 baseline
<!-- AC:END -->

## Implementation Plan
<!-- SECTION:PLAN:BEGIN -->
Add `changed_paths` to `ShellGit` beside `unstaged_modifications` (same `run_text` plumbing,
`status --porcelain --untracked-files=no`, name from byte 3). Split `write_touched_report`
into a pure `write_touched_report(root, touched_file)` plus env resolution at the call site.
Fixture tests in prepare_release.rs's test module, real git in a tempdir, fixture identity
via env vars per command — the commit-release test pattern.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-02: Implemented as proposed. `ShellGit::changed_paths` sits beside `unstaged_modifications` and documents why the two predicates are deliberately different; `write_touched_report(root, touched_file)` is pure and the `RELEASE_TOUCHED_FILE` override is resolved once at the entrypoint. The new fixture tests earned their keep immediately: the first cut routed the query through `run_text`, which trims the command output — porcelain's first line starts with a status *space* (` M file`), so the trim shifted the first entry's name by one byte and the modified/staged/untracked test failed on `HANGELOG.md`. Parsing now reads `Output::utf8` raw, byte-identical to the retired inline implementation, with the why documented on the primitive. The other two xtask `Command::new("git")` sites are `#[cfg(test)]` fixture builders (commit-release, airlock), which invariant 8's clarified wording in AGENTS.md now explicitly permits. Gates: fmt, lint (incl. check-repository), and the full suite green at 405 passed / 1 skipped (was 403/1).
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The last production git subprocess outside pixi-sandbox-git is gone: prepare-release's touched-file report goes through `ShellGit::changed_paths`, is tempdir-fixture tested against real git, and invariant 8 now states its scope so the test-fixture oracle builders stop being re-flagged. Suite 405/1.
<!-- SECTION:SUMMARY:END -->
