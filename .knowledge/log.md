# Bundle update log

History of this corpus. The dates are when the material was authored or corrected in the session that produced
it; `legacy:` front matter on each concept names the pre-OKF file and section it came from.

## 2026-09-19

* **Rename** `context.md` → **`CONTEXT.md`** (user question: "should context.md be uppercased?") — yes:
  the ecosystem spelling in [AGENTS.md and CONTEXT.md conventions](research/agent-instruction-files.md) is
  uppercase (unlike `AGENTS.md`, where all-caps is tool-*required*, `CONTEXT.md` uppercase is convention
  only — no tool discovers it by name), and it puts the three root meta-files (`AGENTS.md`, `CONTEXT.md`,
  `README.md`) in one visually consistent group. References updated in `AGENTS.md` (link, deliverable
  list, layout table) and in the workspace validator's root-file list; dated log entries above keep the
  spelling they used at the time, by design.
* **Add** [convco: conventional-commit linting and changelog generation](research/convco.md) (user: "add
  convco via the conda-forge channel via pixi, so I have changelog and commit linting done for repo;
  web search proper documentation and update KB") — researched from upstream first-hand ✅: command surface
  (check takes `<commit>..<commit>` ranges with `--ignore-reverts`/`--merges`; changelog has
  `-o/-s/-m/-u/--paths`; version computes current + `--bump`; config in `${PWD}/.convco`, defaults via
  `convco config --default`), the official GH Actions PR-range recipe, and the feedstock facts:
  **conda-forge v0.7.2** (bot-automerge 2026-09-03) building **all six subdirs including win-64** — the
  contrast to bun's missing win-64 recorded. Adopted into the workflow set ([ci-workflows](spec/ci-workflows.md)):
  `convco >=0.7` + `commit-lint`/`changelog` tasks in the `lint` feature; a PR-only `commit-lint` job in
  `ci.yml`; `release.yml` gates gain the `convco version --bump` semver cross-check and its release notes
  become `convco changelog -s -m 1` at the tag — one convention, three consumers (§9 item 14). The
  official recipe's curl-from-release-assets install is deliberately replaced by the pixi/conda-forge
  route (one dependency manager, one lockfile pin). Honest limits: the tool never ran in this sandbox
  (conda blocked ❌) — flags are doc-verified ✅, integration ⚠️ reasoned. Counts move to
  **58 concepts / 66 files** (research 16 → 17).
* **Decide** (user: "the binary shouldn't be fetched from the airlocked host — it's fetched inside the
  action, in CI; the user only defines `sandbox-environment.yml` and calls one command on the airlocked
  machine to reassemble"): the binary's journey is now stated as **CI-only until the kit exists** — release
  assets and the `pixi-sandbox-bin` branch are channels CI consumes (the action's digest-pinned fetch step),
  `publish` embeds the bytes in the kit's `bin/` under `SHA256SUMS`, and the airlocked host clones exactly
  one branch and contacts nothing else. The far-side UX collapses to
  `git clone --depth 1 --branch pixi-sandbox-dist <url> kit && sh kit/assemble`:
  [The Assembler Binary §2](spec/assembler-binary.md) is retitled **The one command** and the same-blob
  `assemble` ergonomics are **rejected** (a compiled binary is arch-specific — the entry is a ~30-line POSIX
  text shim instead, digest-covered like every payload, so one kit can carry both P0 triples);
  [Airlock Clone](workflows/airlock-clone.md) gains the one-command headline with the manual ladder demoted
  to specification + fallback; [the user template](workflows/sandbox-template.md) §5 documents the shim's
  three responsibilities and its `verify` job now dogfoods the exact user command;
  [The Repository's Workflow Set](spec/ci-workflows.md) §5 marks `pixi-sandbox-bin` CI-only in the channel
  table; the README one-liner drops the stale `assemble.sh` spelling.
* **Add** 🚧: [Publishing Your Environment: the sandbox-environment Template](workflows/sandbox-template.md)
  (user: "I would like to allow users to just copy my own sandbox-envs workflow so they use the same format
  and it should be documented in docs") — the user-facing integration playbook: one workflow file, six
  knobs (`paths:` filter per the lockfile→digest map, cron, dist branch, environments, platforms ↔ runners,
  the verify task), publish → verify → pin, the D19 rules restated as "what you are accepting", and the
  verified gotcha that `uses:` accepts no expressions ✅ (actions/runner#895), which is why the action ref
  is the template's one literal line (pin a SHA). The author's own `sandbox-environment.yml` becomes the
  *dogfood rendering* of this template via two declared markers (`uses: ./`, the extra `dogfood-released`
  job), with a `ci.yml` L0 drift check — one source of truth, two renderings, drift is a test failure.
  The docs site renders the concept unchanged (docs-sync copies `.knowledge/**` ✅). Counts move to
  **57 concepts / 65 files** (workflows 10 → 11).
* **Clarify** (user question: "should the pixi-sandbox binary be released along with the action or can I just
  attach a build artifact?"): [The Repository's Workflow Set](spec/ci-workflows.md) §5 gains the
  **three-channel rule** — workflow artifacts are the internal build→release handoff only (90-day retention ✅);
  release assets are the tag-immutable public record and the airlock-checkable `digest` anchor
  (api.github.com open ✅); the `pixi-sandbox-bin` branch is the operational channel the action fetches,
  because `objects.githubusercontent.com`/`release-assets.githubusercontent.com` are measured closed ❌
  ([Constraints](overview/constraints.md)) and D1's verdict is "git objects, not release assets" for anything
  a target consumes. Answer: **both** — the release ships the binaries (marketplace requires a Release anyway),
  the branch mirror stays load-bearing, artifacts alone are never enough. §9 item 13 added.
* **Add** 🚧: [The Repository's Workflow Set](spec/ci-workflows.md) (user: "reusable action that caches pixi
  environment … deploy-docs.yml … ci.yml … release.yml … build.yml … sandbox-environment.yml … suggest how
  actions/workflows should be named and split") — the file topology for this repo's own CI, drafted and
  reconciled with D1–D21: five workflows (`ci.yml` guards without paths filters; `build.yml` owns the P0/P1
  matrix and the oracle job that replays the nine outcomes as a *release gate*; `release.yml` ships a version
  — release → `pixi-sandbox-bin` branch → digest-pin commit → floating `v1` → the marketplace listing, which
  is the one documented manual step because the API cannot set it ✅; `deploy-docs.yml` builds with the pixi
  `docs` env via `pixi run -e docs build:docs` → `bun install --frozen-lockfile` → `bun run build` and
  uploads with `upload-pages-artifact` v5 — whose since-v4 dotfile exclusion settles the `.nojekyll`
  register entry; `sandbox-environment.yml` dogfoods `uses: ./` on every main push, verifies its own output
  in an alpine container, pins `sandbox.lock.json`, and re-runs the released `@v1` on cron as the G9 tier) +
  two actions (the root product `action.yml` — the only one the marketplace lists ✅ docs — and a local
  `.github/actions/setup-env` composite that pins the setup-pixi *policy*: `cache-write` on main only,
  explicit `environments`, locked installs; machinery stays upstream's ✅). Status `draft` — user review
  pending; counts move to **56 concepts / 64 files** (spec 18 → 19).
* **Fix**: two malformed `sources:` flow maps — stray `"]` fragments from body inline-code had leaked into
  [Artifact Formats](spec/artifacts.md) (`mirroracme-` → `mirroracme`) and
  [pixi ↔ cargo interop](workflows/pixi-cargo-interop.md) (`prefixdev-conda-forge`); repaired in place,
  intent preserved.
* **Tooling**: the workspace-root `val.py` was lost in a snapshot restore and is **reconstructed from
  [conventions item 6](conventions.md)** — zero-dependency parser for the bundle's two front-matter shapes,
  `type`/`confidence`/reserved-name checks, link + anchor resolution (GitHub slug rule; intra-word
  underscores kept), reachability BFS from the root index (directory links followed to `index.md`),
  code-fence parity, and stale pre-OKF-link detection. Baseline and post-edit runs: `ALL CLEAN`.

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
