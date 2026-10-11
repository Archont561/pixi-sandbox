---
id: ADR-5
title: "A restored prefix is not a pixi environment until the markers are written"
status: accepted
tags:
  - decision
---

# ADR-5 — A restored prefix is not a pixi environment until the markers are written

**Decision.** After unpacking, `restore` writes `<prefix>/conda-meta/pixi_env_prefix` and
`<prefix>/conda-meta/.pixi-environment-fingerprint` (when the manifest recorded one).
`unpack` deliberately does *not* — it produces a prefix, not an environment.

**Why.** pixi validates the prefix through those markers before it will trust the environment
on disk and skip solving; without them `pixi install --frozen --offline` tries to do real
work.

**Evidence ✅.** A hand-placed, otherwise-correct environment is accepted by
`pixi install --frozen --offline` in ~0.03 s *with* the markers, and fails (offline) *without*
the `conda-meta` record. The fingerprint is pixi's own value: 16 bytes of xxh3 over the
`(name, sha256)` records of the lock ❓ it identifies the lockfile, and therefore cannot detect
a tampered prefix — that is `sha256` per blob's job.

**What would change it.** pixi changing either file name or its fingerprint algorithm (the
manifest stores the value, so a change is a re-pack, not a schema break).
