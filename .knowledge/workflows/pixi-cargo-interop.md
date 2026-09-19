---
type: Reference
title: "pixi and cargo: What Travels Where"
description: The interoperability verdict - pixi-pack packs environments, not crate graphs - plus source replacement and the build.rs hole.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, cargo, pixi-pack]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["4"] }
sources:
  - { id: prefixdev-conda-forge, resource: https://prefix.dev/conda-forge, title: prefix.dev/conda-forge }
  - { id: githubcom-rust-lang-cratesio-index, resource: https://github.com/rust-lang/crates.io-index, title: rust-lang/crates.io-index }
---

# pixi and cargo: What Travels Where

## 4. pixi ↔ cargo interoperability, and what `pixi-pack` does not carry

### 4.1 The mental model: three layers, only two of them conda

| Layer | Owned by | Lives in | Inside a pixi-pack tarball? |
|---|---|---|---|
| **The toolchain** — `cargo`, `rustc`, `rust-std`, linkers, `sccache`, `openssl` | **pixi** (conda-forge `rust` ✅) | `.pixi/envs/<env>/bin`, `lib/rustlib` | ✅ **yes** — it's just conda packages from `pixi.lock`. `rust` is pinned by `pixi-pack` itself: `rust = "==1.95.0"` ✅ |
| **Your crate's dependency graph** — 400 crates from crates.io | **cargo** (`Cargo.lock` + `$CARGO_HOME/registry`) | *not* in the prefix | ⛔ **no** — never was. A pack contains *conda* packages ✅ (`channel/<subdir>/*.conda` + `environment.yml` + `pixi-pack.json` ✅) plus PyPI **wheels** ✅, not `.crate` files |
| **Your built artifact** — `mybin` | you, via cargo or `pixi build` | `target/release/` | ⛔ not automatically — **but** `--inject myproj-0.1.0-<build>.conda` ✅ is documented *for exactly this*: "useful if you build the project itself and want to include the built package in the environment but still want to use `pixi.lock` from the project" ✅ |

So: **pixi and cargo are not two package managers fighting over your project — they are two layers of one
stack.** pixi hands you a *compiler and its system ABI* with a lockfile; cargo hands you *crate sources*
with a different lockfile. Neither lockfile references the other, and no `pixi-pack` flag merges them.
Concretely, `pixi add rust` and `cargo add serde` mutate different files, and the "three lockfiles"
discipline in `research/synthesis.md §12` is what keeps them honest.

### 4.2 Where pixi *does* reach into cargo: the build backend

`pixi build` (preview: `workspace.preview = ["pixi-build"]` ✅) with

```toml
[package]
name = "mybin"
version = "0.1.0"

[package.build]
backend = { name = "pixi-build-rust", version = "*" }
channels = ["https://prefix.dev/conda-forge"]

[package.build.config]
compilers = ["rust"]                       # default ✅; add "c","cxx" for crates with C build scripts
extra-args = ["--features", "cli,server"]  # appended to the cargo invocation ✅
env = { CARGO_PROFILE_RELEASE_LTO = "true" }
---
extra-input-globs = ["vendor/**", "assets/**"]   # ← see the gotcha below
```

turns your crate into **a conda package**, which is the one move that makes the whole airlocked story
cleaner, because then your *own* artifact is a first-class citizen of `pixi.lock`-based tooling:
`--inject` it into a pack, publish it to a local channel, install with `pixi add <abs path>.conda` ✅.

Documented behaviour ✅ worth knowing before relying on it:

* It **reads `Cargo.toml`** for name/version/license/description/homepage/repository *when `pixi.toml`
  doesn't override* — but you still must set `name` and `version` (tracking issue #4317 ✅), and
  `ignore-cargo-manifest = true` exists for when the two metadata sources fight ✅.
* The build is `cargo install --locked --root "$PREFIX" --path . --no-track --force` ✅ — i.e. **`--locked`
  is not optional**, so a stale `Cargo.lock` is a build error, not a surprise. That is the same exit-3
  class as a stale `pixi.lock` (§3.1), and `plan` reports both symmetrically.
* `sccache` is wired up as `RUSTC_WRAPPER` **if present** ✅ and OpenSSL paths are set when the env has
  `openssl` ✅ — meaning a conda-provided `openssl` gets *linked*, which is precisely the ABI reason to use
  conda for Rust at all.
* Limitations, verbatim in spirit ✅: always release mode, no custom cargo profiles, **"limited workspace
  support for multi-crate projects"** → for a real workspace, build crates individually (or keep plain
  `cargo build` and don't use the backend).
* ⚠️ **`extra-input-globs` gotcha**: default inputs are `**/*.rs`, `Cargo.toml`, `Cargo.lock`, `build.rs`
  and other build-related files ✅. A `vendor/` directory is **not** in that list, so an offline
  `pixi build` of a vendored crate silently loses the sources unless you add `vendor/**` (🚧 confirm the
  exact glob semantics — `.crate`-less `vendor/` is thousands of files, so also check `ignore`-type
  options).

### 4.3 Do you need separate vendor handling? A decision, not a preference

**Yes if the target compiles. No if the target only runs.**

| Situation on the airlocked machine | Use | Why |
|---|---|---|
| Users run `mybin`; nobody edits Rust code | **no `vendor/` at all.** Build in CI (networked), ship `--inject mybin….conda` into the pack ✅ or `bin/mybin-<plat>` in the kit | the crate graph is a build-time concern; shipping 400 crates for a binary is 400× the bytes for zero capability |
| Users must rebuild with their own flags / patch a crate offline | **a crate carrier** ([§4.5](#45-how-your-dependency-crates-travel-four-carriers-one-invariant)): `vendor/` (A) or `local-registry` (B) or a fetched `$CARGO_HOME` (C), wired by [§4.6](#46-wiring-it-up-without-editing-the-users-repo) | nothing else makes crates.io resolvable: source replacement is a *config* mechanism, not a pack mechanism ✅ |
| Users build **only for the host triple** and kit size matters | **C** — `cargo fetch --locked --target <triple>` then ship `$CARGO_HOME` ✅ | `cargo vendor` has no `--target` ✅, so A/B ship every platform's crates; C is the only one cargo documents as target-scoped |
| Users must build *and* you cannot commit `vendor/` (size, review noise) | same, with `vendor.strategy = "branch"` → the `pixi-sandbox-vendor` orphan branch, fetched by digest | a vendored tree is read-only ✅ and reviewing it is pointless; git objects are still addressable |
| You use `pixi build` for your own crate | `vendor/` **in the build inputs** (`extra-input-globs`) and `env = { CARGO_NET_OFFLINE = "true" }` ⚠️ | the backend calls cargo; cargo still resolves crates from somewhere |

And the specific thing people hope for first and get once: **`pixi-pack --inject` is not a way to smuggle
crates into an environment.** Injected inputs must be `.conda`/`.tar.bz2` (compatibility-checked against the
env ✅) or `.whl` for PyPI (**not** checked ✅, per its own warning). A crate is neither.

`pixi sandbox` therefore treats `vendor` as a *conditional* component (`Cargo.lock` present **and** the kit
is marked `--components …,vendor`, or `auto` + "target compiles" mode) rather than always-on, and
`plan` prints the reason: `"vendor": enabled (Cargo.lock detected, kit.targets_compile = true)`.

### 4.4 The one command that proves the Rust half is offline

```bash
cargo metadata --locked --offline >/dev/null && echo "vendor covers the graph ✅"
cargo build --locked --offline --release
cargo tree -d                                   # duplicate versions — not offline-related, but free to check
```

If `cargo metadata --locked --offline` succeeds, `cargo build --offline` will not reach the network for
*resolution*; if it fails, no amount of packing fixes the build. `vendor check` is exactly this, wrapped
with a remediation message (and the `build.rs`-downloads caveat ✅ `spec/packagers.md §8.2` point 4).

---

### 4.5 How your dependency crates travel: four carriers, one invariant

Your crate's deps are ~819 packages for a real CLI of this shape (measured: `pixi-pack`'s `Cargo.lock` has
**819** `[[package]]` entries, **818** from a single source —
`registry+https://github.com/rust-lang/crates.io-index` ✅, the 819th being the root). Nothing about that
graph is in `pixi.lock`, so it needs a carrier. There are four, and they differ in size and in who unpacks
them:

| Carrier | Created by | Shape on the target | Size profile | Cargo side |
|---|---|---|---|---|
| **A. directory source** (`vendor/`) ✅ | `cargo vendor --locked --versioned-dirs` ✅ (*"vendor **all crates.io and git dependencies**"* ✅) | unpacked source trees + one `.cargo-checksum.json` per crate | **all targets, all dev/build deps** — `cargo vendor` has *no* `--target` flag ✅, so you ship osx/windows crates to a linux box 🚧 pruning is not supported by resolution (see below) | `[source.<name>] directory = "vendor"` + `replace-with` ✅ |
| **B. local registry** ✅ | `cargo vendor`-like sync via **`cargo-local-registry`** (third-party subcommand ✅) | one dir of `*.crate` files **plus a `index/` in crates.io-index format** ✅ | compressed `.crate` archives ⇒ a fraction of A; still all-targets, because the lockfile is | `[source.<name>] local-registry = "registry"` ✅ |
| **C. copied `$CARGO_HOME`** | `cargo fetch --locked [--target <triple>]` on the build box ✅ — *"subsequent Cargo commands will be able to run offline after a `cargo fetch` unless the lock file changes"* ✅ | cargo's own registry cache (layout is an implementation detail ⚠️, never parse it) | **the only carrier that can be target-scoped**: `--target` is documented on `cargo fetch` ✅ ("if `--target` is not specified, then all target dependencies are fetched") ✅ | nothing to configure: `cargo build --offline --locked` reads the cache ✅ |
| **D. no carrier** | you built the artifact ✅ | a binary, or a `.conda` injected into the pack (`--inject` ✅) | zero crate bytes | **cargo never runs on the target** |

**The invariant:** *A and B must be supersets of what the build resolves, not just what it compiles.* Cargo
reads a directory/local source while **resolving**, so a vendor tree pruned to `x86_64-linux` alone is
expected to fail with *"no matching package named … found"* even though the pruned crates were the only ones
compiled 🚧 (inferred from resolution semantics — **verify**, and if pruning does work it's a large kit-size
win worth a config flag). C sidesteps this because `cargo fetch --target` decides the payload by download,
not by resolution.

### 4.6 Wiring it up without editing the user's repo

Three scopes, pick the least invasive that works:

```toml
# scope 1 — committed config in the repo (what `vendor sync` writes; simplest, visible in review)
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = "vendor"
```

```bash
# scope 2 — per-invocation, NOTHING written into the tree ✅ (what `reconstruct` uses on a foreign repo)
cargo build --release --locked --offline \
  --config 'source.crates-io.replace-with="kit-vendored"' \
  --config 'source.kit-vendored.directory="/var/tmp/kit/vendor"'

# scope 3 — cargo's own config file in a kit-private CARGO_HOME (best when tasks hard-code `cargo build`)
export CARGO_HOME="$KIT/cargo-home"      # contains config.toml (scope-1 content) + maybe a fetched cache (C)
export CARGO_NET_OFFLINE=true            # the env twin of --offline
```

Scope 2 exists precisely because *"the configuration necessary to use the vendored sources would be printed
to stdout after `cargo vendor` completes"* ✅ — the tool captures that output, and can replay it as flags
instead of editing files. Scope 3 is what makes `pixi run build` work when the task text is
`cargo build --release` and cannot be changed.

> [!WARNING]
> Three sharp edges, all documented ✅:
> 1. **A replacement source may not contain crates the original lacks** — *"a replacement source is not
>    allowed to have crates which are not present in the original source"*, so vendoring is **not** a way to
>    patch or privately-hosted-mirror a dependency; use `[patch]` or a `path` dep ✅.
> 2. **`.cargo-checksum.json` "is not a security mechanism and does not protect against malicious
>    changes"** ✅ — it only catches *accidental* edits. That is exactly why `sandbox.lock.json` carries a
>    sha256 of the whole `vendor/` payload: cargo gives you tamper-*evidence-free* integrity, we add the
>    digest.
> 3. **Git sources cannot replace registry sources** ✅ — so "mirror crates.io as a git repo" is not a
>    carrier. Our `pixi-sandbox-vendor` **branch** is a git *transport of files* that gets checked out into a
>    directory source (A); it is never configured as `[source.x] git = …`.

### 4.7 What the tool does, mechanically

```bash
pixi sandbox vendor sync           # cargo vendor --locked --versioned-dirs  (+ --sync <extra manifest> per
                                   #   workspace member: "-s" lets you sync several manifests into one dir ✅,
                                   #   --no-delete ✅ keeps unrelated content when re-vendoring incrementally)
pixi sandbox vendor pack           # carrier A → vendor.tar.zst for the dist branch, or carrier B via
                                   #   cargo-local-registry if vendor.format = "local-registry"
pixi sandbox vendor check          # cargo metadata --locked --offline   → then cargo build --offline
                                   #   maps cargo's documented exit codes: 0 ok / 101 failed ✅ — 101 with an
                                   #   empty stderr is reported as E-VENDOR-INCOMPLETE, never as a pass
pixi sandbox vendor disable        # removes replace-with (so `cargo add` can run)  ✅ see §1.2 ordering
pixi sandbox cargo fetch-cache     # carrier C: runs `cargo fetch --locked --target <t>` on the build box and
                                   #   ships $CARGO_HOME; --target list comes from [platforms] per-env  🚧
```

`vendor.strategy = "branch"` (D9) is the default for a reason that only shows up at 819 crates: a vendored
tree is ~4× the compressed size of the same graph in `*.crate` form, it is *read-only* to cargo ✅, it
cannot be pruned per target in A/B (above), and reviewing it is worthless — so the payload belongs on a
disposable orphan branch while `Cargo.lock` (the thing humans actually review) stays on `main`.
