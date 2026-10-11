---
id: ADR-22
title: "The canonical starter main is one leased root snapshot; release tags retain evidence"
status: accepted
tags:
  - decision
---

# ADR-22 — The canonical starter main is one leased root snapshot; release tags retain evidence

**Decision.** Owner-approved on 2026-10-11. Each successful starter publication replaces only
`Archont561/pixi-sandbox-starter`'s `main` with one parentless commit. Older namespaced release
tags remain immutable and reachable. This supersedes doc-10's blanket prohibition on rewriting
starter history; consumer repositories and forks remain out of scope.

**Why.** A template's main should present the current scaffold as a single starting point, not
an accumulation of generated release commits. Release tags preserve provenance without putting
that history in a new checkout's main ancestry. Retained tags mean this is not a whole-repository
object-count or storage-size guarantee.

**Mechanism.** Stage the validated publishable tree, use `write-tree` and `commit-tree` without
parents, annotate the release tag, then push both refs atomically. The explicit main lease is
its checked-out HEAD SHA, never a tracking ref refreshed just before the push. There is no plain
force or non-atomic fallback. Dry runs do not stage, commit, tag, or push. A new release can
publish an unchanged tree; an existing release tag is refused without overwriting evidence.
All starter releases share a concurrency group and explicitly check out main.

**Evidence.** Bare-remote integration tests prove one commit on main across two publications,
complete template files, retained old tag objects, new tag/main agreement, unchanged-tree
publication, and dry-run safety. Git adapter tests prove rejection of stale leases, existing
remote tags, protected branches, and servers without atomic support leaves both refs unchanged.
Removing the lease or atomic flag causes the corresponding tests to fail.

**Rollout boundary.** The live starter has not been rewritten by this session. Its owner must
allow the publisher credential to force-update main; the workflow never edits branch protection.
A connected publisher run is still needed to prove that permission policy on GitHub.

**What would change it.** A requirement to preserve a linear audit trail on main rather than
through immutable release tags, or a hosting policy that forbids all leased branch replacement.
