---
id: ADR-2
title: "`pixi-pack --directory-only` is the packer"
status: accepted
tags:
  - decision
---

# ADR-2 — `pixi-pack --directory-only` is the packer

**Decision.** Environments are packed with `pixi-pack` (a static release asset, see ADR-4) using
`--directory-only`, one directory per env under `.pixi-sandbox/envs/<env>/pack/`.

**Why.** It produces exactly the transportable unit: the environment's raw `.conda` files as a
local channel plus `environment.yml` + `pixi-pack.json`. Directory mode keeps file
granularity, which is what git dedup and our sharding need. Re-implementing this on top of
`rattler` is possible but is not v1 work.

**Evidence ✅.** `--directory-only` output for a real environment is ≈3.7× smaller than the
same environment as a prefix tar (which is mostly hardlink-flattened duplicates); 5567 prefix
files vs a few dozen `.conda` blobs; the tar form also loses the per-file digests we verify.
`pixi-pack`/`pixi-unpack` are **not on crates.io** ✅, so using them means shipping binaries
(ADR-4), not linking them.

**What would change it.** A `rattler`-based packer that is byte-compatible with what
`pixi-unpack` expects would remove the subprocess and let us drop the external tools
(the ⚠️ v2 path in `doc-2` §1).
