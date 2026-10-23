---
id: TASK-133
title: Update ctutils and tokio for the two advisories that redden cargo deny
status: To Do
assignee: []
created_date: '2026-10-23 19:03'
labels:
  - dependencies
  - security
  - ci
dependencies: []
references:
  - Cargo.lock
  - deny.toml
documentation:
  - >-
    backlog/docs/adr/ADR-31 - Native format-1 transports replace helper
    acquisition.md
priority: high
type: bug
ordinal: 133000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two advisories published after the v0.8.0 release make `cargo deny check advisories` fail on an otherwise unchanged tree, which means the next `ci` run on main goes red at the lint step.

- **RUSTSEC-2026-0385** (vulnerability): `ctutils` 0.4.3 generates non-constant-time masks for the small integer widths; fixed in 0.4.4 (`cargo update -p ctutils`).
- **RUSTSEC-2026-0386** (unsoundness): `tokio` 1.53.2 `AsyncFd::new` and related constructors violate I/O safety; fixed in >=1.53.3, <1.52.0 is excluded so 1.53.3 is the target (`cargo update -p tokio`).

Both are transitive (neither is a direct dependency in this workspace). Neither fixed release is in the restored offline vendor — the vendor carries `ctutils-0.4.3` and `tokio-1.53.2` — so the update needs connected egress to fetch and re-vendor, or a CI runner with crates.io access. Do not add advisory exceptions: both have fixed releases, unlike the narrowly approved unmaintained-advisory exception already in \`deny.toml\`.

Found during TASK-132, where the failure was observed but deliberately not fixed: that task changes neither manifests nor the lockfile, and hiding an unrelated advisory behind an unrelated change is exactly how a stale claim enters the record.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ctutils resolves to >= 0.4.4 and tokio to >= 1.53.3 in Cargo.lock, with the new sources genuinely vendored (checksums from the crates.io index, no fabricated vendor data).
- [ ] #2 cargo deny check advisories/bans/licenses/sources passes with no new advisory exception; existing approved exceptions are unchanged and re-justified only if they still apply.
- [ ] #3 The workspace still builds offline from the vendor tree and every suite passes without a reduced baseline; any crate whose source cannot be vendored is reported instead of being forced.
- [ ] #4 The dependency change is one focused commit with the advisory ids in its body, so the release notes state why the lockfile moved.
<!-- AC:END -->

## Definition of Done
<!-- DOD:BEGIN -->
- [ ] #1 fmt, lint and test green on the updated graph, with the local versus connected boundary stated explicitly if the fix can only be proven on CI.
<!-- DOD:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
1. Bump the two crates and fetch sources (needs connected egress or a runner). 2. Re-vendor and verify the manifest hashes. 3. Run deny, clippy, the full suite and an offline build from the vendor tree. 4. Commit with the advisory ids.
<!-- SECTION:PLAN:END -->
