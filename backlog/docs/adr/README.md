# Architecture Decision Records

One file per decision, `ADR-N — <title>`, each with frontmatter (`id`, `title`, `status`,
`tags: [decision]`) so `backlog doc list` and `backlog search` index it.

**`ADR-N` is the anchor.** It is stable: source comments and tasks cite it, and renumbering one
would break those citations. New decisions take the next free number; never reuse or reorder.

The decisions formerly lived in two places — `.knowledge/decisions.md` (as `D1`…`D22`) and
`backlog/decisions/` (as `decision-1`…`decision-5`). Both are gone; this directory is the single
decision home. The old anchors map forward as follows.

## Former `D` anchors (`.knowledge/decisions.md`)

`D1`–`D22` became `ADR-1`–`ADR-22` **with the same number**.

| Old | New | Decision |
| --- | --- | --- |
| `D1` | `ADR-1` | The transport is a git orphan branch on the project's own remote |
| `D2` | `ADR-2` | `pixi-pack --directory-only` is the packer |
| `D3` | `ADR-3` | `pixi-unpack` is shipped, not assumed |
| `D4` | `ADR-4` | Helper tools are pinned release assets, compiled into the CLI by default |
| `D5` | `ADR-5` | A restored prefix is not a pixi environment until the markers are written |
| `D6` | `ADR-6` | Vendored cargo crates live under `.pixi-sandbox/vendor/` |
| `D7` | `ADR-7` | Verify before write; a join is atomic; never a half-populated environment |
| `D8` | `ADR-8` | The tool itself ships as a conda package |
| `D9` | `ADR-9` | Git access goes through a trait (`pixi-sandbox-git`) |
| `D10` | `ADR-10` | Tests use fixtures, never this repository |
| `D11` | `ADR-11` | Published sandbox branches come from an explicit native matrix |
| `D12` | `ADR-12` | The conda pack/unpack subprocesses stay (rattler is not v2 work yet) |
| `D13` | `ADR-13` | A restored project is verified against a per-file oracle, not against its shape |
| `D14` | `ADR-14` | The standalone bootstrap is single and the helpers stay |
| `D15` | `ADR-15` | Turbo owns the cross-language task graph; Pixi remains the environment facade |
| `D16` | `ADR-16` | Consumer upgrades regenerate generated files behind a review gate; pins never float |
| `D17` | `ADR-17` | Init writes only files it owns; the consumer's `pixi.toml` is never mutated |
| `D18` | `ADR-18` | `[workflow]` table presence, not mere field defaults, is the opt-in to safer policy |
| `D19` | `ADR-19` | Host-level requirements are declared in `[host_requirements]`, never installed |
| `D20` | `ADR-20` | The clippy gate is pedantic, inherited workspace-wide, with named exceptions |
| `D21` | `ADR-21` | uv-regenerated install metadata is presence-checked, not digested |
| `D22` | `ADR-22` | The canonical starter main is one leased root snapshot; release tags retain evidence |

## Former `decision-N` records (`backlog/decisions/`)

These were a **separate** decision set whose numbers overlapped with the `D` set, so they were
renumbered onto the end of the series rather than merged number-for-number.

| Old | New | Decision |
| --- | --- | --- |
| `decision-1` | `ADR-23` | Make pixi-sandbox the standalone transport and restore orchestrator |
| `decision-2` | `ADR-24` | Retain pixi-pack and pixi-unpack helpers; pixi-sandbox stays the single transport bootstrap |
| `decision-3` | `ADR-25` | Defer Turbo until the JS workspace graph earns it |
| `decision-4` | `ADR-26` | Consumer binaries self-update only to propose reviewed regenerated files, and production pins never float |
| `decision-5` | `ADR-27` | Adopt the `self_update` crate's transport, not its replace path |

Note `ADR-16` and `ADR-26` cover closely related ground (consumer upgrades behind a review gate).
They came from the two former sets and are both retained; a later consolidation should supersede
one explicitly rather than delete it.

## Former `.knowledge/` files

The five pointer stubs were deleted outright — they were 7–9-line notes saying "read this
Backlog document", and every inbound reference now points at the document itself.

| Old | New |
| --- | --- |
| `.knowledge/design.md` | `doc-2` — Transport and Restore Specification |
| `.knowledge/publish-automation.md` | `doc-3` — Publish Automation Specification |
| `.knowledge/rust-bootstrap.md` | `doc-4` — Rust Bootstrap Specification |
| `.knowledge/rattler-spike.md` | `doc-5` — Rattler Transport Spike |
| `.knowledge/research-results.md` | `doc-6` — Initial Research Results |
| `.knowledge/research/EVIDENCE.md` | `doc-16` — Evidence |
| `.knowledge/research/REPRODUCE-TRANSCRIPT.md` | `doc-17` — Reproduce Transcript |
| `.knowledge/v1-evolution-plan.md` | `doc-15` — v1 Evolution Plan |

## Consequence for the backlog CLI

`backlog/decisions/` no longer exists, so `backlog decision list` returns nothing and
`backlog doctor`'s duplicate-decision-ID check has nothing to inspect. Decisions are documents
now — use `backlog doc create` and tag the result `decision`. Running `backlog decision create`
would recreate `backlog/decisions/` starting again at `decision-1`, which would collide with the
`ADR` series; don't.

## Subsequent records

- ADR-30 — Generated CI bootstraps directly from verified standalone releases (TASK-126).
- ADR-31 — Native format-1 transports replace helper acquisition (TASK-125); supersedes
  ADR-2/3/4/24 for new producers, preserving legacy transport reading.
- ADR-32 — A transport branch is one bundle for one platform, and its payload stays
  self-contained (TASK-129); records the shipped topology and forbids a separate vendor branch.
- ADR-33 — One cargo vendor tree per workspace in one transport (TASK-131); extends ADR-6
  with per-workspace trees under `vendor/workspaces/<slug>/`, shared crates counted not deduped.
