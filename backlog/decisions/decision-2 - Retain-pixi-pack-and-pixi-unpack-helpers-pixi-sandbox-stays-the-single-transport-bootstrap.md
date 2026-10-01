---
id: decision-2
title: >-
  Retain pixi-pack and pixi-unpack helpers; pixi-sandbox stays the single transport
  bootstrap
date: '2026-10-01 15:18'
status: accepted
---
## Context

Task-24 asked whether the standalone `pixi-sandbox` binary should absorb `pixi-pack` and
`pixi-unpack` (instead of embedding them as pinned helper tools), what that would buy in
binary size and restore disk, how the transport schema stays compatible, and how
`pixi-sandbox` is kept from being shipped twice (bootstrap tool plus environment dependency).

The measurements are in `backlog/docs/spikes/standalone-transport/doc-7`, taken on 2026-10-01
from the published `sandbox/developer-linux-64` branch (snapshot `7c69402`, packed by 0.3.6)
and a fresh offline build of v0.3.7. Decisive numbers: `pixi-unpack` costs **6.5 MiB in-pack
(1.3 % of the 503.9 MiB branch)** — the "15 MiB prize" is the file size, not the branch cost;
`pixi`, which cannot be removed, costs 32.9 MiB in-pack; absorbing the unpacker grows the
binary by roughly what it saves (proxy: `pixi-unpack` is itself a 15.0 MiB static rattler
front-end) and adds ~28 rattler crates to this repository's own vendored payload (measured
vendor cost: 1.79 MiB raw / 0.17 MiB in-pack per crate); measured peak restore disk is
2 746 MiB project / 4 098 MiB combined for a 1 830 MiB environment, restoring in 16 s — not
reported as a blocker by any real airlock.

## Decision

**Accepted — the standalone design stands as shipped; the helpers are retained.**

1. `pixi-sandbox` is the standalone transport/restore orchestrator and the **single transport
   bootstrap**: the verified binary travels exactly once, under
   `.pixi-sandbox/tools/<platform>/`, and every launcher path (generated `restore.sh` /
   `restore.ps1`, `scripts/restore.sh`, the bare-binary bootstrap, and TASK-33's user-PATH
   registration) executes that manifest-verified copy — never a `PATH`-discovered or
   environment-embedded one.
2. `pixi-pack` and `pixi-unpack` remain pinned, sha256-verified static helper tools driven as
   subprocesses (D2/D3/D4, reaffirming D12 with current measurements).
3. **Schema stays 2.** Readers accept 1..=2 and refuse newer; additive fields may land without
   a bump only when old readers ignore them safely; removals or semantic changes are a
   schema-3 event that must keep reading 1 and 2 for as long as published branches exist.
   Git hardlinks are not a deduplication mechanism (archives and worktrees do not preserve
   them).
4. **Duplicate-packaging rule:** the `tools` map holds one entry per tool name; an environment
   that depends on the `pixi-sandbox` package is restored verbatim (it is payload, bounded by
   the bootstrap's 1.7 MiB in-pack cost); `pack` warns about the duplication instead of
   refusing; promoting the environment's copy to the bootstrap is rejected (version coupling:
   an older pinned copy could not read a newer manifest schema).

## Consequences

- No transport-format change, no migration, no new dependency; D2/D3/D4/D12 stand as written
  and `.knowledge/decisions.md` gains D14 recording this outcome.
- The cheapest remaining disk lever is option C′ of doc-7 §2 (restore from the git object
  store, no worktree checkout; −845 MiB combined), noted for the backlog — it changes the
  `--branch-location` contract shared by `doctor`/`restore`/`unpack` and the launchers, so it
  is a task of its own, not a rider on this decision.
- Follow-up implementation tasks: the pack-time duplicate-`pixi-sandbox` notice (doc-7 §4.3)
  and TASK-33's registration pointing at the `tools/<platform>/` copies.
- Revisit on the doc-5 §5 triggers (unmaintained unpacker pin, a real airlock blocked by peak
  restore disk, a supported rattler install entry point, a versioned pack format) plus the
  new one: a real airlock blocked by *combined* disk, where option C′ is the first lever.
