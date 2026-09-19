# AGENTS.md — how to work in `Archont561/pixi-sandbox`

This repository is a **design corpus with a code-shaped payload**, written as an
[Open Knowledge Format v0.2](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md)
knowledge bundle. Read these before editing anything.

## Read first

1. [`.knowledge/index.md`](.knowledge/index.md) — the bundle root; areas → groups → concepts.
2. [`.knowledge/conventions.md`](.knowledge/conventions.md) — labels ↔ trust tiers, front-matter extensions,
   editing rules.
3. [`CONTEXT.md`](CONTEXT.md) — the five-line state summary, if you only have a moment.

## Hard rules

- **Markdown only.** No source files, no binaries, no `action.yml`, no `scripts/` — the deliverable of this
  repo is the bundle plus root `README.md`, `AGENTS.md`, `CONTEXT.md`. [D21](.knowledge/spec/decisions.md) is
  decided, so the *implementation* will be a Rust crate — but it arrives in a later phase; until then all code
  lives as fenced blocks inside markdown, with `assemble.sh` as the measured behavioural oracle.
- **Never smooth the confidence labels.** `verified` (✅) means measured here or read from upstream source;
  `reasoned` (⚠️) means inferred; `open` (🚧) means unproven. Every claim keeps its label.
- **Do not commit or push unless asked.** Work stays in the working tree.

## The one-line design

A public GitHub Action packs `pixi` environments + `cargo vendor` trees onto an orphan branch of this repo and
ships a reconstructor with them, so a machine with nothing but git access reconstructs a working
`pixi run`-able environment. The v1 artifact is **one Rust binary** ([D21](.knowledge/spec/decisions.md),
decided): the action runs its CI verbs (`pack`/`publish`) and the kit ships the same binary as its assembler
(`reconstruct`) — built in CI, mirrored digest-pinned, installed by nobody.

## Current state of the design

- **Decided:** D1–D21 — the full spec, including D21's Rust assembler binary; **M0 (spec) is closed**.
- **Next:** M1 — build the crate + action; its acceptance vectors are the measured `assemble.sh` outcomes
  ([Testing Strategy](.knowledge/spec/testing-strategy.md)).
- **Measured:** the 183-line `assemble.sh` prototype, nine outcomes with stubbed drivers —
  [Running the Action](.knowledge/workflows/action-run.md). That script is the behavioural oracle the compiled
  assembler must reproduce vector-for-vector.

## Layout

| Path | What it is |
|---|---|
| `.knowledge/` | the OKF bundle (58 concepts, 5 areas) |
| `README.md` | front door for humans |
| `AGENTS.md` | this file |
| `CONTEXT.md` | slim state summary for agents |
