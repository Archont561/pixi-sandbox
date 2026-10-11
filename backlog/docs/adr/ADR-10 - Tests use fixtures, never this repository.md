---
id: ADR-10
title: "Tests use fixtures, never this repository"
status: accepted
tags:
  - decision
---

# ADR-10 — Tests use fixtures, never this repository

**Decision.** Project-level tests operate on `crates/pixi-sandbox/tests/fixtures/demo-project`
(a complete pixi project definition with no packer in its environment) and payload-level tests
on `tests/fixtures/transport` (a committed synthetic transport with real digests, including a
split blob). Two tests enforce the rule: the fixture must not depend on `pixi-pack`/`pixi-unpack`,
and no test may walk out of its crate from `CARGO_MANIFEST_DIR`.

**Why.** This repository is the tool's own development environment — `default` contains the
packer, the lockfile resolves hundreds of MiB — so a test pointed at the repository root tests
the machine, not the code: it passes where a fresh clone would fail, it is slow, it mutates
state, and a bug that only appears on a plain project would never surface. The payload fixture
also makes the verification path (including `.partNNN` reassembly) hermetic and instant
instead of needing a 95 MiB payload.

**Evidence ✅.** 5 policy tests plus 14 CLI tests run against the fixtures in milliseconds;
one of them caught a real `verify` bug (a split blob reported as *missing*). Measured bonus:
the fixture project packs **without installing an environment** — `pixi-pack` resolves from
`pixi.lock` — so pack-integration tests need network but not a multi-GiB local install
(485 blobs / 100.6 MiB / 2.2 s).

**What would change it.** Nothing we can think of; a test that cannot use a fixture should say
so in a comment and be `#[ignore]`d.
