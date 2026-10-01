# `.knowledge/` — Open Knowledge Format

Everything needed to understand *why* this project is designed the way it is. Written for humans and agents who were not in the room.

Actionable specifications, research summaries, and spikes now live in Backlog.md documents. `.knowledge/` retains raw evidence, historical decisions, and migration pointers.

| File | Purpose |
|:---|:---|
| [`decisions.md`](decisions.md) | Historical load-bearing transport and restore decisions (D1–D12); proposed changes are tracked through Backlog decisions |
| [`design.md`](design.md) | Pointer to Backlog document `doc-2`, the canonical transport and restore specification |
| [`rust-bootstrap.md`](rust-bootstrap.md) | Pointer to Backlog document `doc-4`, the canonical bootstrap specification |
| [`publish-automation.md`](publish-automation.md) | Pointer to Backlog document `doc-3`, the canonical publish automation specification |
| [`rattler-spike.md`](rattler-spike.md) | Pointer to Backlog document `doc-5`, the canonical transport spike |
| [`research-results.md`](research-results.md) | Pointer to Backlog document `doc-6`, the canonical research summary |
| [`v1-evolution-plan.md`](v1-evolution-plan.md) | Broader v1 rationale, alternatives, and unresolved questions |
| [`research/EVIDENCE.md`](research/EVIDENCE.md) | Raw lab measurements: bundle sizes, restore timings, Git object economics, Pixi internals |
| [`research/REPRODUCE-TRANSCRIPT.md`](research/REPRODUCE-TRANSCRIPT.md) | Verbatim cold-run reproduction log on a clean system |

## Canonical Backlog documents

- `backlog/docs/specifications/transport-and-restore/doc-2 - Transport-and-Restore-Specification.md`
- `backlog/docs/specifications/publish-automation/doc-3 - Publish-Automation-Specification.md`
- `backlog/docs/specifications/rust-bootstrap/doc-4 - Rust-Bootstrap-Specification.md`
- `backlog/docs/spikes/rattler-transport/doc-5 - Rattler-Transport-Spike.md`
- `backlog/docs/research/initial-results/doc-6 - Initial-Research-Results.md`
- `backlog/docs/plans/v1-platform-workflow-transport/doc-1 - v1-Platform-Workflow-and-Transport-Plan.md`

## Conventions

- **Provenance Tags**: `✅` marks empirically measured lab results; `⚠️` marks reasoned assumptions requiring validation.
- **Stable Anchors**: Code comments and decisions link to specific numbered sections (`… §11.3`, `… D4`). Keep these anchors stable when updating docs.
- **Reference Environment**: Debian 13, 2 vCPU, Pixi 0.81.0, pixi-pack/pixi-unpack 0.7.11, Cargo 1.98.1, linux-64.
- **Planning boundary**: backlog tasks are actionable work; backlog documents are canonical specifications/research/spikes; `.knowledge/` retains rationale, raw measurements, historical decisions, and discovery pointers.
