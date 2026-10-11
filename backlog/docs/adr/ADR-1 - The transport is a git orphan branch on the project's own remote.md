---
id: ADR-1
title: "The transport is a git orphan branch on the project's own remote"
status: accepted
tags:
  - decision
---

# ADR-1 — The transport is a git orphan branch on the project's own remote

**Decision.** `pack` writes a directory; `publish` puts it on an orphan branch
(`sandbox/<envs>-<platform>`) as a single force-pushed commit; the airlock `git fetch`es that
branch and never merges it.

**Why.** No new infrastructure (no registry, no artifact server, no credentials beyond the
remote the project already uses), the airlock already knows how to clone, and git gives us
content-addressed dedup for free.

**Evidence ✅.** Two environments packed separately share ~2× of their bytes in git; a new 5 MB
package adds ≈ +4.8 MiB to the branch; `git gc --aggressive` after a force-push does *not*
shrink an orphan repo, so history growth must be managed by rotating branches, not by
force-pushing.

**What would change it.** A project whose remote forbids force-push, or a payload > 1 GB per
platform set (GitHub's practical repo budget) would push us toward one branch per env, or to a
release-asset/artifact channel for the payload with the branch kept as the manifest.
