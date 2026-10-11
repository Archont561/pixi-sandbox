---
id: ADR-12
title: "The conda pack/unpack subprocesses stay (rattler is not v2 work yet)"
status: accepted
tags:
  - decision
---

# ADR-12 — The conda pack/unpack subprocesses stay (rattler is not v2 work yet)

**Decision.** `pixi-pack` and `pixi-unpack` remain external, sha256-pinned static release assets
driven as subprocesses (ADR-2, ADR-3). `rattler` is **not** linked into `pixi-sandbox`. This is a
recorded **no-go** on backlog task-9's v2 spike, not a rejection forever — see the four triggers
below.

**Why.** The measurable prize is small and the measurable costs are not. Removing the
`pixi-unpack` subprocess drops one blob and ~1.8 % of transport bytes, but the code that replaces
it is itself roughly the size of the binary it removes, drags a networking/async crate tree into
*this* repository's own vendored payload, and makes prefix installation — placeholder rewriting,
`conda-meta`, activation scripts — our correctness problem on a machine with no network and no
second chance. `pixi` itself (the majority of the tools bytes) has to stay regardless, so no
version of this changes the payload's category.

**Evidence.** Measured on this repository's own `sandbox/developer-linux-64` transport, restored
and verified offline on 2026-09-29 (see `rattler-spike.md` for the full workup):
`pixi-unpack` is **15.0 MiB / 1 blob of the 826.0 MiB, 10 313-blob payload (1.82 % / 0.01 %)**;
`pixi` is 76.6 MiB (81 % of the tools bytes) and cannot be removed; this repo's vendored tree
costs **1.79 MiB of transport per crate**, so adding rattler's ~28-crate tree grows the payload
by the opposite sign and a larger magnitude than the 15 MiB saved; `pixi-unpack` is already a
rattler front-end, making its static musl binary the best empirical proxy for what linking rattler
in would cost. Of the four things a v2 would genuinely buy, only peak restore disk (2 876.8 MiB of
scratch for a 1 867.5 MiB environment) is a categorical win, and only option C (replace the
*install* step alone) targets it.

**What would change it.** The four triggers in `rattler-spike.md` §5: (1) `pixi-unpack` stops
shipping static assets for a tier-1 platform or a pin becomes unmaintainable; (2) peak restore
disk blocks a real airlock — then prototype **option C only**, measured against today's restore;
(3) rattler publishes a supported "install this local channel into this prefix" entry point that
already owns placeholder/`conda-meta` handling; (4) the pack format gains a versioned
specification. Any prototype is measured, not argued: binary size for all five targets,
`cargo deny` delta, cold `cargo check` time, restore wall clock, peak disk, and a byte-for-byte
comparison of the installed prefix against a `pixi-unpack` restore of the same transport.
