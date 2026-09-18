---
type: Measurement
title: "The Rust Toolchain Is a Conda Package"
description: Evidence that conda-forge rust is rustc+cargo in package form, what that fixes in the design, and what it does not.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, rust, conda-forge]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified:
  - { by: human:archont561, at: 2026-09-19T21:00:00Z }
  - { by: process:sandbox-read, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["19"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: staticrust-langorg-dist-rust-version-tripletargz, resource: https://static.rust-lang.org/dist/rust-{{version}}-{{triple}}.tar.gz, title: static.rust-lang.org/dist/rust-{{version}}-{{triple}}.tar.gz }
confidence-note: derived from the measurements and upstream reads cited in [Bibliography](/research/sources.md)
---

# The Rust Toolchain Is a Conda Package

## 19. The Rust toolchain is a conda package — a correction with consequences

The previous turn concluded *"`rustc` cannot reach this box through any allowlisted route"* and generalised it
into "no compiler in an airlock". **Half of that was wrong**, and the user's correction ("`rustc` and `cargo`
are available by installing the `rust` conda-forge package") is right: the route that doesn't exist here is
*conda-forge reachability*, not a compiler. `conda-forge::rust` is an ordinary conda package, which means it can
ride inside a **pack** — and packs are exactly what the airlock receives.

Evidence, from `conda-forge/rust-feedstock` (a **split** feedstock: `rust-split`) at recipe/main ✅:

| Fact | Quoted / observed ✅ | Consequence for the kit |
|---|---|---|
| Version + reach | `1.98.1`; `platforms: linux-64, linux-aarch64, linux-ppc64le, linux-riscv64, osx-64, osx-arm64, win-64, win-arm64` (win-32 stale at 1.25.0) | broader than most packages (compare `bun`'s four ✅) — so "every platform in `pixi.lock`" is a safe default for a toolchain env |
| **`cargo` is in `rust`** | its own `test.commands`: `rustc --help`, `rustdoc --help`, **`cargo --help`** | a reconstructed env compiles *and* resolves crates, no separate package |
| Registry config via flags | `cargo --config registries.crates-io.protocol="sparse" install --force xsv` | conda-forge itself wires cargo behaviour with **`--config KEY=VALUE`** — precedent for §8.2's non-mutating wiring |
| Upstream provenance | sources are `https://static.rust-lang.org/dist/rust-{{version}}-{{triple}}.tar.gz` with `sha256` per platform | the conda package *is* the official dist tarball, re-packaged: no patching of the compiler beyond `patches:` on `install.sh` |
| The linker is part of the contract | `run:` `gcc_impl_{{ target_platform }}`, `sysroot_{{ target_platform }} >={{ c_stdlib_version }}`; host `{{ compiler('c') }}` with the comment *"rustc needs a toolchain to link executables"*; `zlib` because *"zlib is linked by /lib/libLLVM-*-rust-*.so"* | a `rust`-bearing kit carries a C toolchain too. Free (they're lockfile deps ⇒ pixi-pack includes them), but it must be *visible* in `plan`, or the byte estimate lies |
| Cross-target std = packages | outputs `rust-std-<triple>` for `aarch64-apple-ios(-sim)`, `aarch64/armv7/i686/x86_64-linux-android`, `wasm32-unknown-unknown`, `wasm32-unknown-emscripten`, `wasm32-wasip1-threads`, `thumbv7em-none-eabihf`, `thumbv8m.main-none-eabihf`, `x86_64-pc-windows-{gnu,msvc}`, `aarch64-pc-windows-msvc`, plus `rust-src`, `rust-docs` | §7.4's platform check should ask *"does `rust-std-<triple>` exist"*, not *"does `rust` exist"* — that is the actual gap between "runs" and "builds for wasm" |
| Those outputs are `noarch: generic` | `rust-std-*`, `rust-src`, `rust-docs`: `noarch: generic` | platform-independent payloads ⇒ one `.conda` serves every subdir in the pack, cheap to mirror |
| Versions cannot drift | `run_constrained: pin_subpackage("rust", min_pin="x.x.x", max_pin="x.x.x")` on every `rust-std-*`/`rust-src`/`rust-docs`; `rust` itself `pin_subpackage("rust-std-"+arch, exact=True)` | a `--frozen` kit can't pair `rust` 1.98.1 with `rust-std` 1.97.0 — the pinning is enforced *in the package metadata*, not by our tool |
| Relocatable by construction | `rust` output sets `binary_relocation: false`, comment: *"the distributed binaries are already relocatable"* | this is why R5 (`tar -xf`, no rattler relocation ⚠️) is worth a probe for toolchain envs instead of being dismissed 🚧 — while `sysroot_*`/`gcc_impl_*` remain prefix-relative, so expect a **link**-time failure, not a compile-time one |
| Docs are excluded | test asserts `! -d ${PREFIX}/share/doc/rust/html` and `! -f ${PREFIX}/bin/rust-analyzer` in `rust` | the compiler package is smaller than the naive "rustup installs everything" assumption; `rust-docs`/`rust-analyzer` are separate opt-ins |

### 19.1 Design deltas

| Finding | Change in this repo |
|---|---|
| Compiler can travel in a pack ✅ | §8.2 gains an IMPORTANT block; `[kit] targets-compile = "auto"` is now a *supported* configuration rather than a hopeful one; D16/D17 reworded (delegating D3 to CI is about **our box**, not about airlocks) |
| `rust-std-<triple>` is the real existence test ✅ | §7.4 note: validate toolchain-per-target availability when `targets-compile = true`, and `explain` the difference between "env runs" and "env compiles for wasm32" |
| Linker ships along ✅ | `plan`/`kit build` must report the toolchain's byte share separately (risk row added) |
| `noarch` + exact pinning ✅ | `dist-manifest.json` gains nothing new, but `kit verify` can now *state* that a frozen kit's toolchain is internally consistent by construction |
| Relocatability ✅ / sysroot ⚠️ | new 🚧 probe for M4.5: R5 (`tar -xf`) on a `rust`-bearing pack — compile a hello-world, then link one; the delta between the two is the finding |

> [!NOTE]
> Method rule earned: *"this host cannot do X"* is not evidence that *the target cannot do X*. The airlock we
> design for is the user's, and its allowlist is theirs. Measuring our own prison is only useful for the parts
> where the prison is the specification (transport, integrity, no-release-assets), which is why §18/§19 differ in
> what they conclude.
