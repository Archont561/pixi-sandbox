---
id: ADR-9
title: "Git access goes through a trait (`pixi-sandbox-git`)"
status: accepted
tags:
  - decision
---

# ADR-9 — Git access goes through a trait (`pixi-sandbox-git`)

**Decision.** `publish` and the airlock's fetch are expressed as `GitProtocol` operations
(`publish`, `fetch_into`, `branch_exists`, `remote_size`). `ShellGit` implements them with a
real `git` through a `Runner` (`ProcessRunner`, `RecordingRunner`, `PreviewRunner`); `FakeGit`
implements them in memory. Nothing else in the codebase runs `git`.

**Why.** The two operations that touch a remote are exactly the ones a test cannot have and
the ones that are dangerous to run for real (a force-push to a repository that might be
somebody's). A trait makes both testable: the flow's contract — one parentless commit, history
replaced, snapshot read-only, byte-exact fetch — is asserted against a mock *and* against real
git, and `--dry-run` becomes "the same code path with a runner that records instead of
executing" rather than a second implementation that drifts.

**Evidence ✅.** 11 tests in `crates/pixi-sandbox-git/tests/publish.rs`: the mock round-trips
publish → fetch byte for byte, replaces history instead of appending, and keeps the remote
unchanged when a push is rejected; the shell implementation does the same against a local bare
remote, and `RecordingRunner` proves `commit-tree` is called without `-p` (orphan) and `push
--force` is called (a replacement). `publish --dry-run` leaves the transport byte-identical
(asserted in the CLI tests).

**What would change it.** A git library (gix) as a second implementation behind the same
trait, if shelling out ever becomes the bottleneck or the portability problem.
