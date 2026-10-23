---
id: TASK-134
title: The airlock Tier B gate fails on every 2026-10-23 branch
status: To Do
assignee: []
created_date: '2026-10-23 21:20'
labels:
  - ci
  - transport
  - airlock
dependencies: []
references:
  - .github/workflows/airlock.yml
priority: high
type: bug
ordinal: 134000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`airlock linux-64` fails at **Tier B — gate with the network reachable** (`Process completed with exit code 100`) on both branches pushed on 2026-10-23:

| Branch | Head | Airlock |
| --- | --- | --- |
| `arena/5a48024f-pixi-sandbox` | TASK-132, merged as PR #196 | failure, Tier B, exit 100 |
| `arena/542c12a9-pixi-sandbox` | TASK-129 slices 1–5, PR #199 | failure, Tier B, exit 100 |

The last successful airlock runs are the 2026-10-22 branches (`b4a9cf6a`, `fc55de06`, `112eea8f`, `6aceae22`, `aeb1dc12`, `1106ac98`). So the failure is **not** introduced by TASK-129: it is present on the earlier branch, whose head predates every TASK-129 commit. It is either pre-existing on `main` since 2026-10-22 evening or introduced by TASK-132.

The job log could not be read from this machine (`gh run view --log-failed` returns an EOF from the job-log blob storage; only annotations are retrievable, and they say nothing beyond the exit code). Nothing here names the cause — only the boundary.

Tier B runs `pixi run --frozen airlock-gate-run` over a transport packed from the repository's own `.pixi-sandbox.toml`, so a likely area is the pack→restore round trip TASK-132 touched (`artifact_location`), but that is a hypothesis, not a finding.

The task is to reproduce Tier B locally (or read the log from a machine that can reach job-log storage), name the actual failure, and fix it. Until then, "airlock is green" cannot be claimed for any branch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Tier B failure is reproduced locally with the same command the workflow runs, and the failing assertion is named.
- [ ] #2 The cause is either fixed, or shown to be pre-existing and filed separately with the evidence that predates it.
- [ ] #3 `airlock linux-64` is green on a branch that carries the fix.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 The fix is proven by the CI gate that actually runs it, not by a local substitute.
- [ ] #2 fmt, lint and test stay green and the suite count does not drop.
<!-- DOD:END -->
