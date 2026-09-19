---
type: Source Notes
title: "cargo vendor and offline builds"
description: Vendoring mechanics, source replacement, and cargo's own rules for dependency sources - what an offline Rust build really needs.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, cargo, vendoring]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["2", "4", "17"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: prefixdev-blog-launching_pixi, resource: https://prefix.dev/blog/launching_pixi, title: prefix.dev/blog/launching_pixi }
  - { id: mediumcom-nishantspatil0408-mastering-rust-workspace, resource: https://medium.com/@nishantspatil0408/mastering-rust-workspaces-from-development-to-production-a57ca9545309, title: medium.com/@nishantspatil0408/mastering-rust-workspaces-from }
  - { id: stackoverflowcom-questions-49849878, resource: https://stackoverflow.com/questions/49849878/how-to-deal-with-multiple-nested-workspace-roots, title: stackoverflow.com/questions/49849878/how-to-deal-with-multip }
  - { id: rustprojectprimercom-organization-workspacehtml, resource: https://rustprojectprimer.com/organization/workspace.html, title: rustprojectprimer.com/organization/workspace.html }
  - { id: rust-langgithubio-rfcs-2957-cargo-features2html, resource: https://rust-lang.github.io/rfcs/2957-cargo-features2.html, title: rust-lang.github.io/rfcs/2957-cargo-features2.html }
  - { id: rust-for-c-programmerscom-ch23-23_11_cargo_workspace, resource: https://rust-for-c-programmers.com/ch23/23_11_cargo_workspaces.html, title: rust-for-c-programmers.com/ch23/23_11_cargo_workspaces.html }
  - { id: pixiprefixdev-latest-build, resource: https://pixi.prefix.dev/latest/build/workspace_dependencies/, title: pixi.prefix.dev/latest/build/workspace_dependencies/ }
  - { id: githubcom-quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack }
  - { id: githubcom-quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack }
---

# cargo vendor and offline builds

## 2. cargo

pixi is repeatedly described as "a `Cargo.toml`-like manifest, but for any language"
[9](https://prefix.dev/blog/launching_pixi), so cargo is the conceptual template. Key structural
facts that transfer directly to a Rust CLI project:

**Workspaces.** Root manifest declares `[workspace] members = ["crates/a","crates/b"]` (+ `resolver`),
optionally alongside a root `[package]`; "virtual" workspaces have no root package
[2](https://medium.com/@nishantspatil0408/mastering-rust-workspaces-from-development-to-production-a57ca9545309).
A crate **cannot** be both a workspace root and a member of another workspace (no nesting)
[5](https://stackoverflow.com/questions/49849878/how-to-deal-with-multiple-nested-workspace-roots).
`cargo new` inside a workspace auto-adds the member.

**Dependency + metadata inheritance.** `[workspace.dependencies]` defines a version once; members say
`anyhow = { workspace = true }` (and may layer extra features: `serde = { workspace = true, features = ["json"] }`);
`[workspace.package]` shares `license`/`authors`/`edition` via `*.workspace = true`
[3](https://rustprojectprimer.com/organization/workspace.html). Feature unification across members uses
the resolver v1/v2/v3 union semantics; `resolver = "2"` (opt-in) / `"3"` (default for **edition 2024**)
is what decouples build-/dev-dependencies from normal deps
[1](https://rust-lang.github.io/rfcs/2957-cargo-features2.html),
[4](https://rust-for-c-programmers.com/ch23/23_11_cargo_workspaces.html).

> [!TIP]
> **`[workspace.dependencies]` and pixi's `[workspace.dependencies]` are now the same idea, by design.**
> pixi adopted cargo's inheritance: entries declared once at workspace root, consumed per-table with
> `{ workspace = true }` (dotted shorthand `name.workspace = true` works too), with layering such as
> `numpy = { workspace = true, build = "py311*" }`, and path specs re-anchored relative to the root
> manifest (`shared-lib` → `../shared-lib`) [6](https://pixi.prefix.dev/latest/build/workspace_dependencies/).
> If you design a hybrid `pixi.toml` + `Cargo.toml` repo, mirror the same member list and pin the same
> versions in both, or they will drift.

**Relevant commands for the workflow at hand:** `cargo new`, `cargo add`, `cargo build --release`,
`cargo test`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo install --locked --path .`,
`cargo tree`, `cargo vendor`, `cargo metadata --locked --offline`. Note the *upstream repo itself*
documents the source-build escape hatch for a CLI: `cargo install --locked --git
https://github.com/quantco/pixi-pack.git` [1](https://github.com/Quantco/pixi-pack) — git-installs need
only github.com (reachable here) but **still need crates.io to fetch the transitive tree** (blocked here).

---

---

## 4. `cargo vendor` and offline Rust builds

`cargo vendor` "downloads and copies all crates.io **and git** dependencies into a local directory
(default `vendor/`)" and prints the config needed to redirect resolution there; vendored sources are
**read-only** (patch with `[patch]`, don't edit files in `vendor/`); "resolution may differ from online
mode"; run `cargo fetch` first for completeness [3](https://linuxcommandlibrary.com/man/cargo-vendor).

The canonical wiring — vendor, then replace the whole source:

```bash
cargo vendor > vendor-config.toml          # prints the [source] block
cat >> .cargo/config.toml <<'TOML'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
TOML
cargo build --offline --locked            # prove it needs no network
```

Conceptually: **`[source]` replaces an entire registry; `[patch]` swaps individual crates — don't mix
them up** [5](https://www.rustfaq.org/en/how-to-vendor-dependencies-in-rust-with-cargo/). Failure mode in
CI is usually "failed to load source for dependency `X` … replacement is not configured / unable to read
`…/vendor`" because the vendor tree drifted from `Cargo.lock`; the fix is re-vendor + paste the printed
`[source]` block + `cargo build --offline --locked`, and **re-vendor whenever the lockfile changes**
[2](https://latchkey.dev/learn/rust/cargo-failed-to-load-source-replacement-in-ci).

Caveats that actually bite:

* `build.rs` scripts are **not** vendored — if a build script downloads a header or clones something,
  the offline build still fails [5](https://www.rustfaq.org/en/how-to-vendor-dependencies-in-rust-with-cargo/).
* With a vendored replacement active, `cargo add` fuzzy-matches **inside `vendor/`** and will happily add
  the wrong crate (workaround: temporarily remove `.cargo/config.toml`)
  [6](https://github.com/rust-lang/cargo/issues/10729).
* `--respect-source-config` (used by Buildroot to vendor *from* an offline mirror instead of crates.io), and
  `[source.my-mirror] registry = …` + `replace-with` for a partial mirror
  [4](https://ratatoskr.run/buildroot/2026-03-11786023/t).
* Docker pattern: copy only `Cargo.toml`/`Cargo.lock`, `cargo vendor`, build a dummy `main.rs` to warm
  the dependency layer, then copy real sources — layer-cached Rust builds
  [8](https://benjamincongdon.me/blog/2019-12-04/Fast-Rust-Docker-Builds-with-cargo-vendor/).
* Packaging-grade guidance for the same problem: `cargo fetch --locked --target …` in `prepare()`, then
  `cargo build --frozen --release --all-features` [10](https://www.reddit.com/r/rust/comments/137hmah/rust_offline/).

> [!NOTE]
> **pixi's analogue of `cargo vendor` is the pack + mirrors pair.** `cargo vendor` makes *source* hermetic;
> `pixi-pack` makes a *binary environment* hermetic; `[mirrors]`/OCI (`ghcr.io`/`s3://`) make *fetching*
> redirectable; `--offline`/`PIXI_OFFLINE` plus `--frozen` makes a run refuse the network entirely
> [10](https://pixi.prefix.dev/latest/reference/pixi_configuration/). For a Rust CLI distributed to
> locked-down users, the belt-and-braces recipe is: `Cargo.lock` committed + `vendor/` committed (or
> attached to the release) **+** the conda environment shipped as an `environment.tar`.

## 17. Cargo's own rules for dependency sources (rev. 3 — the "how are my crates handled?" research)

Question that drove this: *how do the dependency crates of a Rust crate get handled in a pixi-first,
git-only kit?* Answers below are quoted from the Cargo book ✅, because this is the one place the design had
been hand-waving.

### 17.1 `cargo vendor` — what it does and does not scope

✅ *"`cargo vendor` will vendor **all crates.io and git dependencies** for a project into the specified
directory at `<path>`"* — so git deps are covered by the same command, and there is **no** `--target`
flag: a vendored tree carries every platform's crates, plus dev/build deps.

| Flag | Documented behaviour ✅ | Why the kit cares |
|---|---|---|
| *(stdout)* | *"the configuration necessary to use the vendored sources would be printed to stdout after `cargo vendor` completes"* | the tool can **capture** it and replay it as `--config` flags instead of editing a repo |
| `-s, --sync <manifest>` | *"an extra `Cargo.toml` manifest to workspaces which should also be vendored and synced to the output"* (repeatable) | multi-crate workspaces (`pixi-sandbox` bin + `sandbox-core` lib) need one shared `vendor/` |
| `--versioned-dirs` | *"all directories in the 'vendor' directory to be versioned … can help with the performance of re-vendoring when only a subset of the packages have changed"* | makes `--sync` cheap and diffs readable (`serde-1.0.2` never overwritten) |
| `--no-delete` | *"don't delete the 'vendor' directory … keep all existing contents"* | needed when a vendor dir is shared/append-only across branches |
| `--respect-source-config` | *"instead of **ignoring** `[source]` configuration by default … read it and use it when downloading"* | ⚠️ default is *ignore*: vendoring through an internal mirror needs this flag explicitly |
| `--locked` | errors if the lockfile is **missing** or cargo *"attempted to change the lock file due to a different dependency resolution"* | the CI gate; same semantics as `pixi install --locked` |
| `--offline` / `--frozen` | `--frozen` ≡ `--locked` + `--offline`; offline *"may result in different dependency resolution"* | never mix vendoring with resolution drift |
| `-C <path>` | changes cwd for config discovery — **nightly-only** (`-Z unstable-options`) ✅ | don't build the tool's path handling on it |

Also ✅: *"Cargo treats vendored sources as read-only as it does to registry and git sources. If you intend
to modify a crate from a remote source, use `[patch]` or a `path` dependency"* — which is why editing
`vendor/` by hand is a trap the tool refuses instead of supporting.

### 17.2 Source replacement: the rules that decide the carrier format

From [Source Replacement](https://doc.rust-lang.org/cargo/reference/source-replacement.html) ✅:

* Four source kinds: `registry = "…"` (git **or** `sparse+https://…` protocol), `local-registry = "…"`,
  `directory = "…"`, `git = "…"` (+ optional `branch`/`tag`/`rev`).
* **Vendoring and mirroring are the same mechanism** — *"custom sources … represent crates on the local
  filesystem. These sources are **subsets** of the source that they're replacing and can be checked into
  packages"*. ⇒ A *subset* vendor tree is explicitly legal in principle (see the 🚧 probe in §17.4).
* **Cargo's core assumption**: *"the source code is exactly the same from both sources … a replacement source
  is **not allowed to have crates which are not present in the original source**"* ⇒ vendoring is *not* a
  patching or private-registry mechanism ✅ (use `[patch]` / `[registries]`).
* `local-registry` = *"a number of `*.crate` files … as well as an `index` directory with the same format as
  the crates.io-index project (populated with just entries for the crates that are present)"*, *"typically
  sync'd with a `Cargo.lock`"*, managed by the third-party `cargo-local-registry` subcommand ✅.
* `directory` = *"the unpacked version of `*.crate` files … suitable in some situations to check everything
  into source control"*, **managed primarily by `cargo vendor`** ✅.
* Each crate in a directory source has **`.cargo-checksum.json`**, which ✅ *"protect[s] against accidental
  modifications. **It is not a security mechanism and does not protect against malicious changes.**"*
* **"Git sources are not related to the git registries, and can't be used to replace registry sources"** ✅
  ⇒ a crates.io mirror inside a git repo is *not* a valid carrier; our `pixi-sandbox-vendor` **branch**
  therefore ships *files that get checked out into a `directory`/`local-registry` source*, never a
  `[source.x] git = …` entry.
* With a replacement active, commands that *"need to contact the registry directly"* require `--registry` ✅
  (relevant to `cargo publish`, and to any `cargo`-adjacent tooling in CI).

### 17.3 `cargo fetch` — the only target-scoped offline path ✅

* *"If a `Cargo.lock` file is available, this command will ensure that all of the git dependencies and/or
  registry dependencies are downloaded and locally available. **Subsequent Cargo commands will be able to run
  offline after a `cargo fetch` unless the lock file changes.**"*
* *"**If `--target` is not specified, then all target dependencies are fetched.**"* with `--target`
  repeatable, accepting any `rustc --print target-list` value, **`"host-tuple"`**, or a custom target spec
  JSON path ✅. (`host-tuple` is documented for exactly our case: cross-compiling some crates without
  dragging the host target in.)
* `--locked` / `--offline` / `--frozen` behave as in §17.1; exit codes are documented as **`0` ok / `101`
  failure** ✅.
* Cargo itself points at the third-party `cargo-prefetch` plugin for *"download popular crates … if you plan
  to use Cargo without a network with the `--offline` flag"* ✅ — evidence that "pre-populate `$CARGO_HOME`"
  is the intended offline workflow, not a hack.

⇒ **The carrier table**: `directory` (from `cargo vendor`) and `local-registry` (from
`cargo-local-registry`) carry *all targets*; **copying `$CARGO_HOME` after `cargo fetch --target …` is the
only cargo-documented way to make the payload target-scoped**, at the cost of depending on an
implementation-detail layout ⚠️.

### 17.4 What changed in the design because of this

| Finding | Design change |
|---|---|
| `cargo vendor` has no `--target`, and vendor trees include dev/build deps for all platforms | `[vendor] format` gained `"local-registry"` and `"cargo-home"`; `kit build` reports the crate count (819-scale matters) and prefers `cargo-home` when `[platforms]` is narrow |
| `cargo vendor` prints its config to stdout | `[vendor] wiring = "cli-flags"`: `reconstruct` replays it as `cargo --config KEY=VALUE` ✅ and **does not write into a foreign repo** |
| `.cargo-checksum.json` is explicitly *not* a security mechanism | `sandbox.lock.json` must keep its own sha256 over the vendor payload; `kit verify` stays mandatory, not advisory |
| Subset replacement is legal ("subsets of the source that they're replacing") but resolution consults the source | 🚧 **probe**: prune a `vendor/` to the host triple and run `cargo metadata --locked --offline`. If it resolves, `prune-unverified = true` becomes a ~3–4× kit-size win; if it fails with *"no matching package named …"*, the docs' "subset" language is about *content*, not *resolution inputs*, and we document that trap |
| Git sources can't replace registry sources | §10.1's vendor branch is described as *"files transported by git"*, and the config schema refuses `[source.*] git = …` for mirrors |
| `--respect-source-config` is **off by default** (vendor ignores `[source]`) | vendoring behind an internal mirror requires the explicit flag ⇒ `doctor` checks it and prints it, rather than silently re-downloading from crates.io ✅ |

Method note: §16's `CARGO_NET_OFFLINE` claim ("documented by cargo, ⚠️ not re-verified") is now ✅ —
`--offline` *"may also be specified with the `net.offline` config value"*, and the env-var form is
`CARGO_NET_OFFLINE`.

---
