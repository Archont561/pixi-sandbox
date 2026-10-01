---
id: decision-1
title: Make pixi-sandbox the standalone transport and restore orchestrator
date: '2026-10-01 09:48'
status: accepted
---
## Context

The v1 plan (doc-1) proposed `pixi-sandbox` as the standalone transport/restore orchestrator:
one binary that owns `init`, `plan`, `pack`, `publish`, `restore`, `unpack`, `doctor`, and
`tools`, with connected hosts installing it from the `archont561/pixi-sandbox` prefix.dev
channel and airlocks bootstrapping from the verified binary embedded in the transport branch.
The open question was whether "standalone" also means absorbing `pixi-pack`/`pixi-unpack`
(the D2/D3 helper subprocesses), and how the transport schema and tool deduplication behave if
it does. The proposal stayed `proposed` until task-24 produced measurements.

## Decision

**Accepted, with the measured boundary recorded in decision-2:** `pixi-sandbox` is the
standalone transport and restore orchestrator and the single transport bootstrap — this is the
shipped v0.3.x design. `pixi-pack` and `pixi-unpack` are *not* absorbed; they remain pinned,
sha256-verified static helper tools driven as subprocesses, because absorbing the unpacker
trades 6.5 MiB of in-pack branch bytes (1.3 % of the branch) for a binary that grows by about
the same, a larger vendored payload, and ownership of prefix installation on a machine that
cannot be debugged. The transport schema stays at 2 under the compatibility policy in
decision-2, and the duplicate-packaging rule (one canonical `tools` copy, environment copies
are payload) is specified there.

## Consequences

The v1 workstreams could proceed (and did): provider-neutral `init` (task-21), direct-CLI
generated workflows (task-22), the five-platform package matrix (task-23), and the retirement
of the composite publishing interfaces (task-29/task-32/task-34, PR #47). Restores keep
delegating prefix installation to the unpacker's maintainers; the airlock bootstrap remains
exactly one verified binary per transport; and `.knowledge/decisions.md` D1–D13 remain
authoritative, extended by D14 (decision-2's outcome). See
`backlog/docs/spikes/standalone-transport/doc-7` for the measurements.
