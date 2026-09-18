---
type: Design Spec
title: "Artifact Formats and Integrity"
description: Artifact layout on the dist branch, manifests, channel mirroring, and the reconstruction rung ladder R1-R5.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, artifacts, integrity]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["9!"] }
sources:
  - { id: githubcom-prefix-dev-setup-pixi, resource: https://github.com/prefix-dev/setup-pixi, title: prefix-dev/setup-pixi }
  - { id: githubcom-googlecloudplatform-knowledge-catalog, resource: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md, title: GoogleCloudPlatform/knowledge-catalog — blob/main/okf/SPEC.md }
  - { id: githubcom-quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
  - { id: mirroracme-, resource: https://mirror.acme/"], title: mirror.acme/"] }
---

# Artifact Formats and Integrity

## 9. Artifact formats and integrity

### 9.1 `sandbox.lock.json` — the integrity manifest

The one file that makes everything auditable. Committed on the **working branch** (small, text), while
payloads live on the **dist branch** (large, binary).

```jsonc
{
  "schema-version": 1,
  "generated": { "by": "pixi-sandbox/0.1.0", "at": "2026-09-18T21:45:00+00:00" },
  "host": { "os": "linux", "arch": "x86_64", "platform": "linux-64" },
  "inventory": {                                  // what we DETECTED, and how confidently (§4.4)
    "tier": "L1",                                  // highest tier that ran
    "manifest": "pixi.toml", "manifest-digest": "sha256:…",
    "environments": { "rust": { "features": ["build"], "platforms": ["linux-64"],
                                "locked-packages": { "linux-64": 128 } },
                      "gpu":   { "platforms": ["linux-64"], "heuristic": true } },
    "components": { "env": "on", "vendor": "on (Cargo.lock)", "node": "off (no lockfile)",
                    "self": "on" }
  },
  "sources": {
    "rust":  { "kind": "conda", "channel": "conda-forge", "resolved": "rust-1.95.0-h0.0.1" },
    "bun":   { "kind": "npm",   "package": "bun", "resolved": "1.4.2",
               "note": "conda-forge lacks win-64; npm used for this kit" }
  },
  "artifacts": [
    {
      "id": "pack.rust.linux-64",
      "kind": "env-pack",
      "path": "sandbox/packs/pixi-sandbox-rust-linux-64.tar",
      "sha256": "…64 hex…",
      "bytes": 143215004,
      "environment": "rust",
      "platform": "linux-64",
      "transport": { "git-oid": "…", "ref": "pixi-sandbox-dist", "commit": "…" },
      "requires-hosts": ["prefix.dev", "conda.anaconda.org"],
      "produced-by": "pixi-pack/0.7.11",
      "lockfile-digest": { "pixi.lock": "sha256:…" }   // ties artifact -> inputs
    },
    { "id": "cache.rust.linux-64", "kind": "rattler-cache-subset",
      "path": "sandbox/cache/pkgs/", "provides-rung": "R1", "bytes": 96400000 }
  ],
  "reconstruction": { "preferred-rung": "R1", "guaranteed-rung": "R3",
                      "needs": { "R1": ["bin/pixi-x86_64-unknown-linux-musl", "cache/pkgs"] } },
  "omitted": [ { "id": "pack.bun.win-64", "reason": "unavailable-platform", "detail": "…" } ]
}
```

Design points: `lockfile-digest` answers *"is this artifact current?"* without re-solving (the same trick
`setup-pixi` uses to key its cache ✅ [1](https://github.com/prefix-dev/setup-pixi)); `omitted` makes
degradation visible; `requires-hosts` is the sandbox-aware field; `transport` lets a consumer choose
"read the blob from git" vs "read it from the local cache". Timestamps are ISO 8601 **with explicit
offset**, borrowed from OKF's rule ✅ [SPEC §5](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) — it is the difference between a kit you can order and one you can't.

Two fields are new in this revision, both because requirements (a)–(e) changed what a kit *promises*:

* **`inventory` records what detection concluded, at which tier.** So `git log -p sandbox.lock.json`
  answers "why did the kit contents change?" with something better than a diff of file lists: an
  environment appeared, a platform stopped being locked, `node` turned off because nobody committed a
  JS lockfile. Detection is only debuggable if it is *persisted*.
* **`reconstruction.preferred-rung` / `guaranteed-rung` split promise from hope.** A consumer (or CI gate)
  can assert "this kit can *reconstitute a workspace*" (`R1`/`R2` — and only R2 leaves an index you can
  `pixi add` against offline ✅) versus "this kit can *lay down a prefix*" (`R3`), and the `needs` map is the exact file list to check after a shallow clone — which is also what
  `kit verify` diffs against, so a missing `bin/pixi-*` is caught at build time, not on the target.

### 9.2 `dist-manifest.json` — the per-artifact index on the dist branch

A tiny file, **at the tip of the dist branch**, so `--depth 1` cloning gets everything needed to pick and
verify an artifact without fetching all blobs:

```jsonc
{ "schema-version": 2, "updated": "…", "artifacts": [
    // what we built — source is this repo
    { "name": "pixi-sandbox-x86_64-unknown-linux-musl", "kind": "binary", "role": "tool",
      "triple": "x86_64-unknown-linux-musl", "sha256": "…", "bytes": 8388608, "git-oid": "…",
      "source-rev": "<commit on main>", "source-version": "0.1.0", "toolchain": "conda:rust-1.95.0",
      "provenance": "built" },
    // what we mirrored — source is upstream, and the manifest says so (§10.5)
    { "name": "pixi-x86_64-unknown-linux-musl", "kind": "binary", "role": "driver",
      "version": "0.81.0", "sha256": "…", "bytes": 94000000, "git-oid": "…",
      "provenance": "mirror", "attestation": "gh attestation verify --repo prefix-dev/pixi pixi-x86_64-unknown-linux-musl",  # recorded, not assumed ✅
      "upstream": "https://github.com/prefix-dev/pixi/releases/download/v0.81.0/…",  # asset name to confirm,
      "upstream-sha256": "…", "notice": "NOTICE.md#pixi" },
    // payloads, keyed by (environment, platform) so consumers can select without downloading all
    { "name": "kit-packs-myproj-training-linux-64.tar", "kind": "env-pack", "role": "payload",
      "environment": "training", "platform": "linux-64", "sha256": "…", "git-oid": "…",
      "rung": ["R1","R2","R3"] },
    { "name": "kit-cache-myproj-linux-64.tar", "kind": "rattler-cache-subset", "role": "payload",
      "bytes": 96400000, "rung": ["R1","R2"], "optional": true } ] }
---
```

Consumers: `dist pull` (this host's triple), `kit apply`/`reconstruct` (needs `pixi` + the right rung's
files for the target), `update`-style self-checks, and *humans*. Because it is JSON at a fixed path,
`curl`-less tooling can still read it via `git cat-file`. `kind`/`role`/`provenance` are the three fields
that make "mirror" and "built" distinguishable in a security review — the `upstream-sha256` on a mirrored
entry is what `dist verify` re-checks, so a swapped blob in our branch is detectable *without* trusting
our branch.

> [!WARNING]
> The `upstream:` digest/asset-name fields are **not** filled in by this document: pixi lives at
> `prefix-dev/pixi` ✅ but its release assets are unreachable from this sandbox
> (`release-assets.githubusercontent.com` → blocked ✅), so the exact asset name and sha256 must be recorded
> by a `mirror binary` run on a networked machine. A manifest that pins a *wrong* digest is worse than one
> that pins none — which is precisely why `provenance: "mirror"` entries are refused by `dist verify`
> unless `upstream-sha256` is present.

### 9.3 Kit directory layout

```
sandbox/kit/
├── sandbox.lock.json
├── README.md                    # generated: what this kit is, what it omits, exact apply command
├── apply.sh                     # embedded bootstrap (POSIX; only needs tar + git)
├── apply.ps1                    # embedded bootstrap (Windows)
├── workspace/                   # enough for pixi to recognise a *workspace* on the target:
│   ├── pixi.toml                #   manifest, verbatim (or the `--from` manifest of an alien repo)
│   ├── pixi.lock                #   required — `install --frozen` must never need to re-solve
│   └── pixi-sandbox.toml        #   if present (config is optional, so this may not exist)
├── packs/                       # <ws>-<env>-<plat>.tar  (pixi-pack output, or environment.sh)
├── vendor/                      # cargo vendor tarball, or a pointer to the vendor branch
├── node/                        # node_modules.<plat>.tar.gz
├── bin/
│   ├── pixi-sandbox-<triple>    # (+ .sha256 siblings)   ← the tool itself, self-hosted (§10.5)
│   ├── pixi-<triple>            # ← without this there is no `pixi run`, so no *tasks*, on a fresh box
│   ├── pixi-pack-<triple>       # optional, for re-packing on the target
│   ├── pixi-unpack-<triple>
│   └── <your project's binaries>
├── cache/pkgs/                  # `--use-cache` or rattler cache subset — reconstruction rungs R1/R2
└── channel/                     # unpacked pixi-pack = a LOCAL conda channel, repodata.json included
```

`workspace/` is not decoration: pixi decides "is this a workspace" from the manifest's presence, and
every environment verb needs a **lockfile** to be reproducible offline. A kit with packs but no
`pixi.lock` can put files on disk and cannot run a task — which is the difference between an archive and
an environment.

`channel/` being a real local channel is the strongest offline install path in this whole design, and it
needs no invention: **a `pixi-pack` tarball already *is* one.** Its documented layout is
`pixi-pack.json` + `environment.yml` + `channel/<subdir>/*.conda` **with `repodata.json` per subdir** ✅
[pixi-pack README](https://github.com/Quantco/pixi-pack), and pixi-pack's own words for the no-`pixi-unpack`
case are *"you have a local channel named `pixi-unpack` on your system where all necessary packages are
available"* ✅. Three consequences:

* `kit build` can copy the pack's `channel/` **verbatim** instead of synthesising an index — and the
  `environment.yml` it ships is the input for rung 4 ([§9.4](#94-reconstruction-making-pixi-run-work-offline)).
  `pixi-pack` notes the yml + repodata exist *only* for that fallback (`pixi-unpack` ignores them ✅), so we
  treat them as the compatibility surface, not the primary path.
* pixi accepts **absolute `file://` channels** ✅ (`channels = ["file:///abs/path"]` ✅,
  `pixi project channel add file://…` ✅) → the kit's channel dir is *consumable by pixi*, not only by
  conda/mamba. That is rung 2.
* Producing it is also documented: `[index-config]` in pixi's config applies to the targets **pixi indexes
  itself — "S3 URLs and local filesystem channels"** ✅, i.e. `pixi publish <dir>` writes `repodata.json`
  (+ optional `write-zst`, `write-shards`, and an `info.base_url` via `base-url` ✅, per-channel keyed by
  channel URL or absolute path, longest prefix wins ✅). So `dist push` can publish our own `.conda`
  artifacts into a git-resident local channel that pixi indexed, rather than hand-rolling an index — and
  `pixi add /abs/path/pixi-sandbox-0.1.0-<plat>.conda` ✅ (name taken from the filename) is the
  zero-index-needed fallback. Net effect: the *tool itself* is delivered as a conda package inside a kit
  produced by the tool.

> [!IMPORTANT]
> **A toolchain can travel inside a pack, which changes what "the target compiles" costs.** conda-forge ships
> Rust as `rust` 1.98.1 over nine subdirs ✅, and its own recipe tests assert `rustc --help`, `rustdoc --help`,
> **`cargo --help`** ✅ — `cargo` is inside the `rust` package, and conda-forge even installs crates with
> `cargo --config registries.crates-io.protocol="sparse" install xsv` ✅ (the same per-invocation `--config`
> mechanism §8.2 relies on). So a pack built from an env that has `rust` carries a working compiler as *ordinary
> conda packages* — no crates.io, no rustup, no `apt` ⇒ `[kit] targets-compile = true` is realistic, not
> aspirational. Three consequences, all from the feedstock ✅: `rust` `run:`-depends on `gcc_impl_<target>`,
> `sysroot_<target>`, `zlib` because *"rustc needs a toolchain to link executables"* ⇒ the linker is already in
> the lockfile, and `plan` must show its share of the bytes instead of reporting a tidy 40 MB kit; cross-target
> std is its own package (`rust-std-wasm32-unknown-unknown`, `rust-std-x86_64-pc-windows-gnu`, `rust-src` …)
> marked **`noarch: generic`** ⇒ one artifact for every platform, each `min_pin`/`max_pin = x.x.x` against `rust`
> ⇒ a frozen kit cannot mix toolchain versions; and `rust` sets `binary_relocation: false` — *"the distributed
> binaries are already relocatable"* ✅ — which is why **R5** (`tar -xf`) deserves a probe for toolchain envs 🚧,
> while `sysroot_*`/`gcc_impl_*` are prefix-relative ordinary packages ⚠️, so the expected failure is at *link*
> time, not compile time. And `rustc` calls the linker **`cc`**, which `gcc_linux-64` provides as an
> activation-script shim ⇒ reliable under `pixi run`, unproven under a bare `cargo build` in a `tar -x` prefix ⚠️.
> **None of the above is measured here** — this sandbox installs neither `rust` nor `gcc`; the claim becomes
> supported only when `kit verify` grows a `compile = true` case (`cargo build --offline` inside a reconstructed
> kit) on a runner that can reach conda-forge. Until then D16 (ship the binary; vendor only what `Cargo.lock`
> names) is the supported path. Evidence and reasoning:
> [Dogfooding on a Box Like This One §7.5](/workflows/dogfooding.md#75-what-this-host-cannot-do-directly--and-what-that-does-not-imply),
> [The Rust Toolchain Is a Conda Package §19](/research/conda-forge-rust.md#19-the-rust-toolchain-is-a-conda-package--a-correction-with-consequences).

### 9.4 Reconstruction: making `pixi run` work offline

Unpacking a prefix is not the goal. The goal is: **on the target, `pixi run <task>`, `pixi add`-style
workspace management, `pixi install` for another environment, and `pixi shell` all behave as they do in
development.** That requires `.pixi/envs/<name>` (the layout pixi itself uses ✅), the manifest, the
lockfile, a `pixi` binary, and — for anything beyond the already-packed env — an install source.

Five rungs, most capable first, and — after re-reading pixi's own configuration docs — the top two are
now **documented** rather than folkloric. `reconstruct --mode auto` walks them and picks the first that
*proves* it works by running a self-test task:

| # | Rung | Mechanism | What survives on the target | Basis |
|---|---|---|---|---|
| **1 · cache** | copy `cache/pkgs/` into `PIXI_CACHE_CONDA_PACKAGES_DIR` (the `[cache] conda-packages` kind ✅ [config](https://pixi.prefix.dev/latest/reference/pixi_configuration/)), then `pixi install --frozen` | full workspace semantics: every declared env installable from lock, tasks, `pixi shell`, `pixi add` of anything already cached | ✅ `--frozen`, cache kind/env-var hatch, "a conda package counts as available when it is already in the package cache" ✅; ⚠️ the *composition* is what [M4.5](/spec/roadmap.md#15-roadmap-and-acceptance-gates) measures |
| **2 · local channel** | point the workspace at the pack's own channel dir: `pixi project channel add file://$KIT/channel --no-install` ✅, then `pixi install --frozen` | **same as R1, and additionally `pixi add`/`pixi lock`-style editing offline** — "served from a local (`file://`) channel … needs no download either way" ✅ | ✅ `file://` channels are documented in both forms (`channels = ["file:///abs/path"]` ✅ and the `channel add` CLI ✅); ✅ the pack *is* a channel: `channel/<subdir>/*.conda` + `repodata.json` + `environment.yml` ([pixi-pack README](https://github.com/Quantco/pixi-pack)) — ⚠️ that pixi consumes *that specific* repodata for a frozen solve is the probe |
| **3 · installer unpack** | `pixi-unpack -o .pixi -e envs/<env> <pack>.tar` (real flags, read from source ✅) — it runs rattler's `Prefix::install` and writes `conda-meta/history` ✅ | the packed envs work as *installer-made* prefixes (prefix relocation included); `pixi run` for those envs; no re-solve of other envs | ✅ `src/unpack.rs` (`create_prefix` → `Prefix::install`, history file) — and it mirrors pixi's own `conda_prefix.rs` by design ✅ (cited in its source comment) |
| **4 · foreign installer** | `micromamba create -p .pixi/envs/<E> -f environment.yml` / `conda env create -p … -f environment.yml` on the pack's shipped `environment.yml` ✅; or `pixi project export conda-environment` ✅ for envs you never packed | a working prefix with **pixi not required at all**; ⚠️ conda/mamba install `pip` as a python side-effect ✅ → either `pixi add pip` upstream or `add_pip_as_python_dependency: False` | ✅ documented verbatim by pixi-pack ("Unpacking without `pixi-unpack`") |
| **5 · floor** | `tar -xf <pack>.tar` and use the tree | files at correct paths; **no `conda-meta`, no prefix rewriting** → fine for relocatable/self-contained binaries, wrong for python entry points ⚠️ | ⚠️ inference from what the other rungs do that `tar` doesn't; labelled *rescue*, never advertised as install |

**`[mirrors]` is not a rung.** Its documented contract is *"we expect that mirrors are exact copies of the
original channel"* ✅ — i.e. a *remote* substitute (http(s), `oci://ghcr.io/channel-mirrors/conda-forge` ✅,
`s3://` ✅), where repodata (and therefore every package's sha256) is fetched from the first mirror in the
list ✅ — a trust boundary worth stating in the kit README. `mirror channels` therefore writes `[mirrors]`
only when an internal Artifactory/OCI mirror is configured, and **never** promises a local directory as a
mirror (undocumented ⚠️); local serving is rung 2's job. Mirror keys are prefix-matched longest-first ✅, so
an org-wide `"https://conda.anaconda.org" → ["https://mirror.acme/"]` is expressible, and a self-pointing
longer key is the documented opt-out for channels the mirror lacks ✅.

The matrix the requirement is really about — *which pixi verbs survive, by rung*:

| pixi verb | 1 cache | 2 file:// channel | 3 unpack | 4 micromamba | Why |
|---|---|---|---|---|---|
| `pixi run <task>` (packed env) | ✅ | ✅ | ✅ | ⚠️ env isn't pixi-managed | 1–3 leave `.pixi/envs/<env>` + lockfile; 4 leaves a prefix |
| `pixi shell`, `activation.env`/`activation.scripts` | ✅ | ✅ | ✅ | ⚠️ source the generated `activate.sh` instead | activation is computed from the manifest ✅ (and cacheable — see below) |
| `pixi install --frozen` for **another** declared env | ✅ | ✅ | ⛔ only if packed | ⛔ | needs packages 1–2 can resolve, 3–4 can't |
| `pixi add` an existing version, offline | ✅ | ✅ (`--offline`-legal: local channel = available ✅) | ⛔ | ⛔ | solving needs an index; only 1–2 have one |
| `pixi add` a package **not** in the kit | ⛔ network | ⛔ not in this channel | ⛔ | ⛔ | say so in the README; don't let users discover it at 2 a.m. |
| `pixi list`, `pixi project export` | ✅ | ✅ | ✅ | ⚠️ | both read the lockfile ✅, which every kit ships |
| `pixi global install …` | ⛔ | ⛔ (global = `default-channels`, conda-forge ✅) | ⛔ | ⛔ | by design; use `pixi exec --manifest-path` inside the workspace instead ✅ |

Rungs 1–2 are what make this a *workspace*; 3 is what makes it portable to a box with `tar`; 4 is what makes
it portable to a box with **no pixi at all**; 5 is a rescue. `kit build` therefore ships the union
(`cache/pkgs` when `cache-subset != "none"`, always `channel/`, always `environment.yml`), and
`reconstruct` chooses — because the alternative is asking users to know which rung their machine qualifies
for.

One `.pixi` file a kit *may* ship: pixi's experimental **activation cache**,
`.pixi/activation-env-v0/activation_<env>.json`, whose `hash` covers that environment's entry in
`pixi.lock` plus `[activation.scripts]`/`[activation.env]` ✅ — so it is invalidation-safe by construction.
`reconstruct` copies it only when the lockfile digest in `sandbox.lock.json` matches the target's, and
`pixi run --force-activate` ✅ is the documented way to ignore it. Anything else under `.pixi/**` stays
pixi's business.

**`reconstruct` deliberately does not hand-write pixi's internals.** `.pixi/**` is pixi's business; a
fabricated cache file is how you get a workspace that appears to work and then re-solves at the worst
moment. Its whole contract is: put artifacts where pixi looks, then let pixi install:

```bash
workspace/{pixi.toml|pyproject.toml,pixi.lock} -> <target>/     # verbatim; NEVER regenerated offline
channel/**                                     -> <target>/.sandbox-channel/   # the pack's own channel dir (§9.3)
cache/pkgs/**  -> "$PIXI_CACHE_CONDA_PACKAGES_DIR"              # R1 — the documented `[cache]` kind ✅, not a
                                                               # hand-waved copy of "the pixi cache"
pixi project channel add "file://<target>/.sandbox-channel" --no-install   # R2 — documented channel form ✅
pixi install --frozen                                            # R1/R2: pixi builds .pixi/envs/* itself
pixi-unpack -o <target>/.pixi -e envs/<env> <pack>.tar           # R3: rattler Prefix::install + conda-meta ✅
tar -xf <pack>.tar -C <target>/.pixi/envs/<env>                  # R5: files only — no conda-meta, no prefix
                                                               #     relocation (the pack's repodata is still
                                                               #     inside, which is what makes R2 possible)
pixi run --manifest-path <target>/pixi.toml --environment <env> <task>   # <- the acceptance test, every rung
```

`PIXI_CACHE_CONDA_PACKAGES_DIR` is used rather than `PIXI_CACHE_DIR` because it is the per-kind hatch pixi
documents for *exactly* this cache (`cache.conda-packages`, env var wins over config ✅) — so `reconstruct`
can point pixi at a shipped cache without hijacking the repodata/wheel/mapping kinds that live beside it.

and then a **verification trio** goes into `reconstruct-report.json`: `pixi run` of a trivial task, `pixi
list` resolving against the copied lock, and `pixi shell -e <env>` starting — the three behaviours users
actually touch. `PIXI_ENVIRONMENT_NAME`/`PIXI_ENVIRONMENT_PLATFORMS` ✅ are recorded in the report rather
than assumed, because tasks that branch on them behave differently under R3/R4.

The honest summary to print into `reconstruct-report.json`:

> **R1/R2 give you a workspace. R3 gives you the packed prefixes. R4 gives you a prefix without pixi. R5
> gives you files.** Only R1 and R2 let you run *tasks that were never packed* or `pixi add` offline — R2
> because a `file://` channel is a documented solve source ✅, not because we invented one. The tool never
> says "reconstruction complete" without naming the rung it used and what that rung cannot do.
> One probe still changes a default: whether pixi will `--frozen`-install **from a pack-produced channel
> dir** (pieces ✅, composition ⚠️) — see [What Is Still Unproven §6](/workflows/unproven.md#6-what-is-still-unproven) and the
> executable version of it in
> [Manifest discovery, platform validation, offline reconstruction §15.3](/research/rev2-discovery.md#153-the-offlinecache-facts-that-make-reconstruction-defensible).

Operational view of all of this — what a human types on each side of the airlock, and what breaks:
[One Lockfile, One Digest](/workflows/lockfile-digest-map.md).

---
