---
id: TASK-95
title: Publish starter main as an atomic single-commit snapshot with immutable tags
status: Done
assignee: []
created_date: '2026-10-11 18:51'
updated_date: '2026-10-15 21:10'
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
- [x] #4 Document ADR-22, update the starter contract and workflow concurrency, and pass local gates.
- [x] #5 Record a successful connected publication proving the starter main has one commit and prior tags remain intact.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Owner confirmed single-commit main with old tags preserved and agreed the public starter publication / Git adapter test seams. Initial red: publishing_over_a_readme_only_starter_commits_the_whole_template observed two commits where one was required. Green: parentless commit-tree, checked-out HEAD lease, and atomic branch/tag push. The same integration test now proves dry-run index/ref safety, complete published tree, repeated new-release publication with an unchanged tree, retained old tag object, main/new-tag agreement, and existing-tag/non-main refusal. Four new real-bare-remote cases prove stale lease, tag conflict, protected branch, and unsupported atomic push leave all remote refs unchanged. Mutation checks: replacing the lease with --force makes the concurrent-update test fail; removing --atomic makes the protected-branch test detect a stray remote tag. Canonical destination validation was tightened from substring matching after its regression test accepted a similarly named fork; exact GitHub URL forms and explicit local bare-remote rehearsals remain supported. Workflow serializes all release tags and checks out main explicitly. ADR-22/doc-10/TASK-74 record the owner-approved exception to the previous no-force contract. Refactor review kept snapshot plumbing inside the existing Git adapter, with no Git shell commands added to xtask. Gates: fmt, lint 11/11, tests 996 passing / 1 skipped (was 992/1). AC#5 remains open: no live main rewrite, protection change, push, or workflow dispatch was performed. Branch protection must permit the publisher credential to lease-force-update main; unsupported servers fail closed without a fallback.

2026-10-15 — AC#5 proven by the first connected publication under ADR-22. Starter run https://github.com/Archont561/pixi-sandbox/actions/runs/38804692103 (main at e16884a, release v0.7.0, source commit d54ae974f487e1946d059ee6707dc018724ee32b) passed its "Publish the starter revision" step, 20:48:14–20:48:16 UTC.
- One commit on main: starter `main` is 66704220209df2294655bc61570a8aa77eda40df, a parentless commit (`git rev-list --count main` = 1). Its message is `Initial commit`, then `pixi-sandbox v0.7.0` and `Source commit: d54ae974f487e1946d059ee6707dc018724ee32b`. The previous main, c971ef773ddf8490b29942c332a3d70819820ef7, was replaced. It was the owner's untagged 2026-10-11 `cargo_vendor` edit on top of 1655285c, two commits deep. The replacement also proves the publisher credential may lease-force-update main, the permission ADR-22's rollout boundary left open.
- Main and the new tag agree: annotated tag pixi-sandbox-v0.7.0 (object e1547f2fe844a18c8b391e3c30f4968fa1d4498c, tagged 2026-10-15T20:48:15Z) names 66704220, the same commit as main.
- Prior tags intact:
  - pixi-sandbox-v0.6.2 is still tag object 7f24bfa0977441cea7c7e8c042614c2bba8d2248 → 1655285c27f162f24be33428fb1c1f791bd2e063, the exact SHAs TASK-74 recorded on 2026-10-11.
  - pixi-sandbox-v0.6.1 is still tag object d75c3a8f737c7d9f141b19eccaaa3a37a9275fb4 (tagger date 2026-10-11T09:59:35Z) → 945ed1a718342b203a2937cbaa41a3e6aa173ea0.
- Downstream: the push started the starter's own publisher, run https://github.com/Archont561/pixi-sandbox-starter/actions/runs/38804893239, which succeeded (TASK-94 AC#4).

A successful run cannot exercise lease rejection, tag conflict, protected branches or non-atomic servers; those paths remain proven by the AC#2 bare-remote tests. The evidence is GitHub's job/step conclusions plus `git ls-remote` and `git cat-file` on the published refs; raw logs are egress-blocked from this sandbox.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Starter publication now replaces only canonical main with one parentless, lease-protected snapshot, pushed atomically with its immutable release tag, while older tags stay untouched (ADR-22). AC#1–#4 were proven against bare remotes. AC#5 is proven live by starter run 38804692103: main is the single root commit 66704220 for v0.7.0, the new tag names it, and the v0.6.1 and v0.6.2 tags are unchanged. Task closed.
<!-- SECTION:FINAL_SUMMARY:END -->
