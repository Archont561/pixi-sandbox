---
id: ADR-3
title: "`pixi-unpack` is shipped, not assumed"
status: accepted
tags:
  - decision
---

# ADR-3 — `pixi-unpack` is shipped, not assumed

**Decision.** The branch carries `pixi-unpack` (same static asset as ADR-4) and `restore` drives
it; `unpack` accepts `--unpacker` to override it.

**Why.** It is the only supported way to turn a pack back into a prefix. Without it the airlock
cannot install anything, and "install pixi-unpack first" would violate the no-manual-steps
rule.

**Evidence ✅.** `pixi-unpack <pack> -o <stage> -e <env>` is the documented pairing for
`pixi-pack` output; it stages into `$TMPDIR` (the trap in `doc-2` §4), and the static musl
build runs on the airlock with no loader present.

**What would change it.** Shipping our own unpacker built into `pixi-sandbox` (v2), or an
upstream format guarantee that would let us treat a pack as a plain conda channel install.
