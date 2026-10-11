---
id: ADR-14
title: "The standalone bootstrap is single and the helpers stay (task-24's measured decision)"
status: accepted
tags:
  - decision
---

# ADR-14 — The standalone bootstrap is single and the helpers stay (task-24's measured decision)

**Decision.** `pixi-sandbox` is the standalone transport/restore orchestrator and the *single*
transport bootstrap: the verified binary travels exactly once, under
`.pixi-sandbox/tools/<platform>/`, and every launcher path (generated `restore.sh`/`restore.ps1`,
`scripts/restore.sh`, the bare-binary bootstrap, and the user-PATH registration) executes that
manifest-verified copy — never a `PATH`-discovered or environment-embedded one. `pixi-pack` and
`pixi-unpack` are not absorbed (ADR-2/ADR-3/ADR-4 stand). The transport schema stays at 2 under the
compatibility policy recorded in backlog ADR-24 (readers accept 1..=N and refuse newer;
additive `#[serde(default)]` fields may land without a bump; removals/semantic changes are a
bump that must keep reading every older schema while published branches exist). An environment
that depends on the `pixi-sandbox` package is payload, restored verbatim; `pack` warns about the
duplication instead of refusing; promoting the environment's copy to the bootstrap is rejected.

**Why.** One canonical executable per transport is what makes the launchers and the user-PATH
registration honest: the bytes they run are the bytes the manifest verified. Absorbing the
unpacker is a bad trade measured end to end, and the schema rule is what lets a published branch
outlive the binary that packed it.

**Evidence ✅.** Measured 2026-10-01 on the published `sandbox/developer-linux-64` branch
(backlog doc-7): `pixi-unpack` costs 6.5 MiB **in-pack** of a 503.9 MiB branch (1.3 %) — the
"15 MiB prize" is the file size, not the branch cost; `pixi` (32.9 MiB in-pack, 6.5 %) cannot be
removed; absorbing the unpacker grows the binary by roughly what it saves (proxy: `pixi-unpack`
is a 15.0 MiB static rattler front-end) and adds ~28 rattler crates to this repo's own vendored
payload at 1.79 MiB raw / 0.17 MiB in-pack per crate; measured peak restore disk 2 746 MiB
project / 4 098 MiB combined for a 1 830 MiB environment in 16 s — not a reported blocker. The
bootstrap copy itself costs 1.7 MiB in-pack (0.34 %), so double-shipping it is cheap but
pointless; git content-dedups the vendor tree (10 254 declared files → 9 663 objects,
278.1 MiB declared → 142.2 MiB raw → 25.8 MiB in-pack) while restore still materialises every
declared file.

**What would change it.** The four triggers of doc-5 §5 (an unmaintained `pixi-unpack` pin, a
real airlock blocked by peak restore disk — then prototype option C only, a supported rattler
"install this local channel into this prefix" entry point, a versioned pack format), plus one
new trigger: a real airlock blocked by *combined* disk (git store + checkout + restore), where
doc-7's option C′ (restore from the git object store, no checkout; −845 MiB measured) is the
first lever because it changes no binary and no format.
