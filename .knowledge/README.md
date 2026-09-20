# `.knowledge/` — Open Knowledge Format

Everything needed to understand *why* this project is designed the way it is. Written for humans and
agents who were not in the room.

| File | Purpose |
|:---|:---|
| [`decisions.md`](decisions.md) | The eleven load-bearing decisions (D1–D11) and the empirical measurements behind each — **start here** |
| [`design.md`](design.md) | In-depth technical specification: transport format, CLI surface, airlock invariants |
| [`rust-bootstrap.md`](rust-bootstrap.md) | Standalone static Rust bootstrap replacement strategy and platform checklist |
| [`publish-automation.md`](publish-automation.md) | `.pixi-sandbox.toml` bundle schema, setup actions, and multi-platform branch publication |
| [`research-results.md`](research-results.md) | Concise summary of initial research conclusions and feasibility studies |
| [`research/EVIDENCE.md`](research/EVIDENCE.md) | Raw lab measurements: bundle sizes, restore timings, Git object economics, Pixi internals |
| [`research/REPRODUCE-TRANSCRIPT.md`](research/REPRODUCE-TRANSCRIPT.md) | Verbatim cold-run reproduction log on a clean system |

The original Python reference implementation has been removed in favor of the pure Rust CLI.
Fixture transport is now static and checked in; regeneration is via Rust `pack` flow.

## Conventions

- **Provenance Tags**: `✅` marks empirically measured lab results; `⚠️` marks reasoned assumptions requiring validation.
- **Stable Anchors**: Code comments and decisions link to specific numbered sections (`… §11.3`, `… D4`). Keep these anchors stable when updating docs.
- **Reference Environment**: Debian 13, 2 vCPU, Pixi 0.81.0, pixi-pack/pixi-unpack 0.7.11, Cargo 1.98.1, linux-64.
