# Bundle update log

History of this corpus. The dates are when the material was authored or corrected in the session that produced
it; `legacy:` front matter on each concept names the pre-OKF file and section it came from.

## 2026-09-19 (compressed-env restore)

* **Measure** (user: "look at branch compresses-env and check if you could restore it on current branch"):
  * Found `compressed-env` (typo in request) at `aa2a6e1` — orphan branch with 15 files: `pixi-bin.tar.gz` (33 MiB gz → 77 MiB pixi 0.81.0), `cargo-vendor.tar.gz` (15 MiB → 124 crates), `env-dev.tar.gz.part_00..08` (398 MiB), `env-docs.tar.gz.part_00..01` (67 MiB), `env-utils.tar.gz` (18 MiB). Chunked to ≤45 MiB to stay under GitHub 100 MiB limit without LFS.
  * **File type surprise**: `env-*.tar.gz` are **plain tar, not gzipped** (magic `channel\0\0\0`), so `tar -tzf` fails with `not in gzip format`; `tar -tf` succeeds. Same for `env-utils`. `pixi-bin.tar.gz` and `cargo-vendor.tar.gz` are real gzip.
  * **CLI drift**: branch README says `pixi-unpack unpack env-dev.tar.gz --output-dir .pixi/envs/dev` — stale. Real 0.7.11 syntax (measured from binary) is `pixi-unpack <pack> -o <out> --env-name <name>`; `-o` is parent dir, env becomes subdir. Correct restore: `pixi-unpack env-dev.tar.gz -o .pixi/envs --env-name dev` → `.pixi/envs/dev/bin/rustc`.
  * **Bootstrap chicken-egg**: dev env contains `pixi-unpack` and `pixi-pack` conda packages, but need `pixi-unpack` to unpack dev env. Solved by manually extracting conda `.conda` (zip → `pkg-*.tar.zst`) via python `zstandard` module (pip install with `--break-system-packages` since `zstd` CLI absent and `tar --zstd` fails `Cannot exec: zstd`). `pip install zstandard` works (pypi.org reachable even when prefix.dev blocked). Extracted `bin/pixi-unpack` (16 MiB) and used it to unpack all three envs.
  * **Cargo vendor SIGPIPE**: `tar -xzf cargo-vendor.tar.gz -C .cargo/ -v | head` stops after SIGPIPE → only 2 crates extracted, `flate2` missing → `cargo build --offline` fails. Must run `tar -xzf` without pipe. Full extract → 124 crates, `cargo build --offline` green, `cargo test --offline` green.
  * **Network re-probed**: this arena host is E2B-like airlock: `prefix.dev`, `index.crates.io`, `release-assets.githubusercontent.com` → `SSL_ERROR_SYSCALL` (000), but `github.com`, `api.github.com`, `codeload.github.com` → 200. Validates design target. `pip install zstandard` still works (pypi.org reachable).
  * **Pixi tasks**: `pixi run lint` → `lint-cargo` (clippy + fmt check) green, `lint-biome` green (8 files), `lint-actions` skipped (no workflows). `.pixi/envs/*` gitignored, so branch stays clean after restore (1.7 GiB dev, 255 MiB docs, 55 MiB utils).
  * **Design lessons**: kit must ship **both** `pixi` and `pixi-unpack` binaries (pair, not single) in `bin/`; `pixi-bin.tar.gz` only had pixi, causing bootstrap. Chunked tar naming `.tar.gz.part_*` misleading when payload is tar; prefer `.tar.part_*` or document. Assembler binary (D21) must handle plain tar vs gz detection and include zstd handling. `sandbox.lock.json` should record chunking.

## 2026-09-19

* **Tooling change** (user: "remove python val script from repo and remove rule that enforced this, install
  pixi from installer script"):
  * **Removed the workspace-root `val.py`** and the rule that ran it — AGENTS.md hard-rule bullet gone;
    `CONTEXT.md` "Do:" no longer says `python3 val.py` / `ALL CLEAN` and the "validator must be rebuilt from
    conventions item 6" note is deleted; [Bundle Conventions](conventions.md) editing rules lose item 6 (the
    validator contract). No validator checks stand in — bundle-integrity claims now rest on the Starlight
    links validator in CI (`docs` job) and on review, which is a confident-label change worth a read.
  * **Cleaned every reference** that assumed a live validator: `spec/ci-workflows.md` drops the
    `checks-l0`'s `python3 ../val.py` step and the L0 mention (js-yaml remains); `spec/ci.md` and
    `workflows/sandbox-template.md` reword the "val.py" clauses; `index.md` drops "as checked by the repo
    validator"; the dated validator-reconstruction log entries above kept their spelling by design.
  * **Installed pixi from the official installer script** (`curl -fsSL https://pixi.sh/install.sh | bash`):
    **0.81.0** musl-static at `~/.pixi/bin/pixi`, on `PATH` via `~/.bashrc`, `pixi info` green. The §16.8
    probe's **0.80.0 conda-forge** measurement stands; the installer-script route is now the documented
    way to get pixi on this host ([inventory §6](environment/inventory.md#6-re-measurement-2026-09-19-the-host-this-corpus-is-now-edited-on)).
* **Propagate** the host re-measurement (E2B box closed; the current authoring host runs the whole stack from
  conda-forge) through the corpus, and **measure what the previous turn had referenced but not written**:
  * **Add** the **§16.8 probe to [pixi-pack](research/pixi-pack.md)** — the exact run the re-measured
    [environment](environment/index.md) rows point at. Fan out on this host, all binaries from conda-forge
    (`pixi` 0.80.0, `pixi-pack`/`pixi-unpack` 0.7.11): `pixi init/add libcurl/install` (19 packages) →
    `pixi pack` → `environment.tar` **29.77 MiB** with the §16.4 layout verbatim (`channel/linux-64` +
    `channel/noarch` + `pixi-pack.json`) → `pixi-unpack` (writes `conda-meta/history`, emits `activate.sh`;
    `source activate.sh` serves the prefix's own binaries) → a *consumer* project pointed at the pack's
    `file://` channel via `pixi project channel add` reconstructs with `pixi add --offline` +
    `pixi install --offline` + `pixi run icuinfo`, **no channel access**. So the §16.6 "single probe that still
    moves a default" is **green: rung 2 is the guarantee** and `cache/pkgs/` is optional (the sealed-container
    `--frozen` variant stays M1.5). Also settles **§16.7's open tail**: `pixi-pack`/`pixi-unpack` **are on
    conda-forge** at exactly the GitHub release version (`0.7.11`) — the old `000` from api.anaconda.org was the
    E2B box, so `conda create -c conda-forge pixi-pack=0.7.11 …`, not `pixi global install`, is the measured
    route. Quirk recorded: bare `pixi-pack pack` parses `pack` as the manifest path and errors; `pixi pack`
    (pixi's proxy) builds the right argv.
  * **Move rows in [What Is Still Unproven](workflows/unproven.md)**: pack-as-a-channel ⚠️→✅ measured; the
    CI-half row is driver-real for pixi-pack (full action flow still needs a runner); the D21 "no rustc here"
    justification is retired (conda-forge `rust` 1.98.1 runs here — only the Oracle-vs-binary gate is CI-shaped).
  * **Dated notes** on the two test plans so an M1 agent is not misled: [Testing Strategy](spec/testing-strategy.md)
    (L1 is now locally executable; L3's drivers proven real, sealed-container run still the gate) and
    [Testing Without a Compiler](spec/testing.md) (shell-era, E2B-dated). Root [index](index.md) gains the same
    "read the date" warning as the environment files; `CONTEXT.md` line 3 rewritten.
* **Measure the convco adoption** ([research/convco.md](research/convco.md)) — the documented limit "the
  tool never ran in this sandbox" is resolved the same day on the 2026-09-19 host: `convco` 0.7.2 installed
  both as a raw conda-forge binary and through the exact `feature.lint` manifest (`pixi install -e lint`,
  the D8 pattern, `>=0.7` → lockfile-bound 0.7.2), then exercised against *this repo's own history* with
  `-C /workspaces/pixi-sandbox` and the ci.yml range shape: `pixi run -e lint commit-lint -- -C <repo>
  <base>..<head>` forwards the range verbatim (the `--` passthrough works); a bad commit prints
  `FAIL <hash>  first line doesn't match '<type>[optional scope]: <description>'`, exit 1; a range spanning
  only merge commits (this repo's `1a07676..HEAD`) is `no commits checked`, exit 0 (`merges: false`
  default); `changelog -s -m 1` on this tagless repo is just `# Changelog`, so the tagged-section rendering
  and `convco version --bump` gate stay ⚠️ until the first tag exists; and a range whose **outer boundary
  is the root commit** fails `parent 0 does not exist` — a fresh-repo gotcha that ci.yml ranges
  (`origin/main..HEAD`) can't hit. Also **corrected the `.convco` sketch**: `convco config --default`
  (0.7.2) has **no `scopes` key** — scope validation is the `scopeRegex` key (default
  `^[[:alnum:]]+(?:[-_/][[:alnum:]]+)*$`), and description `min: 10` is the default gate.
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
