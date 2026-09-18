# Bundle update log

History of this corpus. The dates are when the material was authored or corrected in the session that produced
it; `legacy:` front matter on each concept names the pre-OKF file and section it came from.

## 2026-09-19

* **Restructure**: converted the five root documents (`DESIGN.md`, `WORKFLOWS.md`, `WEB_SEARCH_RESULTS.md`,
  `SANDBOX_CONSTRAINTS.md`, plus the index in `README.md`) into this OKF v0.2 bundle — 51 concepts across five
  areas, with `index.md` per directory, a bundle-root [log](log.md) and
  [conventions](conventions.md). The root markdowns were removed; `README.md` is now a pointer into the bundle.
* **Update**: docs-source paths follow the move — `[docs].source-dir`, the sync globs and the Astro project
  diagram now read `.knowledge/**/*.md` and skip `index.md`/`log.md`
  ([Docs Site Design](/spec/docs-site.md), [Publishing the Docs](/workflows/publishing-docs.md)).
* **Creation**: the docs-publishing line of work — [Publishing the Docs](/workflows/publishing-docs.md),
  [Docs Site Design](/spec/docs-site.md), [Docs Pipeline, Measured in the Airlock](/research/docs-pipeline.md) —
  after building the whole Starlight pipeline inside this sandbox and rebuilding it with the network blackholed.
* **Correction**: *`rustc` and `cargo` are available as the conda-forge `rust` package* — from you, not from a
  probe ⇒ [The Rust Toolchain Is a Conda Package](/research/conda-forge-rust.md), plus rescoping of
  [Toolchain Resolution](/spec/toolchain-resolution.md), [Dogfooding](/workflows/dogfooding.md) and the earlier
  "no compiler can enter an airlock" claim.
* **Creation**: [Dogfooding on a Box Like This One](/workflows/dogfooding.md) with the D0–D4 fidelity ladder,
  including the docs tier that this sandbox *can* run end to end.

## 2026-09-18

* **Creation**: [The reference sandbox, measured](/environment/index.md) — host inventory, egress matrix,
  execution semantics, budgets and the reproduction script, all measured rather than assumed.
* **Creation**: the first evidence base — [pixi](/research/pixi.md), [pixi-pack](/research/pixi-pack.md),
  [cargo vendor and offline builds](/research/cargo-offline.md), [node_modules and bun](/research/node-bun.md),
  [GitHub Actions](/research/github-actions.md), [GitHub Markdown](/research/github-markdown.md),
  [AGENTS.md conventions](/research/agent-instruction-files.md),
  [clap + thiserror](/research/rust-cli.md), [OKF](/research/okf.md) — with first-hand reads of the cloned
  `prefix-dev/pixi` and `Quantco/pixi-pack` trees, and [Corrections and Method Notes](/research/corrections.md)
  recording each retraction.
* **Creation**: the design corpus itself ([spec/](/spec/index.md), [workflows/](/workflows/index.md)), then
  revised twice against your requirements — [Requirement Traceability](/overview/requirements.md) is the map of
  that revision, and [Manifest discovery, platform validation, offline reconstruction](/research/rev2-discovery.md)
  is the research behind it.
* **Creation**: [What Is Still Unproven](/workflows/unproven.md) as a first-class concept, so the gap register is
  a page an agent can be pointed at rather than a mood.
