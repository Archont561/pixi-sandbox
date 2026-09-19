# Bundle update log

History of this corpus. The dates are when the material was authored or corrected in the session that produced
it; `legacy:` front matter on each concept names the pre-OKF file and section it came from.

## 2026-09-19

* **Accept D21 + reshape the corpus** (user: "Accept D21 and reshape knowledge to fully reflect that direction"):
  D21 is now **Decided** and D19 is marked *amended by D21* in [Decision Log](spec/decisions.md) — the v1
  artifact is one Rust binary (the action runs `pack`/`publish`, the kit ships the same binary as its
  assembler), installed by nobody. Reshaped to match: [The Action Shape](spec/action-shape.md) (one-sentence,
  comparison table, kit layout with `bin/pixi-sandbox-<triple>` + payload branches, action.yml delta note,
  embedded `assemble.sh` retitled *the measured oracle*, v2 section), [Roadmap](spec/roadmap.md) (M1 = the
  binary + action, M1.5 = end-to-end real-packagers run, order paragraph rewritten),
  [Command Surface](spec/cli.md) note, [Bootstrap](spec/bootstrap.md) lead,
  [Error Taxonomy](spec/error-taxonomy.md) shell-obligations intro, [Running the Action](workflows/action-run.md)
  (target one-liner is now the binary; publishing steps call CI verbs),
  [Airlock Clone](workflows/airlock-clone.md) ladder note, [Dogfooding](workflows/dogfooding.md) §7.7,
  [What Is Still Unproven](workflows/unproven.md) ("settled by D19 + D21" note),
  [Problem](overview/problem.md) delivery bullet, root [index](index.md), and the repo-root `README.md`,
  `AGENTS.md`, `context.md`. **M0 (spec) is closed by this acceptance.** Next
  milestone: M1.
* **Propose D21** (user question: "what if the action were executed by a Rust binary and shipped an assembler
  *binary*, not sh?"): new decision row in [Decision Log](spec/decisions.md) (now D1–D21) plus two new spec
  concepts — [The Assembler Binary](spec/assembler-binary.md) and
  [Testing Strategy](spec/testing-strategy.md). One Rust crate, two surfaces: the action runs the CI verbs
  (`pack`/`publish`), the kit ships the same binary as its assembler (`reconstruct`); it is prebuilt
  musl-static, mirrored into `bin/` with a sha256 exactly like the `pixi`/`pixi-unpack` pair, and **installed by
  nobody** (no conda-forge route, no git-install). The measured 183-line `assemble.sh` is frozen as the
  **behavioural oracle** — its nine outcomes become the Rust assembler's acceptance vectors. Distribution keeps
  every D19 promise (action + branch + one-liner, verify-before-execute); the D11 bootstrap stays dead because a
  mirrored blob is built in CI (networked) and consumed in the airlock. Counts move to **55 concepts / 63 files**
  (spec 16 → 18). Per the user's file constraint this turn wrote **only markdown** (`.knowledge/**`, root
  `README.md`, `AGENTS.md`, `context.md`); the two throwaway shell prototypes drafted mid-turn were deleted and
  their design content folded into the new concepts instead.
* **Harden**: `assemble.sh` grew from the 105-line first draft to the **183-line embedded script** in
  [The Action Shape](spec/action-shape.md), and [Running the Action](workflows/action-run.md) now measures
  **nine outcomes** (the earlier seven plus two new classes, all re-run in `/home/user/proto`, `sh`, stubbed
  drivers): rung 2 via the kit's `file://` channel and rung 4 via a foreign `micromamba`/`conda` when the kit
  ships no `pixi-unpack`, and an **explicit refusal** (exit 5, `E-UNAVAILABLE`) when a channel-shaped pack has no
  rung above the tar floor reachable — never a blind `tar -xf` that masquerades as success. Three contract
  findings went into the script and the Error Taxonomy pointer in that workflow: **drivers are resolved by
  provenance** (the kit's own `bin/` wins, a foreign `pixi-unpack` on `PATH` only earns a `W-PROVENANCE` warning,
  and `$BIN/.pixi-sandbox-kit` records which kit last wrote it); `--pack-format auto|pack|raw` tells a
  pixi-pack tarball from a raw prefix tar by listing the archive, so the floor rung never claims rung 3; and
  **PATH exposure is part of reconstruction** (`$GITHUB_PATH` gains the driver dir so later CI steps and jobs see
  `pixi`, every workspace gets a sourceable `assemble.env`, `--promote-path` appends to rc files idempotently,
  and the user's `~/.local/bin` is left alone). Counts stay **53 concepts / 61 files**; the four anchor links to
  the measured-outcomes heading were retitled to `the-nine-outcomes…` bundle-wide.
* **Add**: [The Action Shape (v1)](spec/action-shape.md) and [Running the Action](workflows/action-run.md) —
  D19 replaces the v1 binary with a public GitHub Action (`action.yml`: `branch`, `environments`, `platforms`,
  `bundle-crates`, `ship-pixi`, `push`) plus one 105-line POSIX script, `assemble.sh`, which the action ships
  inside every kit. The branch layout, the four CI-push rules (`contents: write`; `GITHUB_TOKEN` pushes do not
  retrigger workflows; fork PRs stage with `push: false`; blobs are forever) and the relocation trap — why a `tar`
  of `.pixi/envs` is rung 5 and not a kit — are in the spec concept, with `assemble.sh` embedded verbatim.
  Counts move to **53 concepts / 61 files** (spec 16, workflows 10).
* **Update** (D20, user instruction): **the `node_modules` component is dropped** — `node` and `npm` exist on this
  host and the JS surface that matters (the docs site) is built in CI, which also removes the largest payload class
  (227 MB measured). `nodejs`/`bun` still travel as conda packages inside env packs, so
  [research/node-bun.md](research/node-bun.md) stays as the evidence for *that*. `node pack` / `node sync` are off
  the verb list and `[node].pack` is reserved (returns `not-supported`); `bun ci` in CI remains as a lockfile
  assertion. Touched: `overview/{problem,requirements,goals}`, `spec/{cli,artifacts,packagers,configuration,
  roadmap,decisions,risks,git-registry}`, `workflows/{lockfile-digest-map,create-environment}`, root `index.md`,
  `README.md`.
* **Update**: [Goals](overview/goals.md) and [Dogfooding](workflows/dogfooding.md) §7.7 — D1.5 moved from
  "not started" to measured. `assemble.sh` ran here against a fixture kit with `pixi`/`pixi-unpack`/`cargo`
  stubbed on `PATH`, producing seven recorded outcomes (rung 3; rung 5 with a warning; exit 4 before executing a
  payload byte; exit 8 on `cargo metadata` 101; exit 7 for a named-absent env; exit 5 for a missing `bin/pixi`;
  `bash -n` clean on the script). Two contract findings went into
  [Error Taxonomy](spec/error-taxonomy.md): `set -eu` leaks foreign exit codes unless every external command maps
  its own failure, and `tar -C` needs the prefix directory to exist first.
* **Update**: [Command Surface](spec/cli.md) gained the v1-delivery note (which rows became CI scripts, which stay
  a binary); [Create an Environment](workflows/create-environment.md) §1.2 the CI-side twin with `push: false` as
  `--dry-run`'s substitute; [Airlock Clone](workflows/airlock-clone.md) §2 a pointer to the script that automates
  the ladder; [Roadmap](spec/roadmap.md) milestone M1.5. `pixi sandbox --self-test` is no longer a needed concept —
  the script is testable in the sandbox, which is the reason the flag existed.
* **Update**: [Research: pixi-pack](research/pixi-pack.md) §16.7 records the measured release assets (`v0.7.11`,
  16 assets, **every one with a `sha256` digest** ✅), which is what justifies `ship-pixi` shipping the
  `pixi` + `pixi-unpack` **pair**; [The Action Shape](spec/action-shape.md) shows both under `bin/`.
* **Note**: the validator is rebuilt and re-documented in [Bundle Conventions](conventions.md) item 6 — it lives
  at the workspace root (`../val.py` from the bundle) because the bundle is markdown-only, is dependency-free by
  design, and its checks are contract text, so a lost file is reconstructible from that list. `ALL CLEAN` on
  61 files · 226 internal links · 78 anchors · 141 sources.
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
