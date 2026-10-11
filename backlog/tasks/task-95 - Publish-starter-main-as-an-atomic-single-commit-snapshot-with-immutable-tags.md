---
id: TASK-95
title: Publish starter main as an atomic single-commit snapshot with immutable tags
status: In Progress
assignee: []
created_date: '2026-10-11 18:51'
updated_date: '2026-10-11 18:52'
labels:
  - starter
  - git
  - publication
dependencies: []
priority: high
ordinal: 95000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Owner requested one commit on starter main after every update, and confirmed retaining immutable release tags. Test seams: public starter publication against isolated bare remotes and the Git adapter lease-protected atomic snapshot operation. No direct live repository rewrite is authorized in this implementation session.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each publication replaces canonical main with a parentless commit; prior release tags remain unchanged, including when a new release has the same tree.
- [x] #2 Main and the new tag update atomically under an explicit checked-out-HEAD lease; concurrent movement and rejection leave remote refs unchanged.
- [x] #3 Dry run makes no index, commit, tag, or remote changes; non-main and noncanonical destinations are refused.
- [x] #4 Document D22, update the starter contract and workflow concurrency, and pass local gates.
- [ ] #5 Record a successful connected publication proving the starter main has one commit and prior tags remain intact.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner confirmed single-commit main with old tags preserved and agreed the public starter publication / Git adapter test seams. Initial red: publishing_over_a_readme_only_starter_commits_the_whole_template observed two commits where one was required. Green: parentless commit-tree, checked-out HEAD lease, and atomic branch/tag push. The same integration test now proves dry-run index/ref safety, complete published tree, repeated new-release publication with an unchanged tree, retained old tag object, main/new-tag agreement, and existing-tag/non-main refusal. Four new real-bare-remote cases prove stale lease, tag conflict, protected branch, and unsupported atomic push leave all remote refs unchanged. Mutation checks: replacing the lease with --force makes the concurrent-update test fail; removing --atomic makes the protected-branch test detect a stray remote tag. Canonical destination validation was tightened from substring matching after its regression test accepted a similarly named fork; exact GitHub URL forms and explicit local bare-remote rehearsals remain supported. Workflow serializes all release tags and checks out main explicitly. D22/doc-10/TASK-74 record the owner-approved exception to the previous no-force contract. Refactor review kept snapshot plumbing inside the existing Git adapter, with no Git shell commands added to xtask. Gates: fmt, lint 11/11, tests 996 passing / 1 skipped (was 992/1). AC#5 remains open: no live main rewrite, protection change, push, or workflow dispatch was performed. Branch protection must permit the publisher credential to lease-force-update main; unsupported servers fail closed without a fallback.
<!-- SECTION:NOTES:END -->
