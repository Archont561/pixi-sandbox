---
type: Source Notes
title: "pixi-pack: format, flags, limits"
description: pixi-pack 0.7.11 in detail: pack layout, pixi-unpack, inject, and the boundaries this design must respect.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, pixi-pack]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T00:55:00Z }
verified:
  - { by: process:sandbox-measurement, at: 2026-09-19T00:55:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["3", "16"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: githubcom-quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack }
  - { id: pixiprefixdev-latest-deployment, resource: https://pixi.prefix.dev/latest/deployment/pixi_pack/, title: pixi.prefix.dev/latest/deployment/pixi_pack/ }
  - { id: techquantcocom-blog-pixi-production, resource: https://tech.quantco.com/blog/pixi-production/, title: tech.quantco.com/blog/pixi-production/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
---

# pixi-pack: format, flags, limits

## 3. pixi-pack

**Purpose.** Starting from a **`pixi.lock`**, `pixi-pack` downloads the exact conda packages and writes
them into a single `environment.tar`; `pixi-unpack` recreates the environment on the target machine and
emits an `activate.sh`/`activate.bat` so **no conda, mamba, or pixi is needed on the target**
[1](https://github.com/Quantco/pixi-pack), [2](https://pixi.prefix.dev/latest/deployment/pixi_pack/).
Unlike `conda-pack`, it **does not require the source environment to exist** — it packs from the
lockfile, which is precisely the reproducibility problem `conda-pack` has
[1](https://github.com/Quantco/pixi-pack).

**Cross-platform by construction:** since it just downloads `.conda`/`.tar.bz2` artifacts, you can
produce a `win-64` pack from a `linux-64` host
[4](https://tech.quantco.com/blog/pixi-production/) — but "you can only `pixi-unpack` a pack on a system
with the same platform as the pack was created for"
[3](https://github.com/Quantco/pixi-pack).

**Archive layout** ✅ verified locally (mirrors the docs):

```
environment.tar
├── pixi-pack.json        # metadata (which env/platform/how it was packed)
├── environment.yml       # only for the conda/micromamba fallback path
└── channel/
    ├── noarch/{tzdata-…conda, repodata.json}
    └── linux-64/{ca-certificates-…conda, …, repodata.json}
---
```

Because the payload *is* a valid **local channel**, you can unpack without pixi-unpack at all:
`tar -xf environment.tar && micromamba create -p ./env --file environment.yml`
[2](https://pixi.prefix.dev/latest/deployment/pixi_pack/).

**CLI (current form, not the old subcommands).** ✅ verified locally from `src/bin/pixi-pack.rs`
(v0.7.11): flat flags, positional manifest —

```bash
pixi-pack --environment prod --platform linux-64 pixi.toml
```

| Flag | Meaning (defaults in brackets) |
|---|---|
| `-e, --environment` | which pixi environment to pack [`default`] |
| `-p, --platform` | conda subdir **or** named platform from lockfile v7 [`Platform::current()`] |
| `<manifest_path>` | path to `pixi.toml`/`pyproject.toml`/dir [`cwd`] |
| `-o, --output-file` | pack destination |
| `--use-cache <DIR>` | cache directory for downloaded packages |
| `--auth-file` | authentication file for fetching packages |
| `-i, --inject <PATH>…` | inject extra conda packages into the final prefix (`num_args(0..)`) |
| `--ignore-pypi-non-wheel` | allow packing when PyPI **sdist** deps are present (sdists unsupported) |
| `--create-executable` | emit a **self-extracting** `environment.sh` / `environment.ps1` |
| `--directory-only` | produce a folder instead of a tar (`conflicts_with = "create-executable"`) |
| `--pixi-unpack-source <URL\|PATH>` | custom unpacker binary for the self-extractor [`requires = create-executable`] |

Two behavioural details worth copying into any similar Rust CLI: `--platform` accepts a *name* and
falls back with a helpful error if a bare conda subdir is ambiguous, and the self-extracting binary is
"just" a shell script with the `pixi-unpack` binary + the payload embedded, whose default download
location is `github.com/Quantco/pixi-pack/releases/latest` — hence `--pixi-unpack-source` exists for
mirrors/air-gapped use [3](https://github.com/Quantco/pixi-pack).

**Engineering profile of the tool** ✅ verified locally (a good template for the CLI we're asked to design):
single crate, `version = "0.7.11"`, `edition = "2024"`, release profile
`{ codegen-units = 1, lto = true, strip = true, opt-level = "z" }` (size-optimised, statically-musl'd
binaries), deps = `clap 4.6.1` (`derive`, `string`) + `clap_complete 4.6.5` + `clap-verbosity-flag 3.0.4`
(with `tracing`) + `anyhow 1.0.102` for errors, `rattler*` for conda logic, `reqwest`, `tokio`
(`rt-multi-thread`), `indicatif` progress, `serde`/`serde_yaml`, `insta` snapshot tests
(`INSTA_UPDATE=always cargo test -p pixi-pack -- test_reproducible_shasum`), `native-tls` default with a
`rustls` feature toggle. Its own `pixi.toml` pins `rust = "==1.95.0"`, `openssl = "3.*"`,
`pixi = ">=0.72.0"`, plus lint features (`pre-commit`, `prettier`, `taplo`, `typos`, `shellcheck`,
`zizmor`).

**Container/deployment angle.** `FROM ghcr.io/prefix-dev/pixi:0.23.0` + `pixi install --locked` and
`rm -rf ~/.cache/rattler` cut a demo image from 691 MB → 402 MB
[4](https://tech.quantco.com/blog/pixi-production/) — i.e. the **rattler cache is a huge share of image
size**, and `pixi-pack` is the alternative delivery vehicle when you don't want an image at all.

## 16. Rev. 2 addendum: local channels, cache kinds, and what a pack actually contains

Prompted by two questions — *what does a user type, on each side of the airlock?* and *do I need separate
`cargo vendor` handling, or can `pixi-pack` carry it?* Both turned on facts I had guessed at rather than
read, so this section is mostly quotations with their sources.

### 16.1 `file://` channels and offline mode — the fact that changed the design

From [pixi configuration → offline](https://pixi.prefix.dev/latest/reference/pixi_configuration/), on
current docs ✅:

> A conda package counts as available locally when it is already in the package cache, **or when it is
> served from a local (file://) channel**, which needs no download either way. Anything else is excluded
> from the solve, and if an exclusion is what makes a solve impossible the error names the packages that
> were ruled out.

and, on channel syntax, from the manifest/configuration references ✅:

```toml
channels = ["conda-forge", "file:///home/user/staged-recipes/build_artifacts"]
```

```console
$ pixi project channel add file:///home/user/local_channel      # also: --no-install, --feature <F>, --prepend
```

Consequences we took:

* **A kit can ship a *solveable* index, not just files.** `spec/docs-site.md §9.4`'s rung 2 is this, so the
  reconstruction guarantee stopped being a bet.
* Offline solving is honest about its limits: *"Git dependencies are only fetched from the local checkout
  cache; local `file://` repositories keep working"* ✅, but **git LFS objects are skipped** ⚠️ (a pack with
  LFS content needs the objects copied explicitly).
* `pixi add`/`pixi upgrade` offline *"record version bounds derived from the versions they resolved, so
  offline those bounds describe what was available locally rather than what the channels offer"* ✅ →
  a kit sets `pinning-strategy = "exact-version"` ✅ so what you add offline is pinned to what is actually
  in the channel, and a later online run cannot silently widen it.
* Documented exemptions from `--offline`: **build backends for source deps are not restricted** ("offline
  mode is currently not enforced for them" ✅) and **PyPI resolution is delegated to `uv`**, which gets
  offline connectivity *"but cannot [be] restricte[d] to cached distributions, so a solve that succeeds may
  still require network access for PyPI"* ✅ → hence the airlock rule "packed envs prefer conda packages;
  PyPI arrives as wheels inside the pack".

### 16.2 Config discovery — where a kit can inject settings without touching `$HOME`

Documented priority ✅ (Linux/macOS): `11` CLI args · `10` `your_project/.pixi/config.toml` · `9`
`$PIXI_HOME/config.toml` · `8` `$HOME/.pixi/config.toml` · `7`/`6` XDG/user · `5`–`3` rattler-shared ·
`2`/`1` `/etc/pixi`, `/etc/rattler`. Highest wins; project-local is **always** merged on top. Plus ✅:
`--no-config`/`PIXI_NO_CONFIG=1` (insulate from machine-wide settings — *"useful in CI, scripts, and
tests"*) and `--config-file <PATH>`/`PIXI_CONFIG_FILE` (load only that file as the global layer).

⇒ **the kit's `reconstruct` writes `workspace/.pixi/config.toml` (priority 10) and never edits `~/.pixi`.**
Also: the rattler-shared locations accept only keys both tools understand — `default-channels`, `mirrors`,
`s3-options`, `index-config`, `concurrency` ✅ — and pixi *warns* about pixi-only keys placed there ✅, which
is why `[experimental]`/`detached-environments` belong in the pixi file, not the rattler one.

### 16.3 Cache kinds: the airlock's real shape

`[cache]` per-kind overrides ✅ with env-var hatches that take precedence over config ✅:

| Env var | TOML field | Contents |
|---|---|---|
| `PIXI_CACHE_CONDA_PACKAGES_DIR` | `cache.conda-packages` | extracted conda packages (`pkgs`) — **rung 1's target** |
| `PIXI_CACHE_REPODATA_DIR` | `cache.repodata` | repodata cache |
| `PIXI_CACHE_PYPI_WHEELS_DIR` | `cache.pypi-wheels` | uv wheel cache |
| `PIXI_CACHE_PYPI_MAPPING_DIR` | `cache.pypi-mapping` | conda↔PyPI name map |
| `PIXI_CACHE_EXEC_ENVIRONMENTS_DIR` | `cache.exec-environments` | `pixi exec` envs |
| `PIXI_CACHE_BUILD_TOOL_ENVIRONMENTS_DIR` | `cache.build-tool-environments` | build-tool envs |
| `PIXI_CACHE_DETACHED_ENVIRONMENTS_DIR` | `cache.detached-environments` | envs when `detached-environments` is on |

Root fallback order ✅: `PIXI_CACHE_<KIND>_DIR` → `[cache.<kind>]` → `PIXI_CACHE_DIR` → `RATTLER_CACHE_DIR`
→ `[cache.root]` → `$XDG_CACHE_HOME/pixi` → platform default. Path rules ✅ that will bite anyone generating
this file: **absolute paths required** (relative rejected at load, because "relative to which config?" is
ambiguous), `~` **is** expanded, **`$HOME`-style substitution is not performed** ✅. `netfs-redirect = "auto"`
moves the non-shared-friendly kinds to `$SLURM_TMPDIR`/`$PBS_JOBFS`/`$SCRATCH`/`$TMPDIR` when the root looks
like a network filesystem ✅ (`"never"`/`"always"`, or `PIXI_CACHE_NETFS_REDIRECT`,
`PIXI_DISABLE_NETFS_REDIRECT`/`PIXI_FORCE_NETFS_REDIRECT` ✅).

Two more: `detached-environments = true | "/opt/pixi/envs"` ✅ producing
`NAME_OF_PROJECT-HASH_OF_ORIGINAL_PATH/{envs,solve-group-envs}` ✅, at the documented cost of
*"a disconnect between the workspace and its environments and manual cleanup … when deleting the
workspace"* ✅ — which is exactly what you want in a sandbox whose repo dir is snapshotted. And
`run-post-link-scripts` **defaults to `false`** because pixi deems post-link scripts insecure (a planned
"sandbox mode" is the future alternative ✅): a package that needs one installs *half-complete*, looking like
a missing file rather than an error → hence a `__selftest__` task after every rung.

### 16.4 What a `pixi-pack` tarball is (and is not)

From the upstream README ✅ (**`Quantco/pixi-pack`** — not `prefix-dev`, which several of my earlier notes
said) and from `src/unpack.rs` ✅:

```
environment.tar
├── pixi-pack.json
├── environment.yml                      # for conda/micromamba consumers ONLY
└── channel/<subdir>/*.conda + repodata.json   # ← "a local channel named pixi-unpack" ✅
```

* *"The `environment.yml` and `repodata.json` files are only for this use case, `pixi-unpack` does not use
  them"* ✅ — i.e. the pack is **already a channel with an index**; that is rung 2's raw material, and it
  corrects my earlier claim that a tar-unpacked pack "has no repodata index" — see **C4** in §14.
* `pixi-unpack` is not a `tar`: `create_prefix` extracts the `.conda` files and calls rattler's
  `Prefix::install(...)`, **writes `conda-meta/history`** ✅, then `install_pypi_packages` queries the
  prefix's own Python ✅; its source even links pixi's `conda_prefix.rs` as the thing it mirrors ✅. So a
  rung-3 prefix has conda metadata and relocation applied; `tar -xf` alone does neither ⚠️.
* Activation is generated *on the target*: `-o/--output-directory` (default cwd) · `-e/--env-name` (default
  `env`) · `-s/--shell bash|zsh|xonsh|cmd|powershell|fish|nushell` ✅, writing `activate.sh` at the output
  root with absolute paths → **never ship an activation script in a kit**.
* `--use-cache <dir>` is *"the same structure as conda channels, organizing packages by platform
  subdirectories"*, recommended for *"environments with limited bandwidth"* and CI reuse ✅ → the kit's
  shipped cache can be **pixi-pack's own output** (documented artifact) rather than a copy of pixi's private
  cache (undocumented composition) — [the design spec](/spec/index.md) now defaults `cache-source = "use-cache"` for that reason.
* `--inject` accepts `.conda`/`.tar.bz2` (compatibility against the env **is** checked ✅) and `.whl`
  (*"we cannot verify that injected wheels are compatible"* ✅). **`.crate` is neither** ⇒ see §16.5.
* Supply chain: releases publish GitHub **Artifact Attestations**, verifiable with
  `gh attestation verify --repo Quantco/pixi-pack pixi-pack-<arch>` ✅ → the `mirror binary` step can record a
  verification result instead of only a digest.
* ⚠️ Compatibility-mode gotcha, verbatim: *"Both `conda` and `mamba` are always installing pip as a side
  effect when they install python"*, unlike pixi → solver errors on `micromamba create -f environment.yml`.
  Documented fixes: `pixi add pip` upstream, or `conda config --set add_pip_as_python_dependency false` ✅.

### 16.5 pixi ↔ cargo: what `pixi-pack` does not carry, and what `pixi build` does instead

**Three layers, only two of them conda** (this is the answer to "do I need separate vendor handling?"):

| Layer | Owner | Where it lives | In a pack? |
|---|---|---|---|
| toolchain (`rust`/`cargo`/`rustc`/`rust-std`, linkers, `openssl`, `sccache`) | pixi / conda-forge ✅ | prefix `bin`, `lib/rustlib` | ✅ yes — pixi-pack pins `rust = "==1.95.0"` in its own `pixi.toml` ✅ |
| **your crate's dependency graph** | cargo (`Cargo.lock` + `$CARGO_HOME/registry`) | *outside* the prefix | ⛔ **no** — a pack is conda `.conda` files (+ wheels) ✅ |
| your built artifact | you | `target/release`, or a `.conda` you made | ⛔ not by default — but `--inject myproj-….conda` is documented *for exactly this* ✅ |

`pixi build` closes the third row: `workspace.preview = ["pixi-build"]` ✅ +
`[package.build] backend = { name = "pixi-build-rust", version = "*" }` ✅, which ✅ (a) reads metadata from
`Cargo.toml` except `name`/`version`, which are still required (tracking issue #4317 ✅), (b) runs
`cargo install --locked --root "$PREFIX" --path . --no-track --force` ✅ — so a stale `Cargo.lock` is a hard
build error ✅, (c) wires `sccache` as `RUSTC_WRAPPER` when present and OpenSSL paths when `openssl` is in
the env ✅, (d) accepts `extra-args`, `env` (per-target merge), `compilers` (default `["rust"]`),
`extra-input-globs`, `ignore-cargo-manifest` ✅, and (e) is limited: always release mode, no custom cargo
profiles, *"limited workspace support for multi-crate projects"* ✅.

Two operational consequences:

1. **Nothing in the backend docs mentions vendoring** — it hands cargo a `$PREFIX` and a manifest and lets
   cargo resolve. So offline builds still need cargo's own mechanism
   (`[source.crates-io] replace-with`, `CARGO_NET_OFFLINE=true` — documented by cargo; ⚠️ not re-verified
   this session), and if that mechanism is a `vendor/` **directory**, it must be added to
   `extra-input-globs`: the default inputs are `**/*.rs`, `Cargo.toml`, `Cargo.lock`, `build.rs` ✅ — `vendor/`
   is not in that list ⚠️ (inference from the documented default, worth a probe).
2. **Therefore the decision is `[kit] targets-compile`, not a preference.** A run-only airlock should ship
   the built binary (`--inject` its `.conda`, or `bin/mybin-<plat>` in the kit) and **skip `vendor/`
   entirely**; a compiling airlock must carry `vendor/` + `.cargo/config.toml` regardless of what pixi does.
   `spec/packagers.md §8.2` and `workflows/pixi-cargo-interop.md §4` carry the table version of this.

### 16.6 Net effect on the design (and one self-correction)

| Claim as written before | Status now |
|---|---|
| "R2 = write `[mirrors]` pointing at a local dir (⚠️ unverified)" | **demoted.** Mirrors are documented as *exact copies of the original channel* ✅; local serving is a `file://` **channel** ✅, so the ladder gained a documented rung and lost a speculative one |
| "`tar -xf` + `channel/` is the guarantee; no repodata index exists" | **wrong on both counts.** The pack *has* `repodata.json` per subdir ✅, and the *guarantee* is `pixi-unpack`'s installer (or micromamba on the shipped `environment.yml`) ✅ |
| "copied `$PIXI_CACHE_DIR/pkgs` (⚠️ undocumented)" | improved twice: `PIXI_CACHE_CONDA_PACKAGES_DIR` ✅ is the documented knob for *where* the cache lives, and `pixi-pack --use-cache` ✅ is a documented *source* of cache-shaped bytes. The composition is still a probe, so rung 1 stays "attempted", not "promised" |
| "`conda-meta/history` needed for `--fallback conda` 🚧" | ✅ verified from source: `pixi-unpack` writes it ✅ |
| "pixi-pack is a prefix-dev project" | ✅ `Quantco/pixi-pack`; attestations verifiable ✅ |

**The single probe that still moves a default:** `pixi install --frozen` against a `file://` channel built
from a real `pixi-pack` tarball, in a container with no network. Green ⇒ rung 2 is the advertised guarantee
and `cache/pkgs/` becomes an optional size optimisation. Red ⇒ rung 3 is the guarantee, offline `pixi add`
is documented as unsupported, and `pixi add <abs path>.conda` ✅ is the only manifest-editing escape.

---

### 16.7 The release assets, measured — why the action mirrors a *pair*

`GET https://api.github.com/repos/Quantco/pixi-pack/releases/latest` from this sandbox, 2026-09-19 ✅:

| Fact | Value |
|---|---|
| Latest release | `v0.7.11`, published 2026-08-31 |
| Assets | **16** — 8 × `pixi-pack-<triple>` (9.6–14.7 MB) and 8 × `pixi-unpack-<triple>` (10.1–15.7 MB) |
| Integrity | **every asset carries `digest: sha256:…`** in the API payload — e.g. `x86_64-unknown-linux-musl` pack `8191f586b734e634…`, unpack `7cf766c38436406f…` |

Three consequences for the design:

* The kit's `bin/` directory is a **pair**, not a single binary ([The Action Shape](/spec/action-shape.md)
  `ship-pixi`): `pixi-pack` is only needed by the *builder*, `pixi-unpack` by the *target* — and dropping the
  second is what pins a sealed machine to the `tar` floor instead of rung 3.
* Because `api.github.com` is reachable even from a GitHub-only sandbox ✅, a sealed target can **re-verify our
  mirrored copy against upstream's own digest** — the property `provenance = "mirror"` + `upstream-sha256` encodes
  in [Artifacts](/spec/artifacts.md), now with a concrete number to compare against.
* ⚠️ `api.anaconda.org` returned `000` from this host on the same day, so "is `pixi-pack`/`pixi-unpack` also on
  conda-forge?" is **unresolved here**; the documented route `pixi global install pixi-pack pixi-unpack` stays the
  assumption, and `binary_relocation`/prefix questions for those two packages are open ⚠️.
