---
type: Design Spec
title: "The Three Packagers"
description: What pixi, pixi-pack and cargo vendor each do and cannot do, and how the tool composes them.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, packagers]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["8"] }
sources:
  - { id: latchkeydev-learn-rust, resource: https://latchkey.dev/learn/rust/cargo-failed-to-load-source-replacement-in-ci, title: latchkey.dev/learn/rust/cargo-failed-to-load-source-replacem }
  - { id: wwwrustfaqorg-en-how-to-vendor-dependencies-in-rust-, resource: https://www.rustfaq.org/en/how-to-vendor-dependencies-in-rust-with-cargo/, title: www.rustfaq.org/en/how-to-vendor-dependencies-in-rust-with-c }
  - { id: githubcom-rust-lang-cargo, resource: https://github.com/rust-lang/cargo/issues/10729, title: rust-lang/cargo — issues/10729 }
  - { id: linuxcommandlibrarycom-man-cargo-vendor, resource: https://linuxcommandlibrary.com/man/cargo-vendor, title: linuxcommandlibrary.com/man/cargo-vendor }
  - { id: buncom-docs-pm, resource: https://bun.com/docs/pm/cli/install, title: bun.com/docs/pm/cli/install }
  - { id: githubcom-rust-lang-cratesio-index, resource: https://github.com/rust-lang/crates.io-index, title: rust-lang/crates.io-index }
  - { id: githubcom-rust-lang-cratesio-index, resource: https://github.com/rust-lang/crates.io-index, title: rust-lang/crates.io-index }
---

# The Three Packagers

## 8. The three packagers

### 8.1 `pack` — pixi environments → `environment.tar`

**Any environment, zero config edits (requirement (a)).** `pack` takes `ENV…` positionally, plus
`--all-envs`, and resolves the manifest from the inventory — so `pixi sandbox pack
--manifest-path ../someone-else/pyproject.toml --env training --env gpu` works on an environment declared under
`[tool.pixi.environments]` ✅ that no `pixi-sandbox.toml` has ever heard of. An unrecognised name is a
`not-declared` error that **lists what was detected** (L1 `environments` + L2 lockfile keys), never a
guess and never a silent skip. Cross-selection is explicit: `--env` and `--target` form a cartesian
product, `plan` prints each cell, and P0/P1 veto per cell
([§7.4](/spec/toolchain-resolution.md#74-platform-validation-does-this-platform-actually-exist)).

Delegates to **pixi-pack**, whose contract was read from source (v0.7.11) rather than assumed ✅:

```bash
pixi-pack --environment <ENV> --platform <PLAT> --output-file <OUT>/<name>-<env>-<plat>.tar \
          [--use-cache <DIR>] [--ignore-pypi-non-wheel] [--inject <pkg.conda>…] <manifest>
```

* Upstream is **`Quantco/pixi-pack`** ✅ (not prefix-dev) and its binaries carry GitHub **Artifact
  Attestations**: `gh attestation verify --repo Quantco/pixi-pack pixi-pack-<arch>` ✅ — which is how
  `mirror binary` should record trust in `dist-manifest.json` for a binary we then ship, instead of asking
  the target to trust our copy on faith.
* **`--platform` accepts conda subdirs *or* named platforms** (`jetson`) when the lockfile is v7 →
  the config stores strings, not a Rust enum, so named platforms pass through. ✅
* **`--use-cache <dir>` is a *channel-shaped* cache** — *"the cache follows the same structure as conda
  channels, organizing packages by platform subdirectories"* ✅, and it exists precisely for *"operating in
  environments with limited bandwidth"* and CI reuse ✅. Two uses we take from that: the build box's
  repeated packs never re-download, and **the kit's `cache/pkgs` can be that very directory**, which is
  documented artifact (pixi-pack wrote it) rather than a copy of pixi's private cache — a stronger claim than
  rung 1's, so `reconstruct` prefers it when it cannot verify a rattler cache layout. ⚠️ Whether pixi
  consumes that dir as a *cache* (vs. as a channel) is unverified; as a `file://` channel it is rung 2 ✅.
* **`--create-executable`** writes `environment.sh` / `environment.ps1` (self-extracting, and it
  embeds or references a `pixi-unpack` binary — overridable with `--pixi-unpack-source`, which is *the*
  lever for "download the unpacker from our git repo instead of GitHub releases"). ✅
* **`--directory-only`** (`conflicts_with = create-executable`) — supported for "give me a local channel
  directory, no tar", handy inside a Dockerfile. ✅
* **Cross-platform is free** (it downloads `.conda` files per subdir), so we always pack
  *target* platforms from the *build* machine ✅; **unpacking is same-platform only** ✅ — hence a kit is
  never "one tar for everyone", it is `name-<plat>.tar` per platform, and `apply` selects by host.
* Unpack fallback chain: `pixi-unpack` → `conda env create` → `micromamba create -p ./env --file
  environment.yml` → **`tar -xf` + use `channel/` as a local channel** (last resort needs only `tar`,
  which is what makes a kit usable in a bare sandbox). ✅ documented by pixi-pack.
* PyPI **sdists are unsupported** by pixi-pack (flag exists to ignore them) → `plan` marks such
  actions `warn: pypi-sdist` and `--strict` turns it into a failure. ✅

* **Second packager, same verb — `pack --format environment-yml`.** `pixi project export
  conda-environment --environment <E> <E>.yml` ✅ produces an `environment.yml` per environment, which is
  the artifact `micromamba create -f` consumes (rung **R4**, [§9.4](/spec/artifacts.md#94-reconstruction-making-pixi-run-work-offline)).
  It matters here for a specific reason: **pixi-pack is not installed in this sandbox and its release
  assets are blocked** ✅, while a `pixi` binary + a lockfile are enough to reproduce an environment. So
  the export path is a supported primary route, not a consolation prize — and `pixi sandbox kit build
  --components env` emits both when both tools exist, letting the target choose.

Output naming: `<workspace>-<env>-<platform>[+<feature-set>].tar` (or `.yml` for the export route), so two
environments never collide on one disk.

### 8.2 `vendor` — cargo deps → `vendor/` (+ optional branch mirror)

> [!IMPORTANT]
> **`pixi-pack` cannot carry the crate graph, and that is not a gap to engineer around.** A pack contains the
> conda packages named by `pixi.lock` (plus PyPI **wheels**) ✅; your crate's dependencies live in
> `Cargo.lock` + `$CARGO_HOME/registry` and appear in neither. `--inject` accepts `.conda`/`.tar.bz2` and
> `.whl` ✅ — not `.crate`. So this component exists for one question: **does the target compile?** If not,
> ship the built artifact (`--inject mybin-….conda` ✅, whose documented use case is literally *"you build the
> project itself and want to include the built package… but still want to use `pixi.lock`"*) and set
> `[kit] targets-compile = false` — `vendor` then drops out of `auto`, and the kit stops carrying crates it
> will never compile (a CLI of this shape has **819** lockfile packages ✅ measured from pixi-pack's
> `Cargo.lock`). Full analysis:
> [pixi and cargo: What Travels Where §4](/workflows/pixi-cargo-interop.md#4-pixi--cargo-interoperability-and-what-pixi-pack-does-not-carry).

When the target *does* compile, the crates need a **carrier** — `[vendor] format` picks between a directory
source (`cargo vendor`, which has **no** `--target` flag ✅ so it carries every platform's crates), a
`local-registry` of `*.crate` + a crates.io-format index ✅ (≈¼ the bytes), a copied `$CARGO_HOME` after
`cargo fetch --locked --target <triple>` ✅ (the only target-scoped option; layout is an implementation
detail ⚠️), or none at all. And the wiring is a separate choice, because the pair cargo documents —
`[source.crates-io] replace-with = "vendored-sources"` plus `CARGO_NET_OFFLINE=true` (the env-var twin of
`--offline`, ✅ `net.offline` config value) — can be installed three ways: committed `.cargo/config.toml`,
per-invocation `cargo --config KEY=VALUE` ✅ (what `reconstruct` uses on a foreign repo, since `cargo vendor`
prints exactly this config to stdout ✅), or a kit-owned `$CARGO_HOME/config.toml` (needed when the task text
hard-codes `cargo build`).

```bash
cargo vendor --locked --versioned-dirs [--features …] [--respect-source-config] <dir>   # full
cargo vendor --locked --sync <dir>                                                       # incremental
```
then write into `.cargo/config.toml`:

```toml
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
```

`vendor check` (the CI gate) asserts **four** things, each a distinct failure with its own code:

1. `Cargo.lock` exists and is up to date (`cargo metadata --locked` succeeds).
2. `.cargo/config.toml` contains a replacement for **exactly** the registry being used
   (the classic error is *"replacement is not configured for source
   `registry+https://github.com/rust-lang/crates.io-index`"* ✅ [latchkey](https://latchkey.dev/learn/rust/cargo-failed-to-load-source-replacement-in-ci)).
3. **Offline resolution succeeds: `cargo metadata --locked --offline`** — the real proof that `vendor/`
   covers every dep, including *transitive* ones and target-specific ones.
4. No `build.rs` in the graph downloads anything (a **known hole**: vendoring doesn't cover build-script
   network I/O ✅ [rustfaq](https://www.rustfaq.org/en/how-to-vendor-dependencies-in-rust-with-cargo/)).
   Implementation: scan vendored `build.rs` files for `reqwest|ureq|curl|wget|Command::new("git")` and
   *report* matches as warnings with the crate name — cheap heuristic, honest label `heuristic=true`. 🚧

Two more cargo gotchas encoded as behaviour:

* **`cargo add` is unsafe while a replacement is configured** (it fuzzy-matches inside `vendor/` ✅
  [cargo#10729](https://github.com/rust-lang/cargo/issues/10729)) → `vendor` prints: *"to add a
  dependency, run `cargo add` **before** `pixi sandbox vendor sync`, or temporarily
  `pixi sandbox vendor disable`"*.
* **Vendored trees are read-only**; use `[patch]` to change a crate ✅
  [linuxcommandlibrary](https://linuxcommandlibrary.com/man/cargo-vendor) → `vendor` refuses to write
  inside `vendor/` and says so.
* `--offline` in `check` is what lets this run *inside* a crates.io-blocked sandbox: **verification is
  always possible, only generation needs egress**. That asymmetry is the whole reason this design works
  here at all.

### 8.3 `node` — **retired in v1** (D20), kept here as the reasoning

* `node sync` → `bun ci` (preferred; equivalent to `bun install --frozen-lockfile`, which errors if
  `package.json` and `bun.lock` disagree ✅ [bun docs](https://bun.com/docs/pm/cli/install)), or
  `npm ci`, `pnpm install --frozen-lockfile` by detection.
* **Text lockfile only**: `bun.lock`, never the pre-1.2 binary `bun.lockb` ✅ — a binary lockfile in git
  defeats the "diffable + auditable" premise, so `node check` **fails** on `bun.lockb` with the migration
  command as remediation (`bun install --save-text-lockfile --frozen-lockfile --lockfile-only`).
* `node pack` → deterministic tar of `node_modules` → `node_modules.<platform>.tar.gz`, with
  `--exclude` (defaults: `.cache`, `*.map`), `--sort`/`--mtime 1970-01-01` normalisation so re-packing
  the same tree yields the same SHA-256 (⚠️ needs `tar` ≥1.28; on Windows use bsdtar `tar.exe`; verify
  flags per flavour at runtime rather than assuming GNU tar).
* ⚠️ *Historical:* **why a tarball was ever proposed**, when pixi could ship `nodejs`: because `node_modules` contains *platform-native*
  optional deps (esbuild/swc/@rollup) that conda won't resolve for you — same problem pixi solves for
  conda packages. But note bun's install cache layout is explicitly documented as unstable ("don't think
  of it as an API" ✅), so we snapshot the **`node_modules` tree only**, never `~/.bun/install/cache`, and
  never treat layout as portable across bun versions (record `bun --version` in provenance and key the
  artifact name on it).
* `--global` installs are **not** managed (out of scope; would collide with `pixi global`).

---
