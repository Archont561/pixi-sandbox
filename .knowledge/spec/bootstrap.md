---
type: Design Spec
title: "Bootstrap: Installing the Tool"
description: How the tool arrives on a machine that can reach nothing but git, including the pixi seed ceremony and digest verification.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, bootstrap]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["11"] }
sources:
  - { id: mintlifywiki-prefix-dev-pixi, resource: https://mintlify.wiki/prefix-dev/pixi/concepts/package-specifications, title: mintlify.wiki/prefix-dev/pixi/concepts/package-specification }
  - { id: prefix-devgithubio-pixi-v0632, resource: https://prefix-dev.github.io/pixi/v0.63.2/build/backends/pixi-build-rust/, title: prefix-dev.github.io/pixi/v0.63.2/build/backends/pixi-build- }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/cli/pixi/add/, title: pixi.prefix.dev/latest/reference/cli/pixi/add/ }
---

# Bootstrap: Installing the Tool

## 11. Bootstrap: installing the tool itself

Your phrasing was "**Rust binary I could download from a git repo via the pixi git download option**".
Here is what pixi actually supports (✅ checked against current docs), because the gap between the wish
and the feature is the crux of the design:

| Route | Works in a GitHub-only sandbox? | Notes |
|---|---|---|
| `pixi add --pypi "x @ git+https://…"` / `[pypi-dependencies] x = { git = …, rev/branch/tag = … }` | ✅ for *Python* packages | pixi's real, documented git support is **PyPI-flavoured**; it builds from an sdist → needs a compiler from conda. Not a Rust-binary story. |
| `[dependencies] x = { git = …, branch/tag/rev/subdirectory }` (conda **source** deps) | ⚠️ partially | Documented as *package specifications for source packages* (a third-party docs mirror shows the exact shape ⚠️ [mintlify mirror](https://mintlify.wiki/prefix-dev/pixi/concepts/package-specifications)); needs the `pixi-build` preview feature and a build backend (`pixi-build-rust` reads your `Cargo.toml` ✅ [pixi docs](https://prefix-dev.github.io/pixi/v0.63.2/build/backends/pixi-build-rust/)) — **and therefore needs crates.io at build time**, which is the thing a sandbox lacks. |
| `cargo install --git <url> --locked --bin pixi-sandbox` | ❌ here, ✅ on a networked machine | ✅ upstream recommends exactly this for pixi-pack. Needs **rustc** + **crates.io**. With a committed `vendor/` it becomes ✅ **offline-capable** — that is the point of [§8.2](/spec/packagers.md#82-vendor--cargo-deps--vendor--optional-branch-mirror). |
| **`git clone --depth 1 --branch dist` + checksum + chmod** | ✅ **the default** | No compiler, no registry, no root, no `curl`. `pixi-sandbox dist pull` does this, and so does the 6-line `bootstrap.sh` we ship in-repo so bootstrapping never *requires* the tool. |
| `pixi add /abs/path/pixi-sandbox-0.1.0-<plat>.conda` | ✅ fully offline | Local prebuilt conda package — explicitly supported ✅ [pixi add docs](https://pixi.prefix.dev/latest/reference/cli/pixi/add/). The dist branch can carry `.conda` artifacts *and* raw binaries; this row is why `kit` emits both. |

**`install` algorithm** (single verb, four strategies, one exit code):

```
pixi sandbox install [--url U] [--ref R] [--triple T] [--bin-dir D] [--via auto|git-branch|conda-file|cargo-git|source]
 1. resolve strategy by capability probe (rustc? pixi? vendor/? network? git on PATH?)
 2. fetch (git plumbing/clone, partial where possible)            -> staging dir
 3. verify sha256 against dist-manifest.json      FAIL = exit 4   (integrity, never "trust the mirror")
 4. install: chmod +x into D (default $PIXI_HOME/bin or ~/.pixi/bin so `pixi sandbox` also resolves)
 5. write <D>/.pixi-sandbox-origin.json {url,ref,commit,sha256,version,toolchain}   <- provenance
 6. print the exact next command ("pixi sandbox doctor --strict")
```

`auto` prefers `git-branch` (works everywhere), then `conda-file` (if pixi is present and the artifact
exists), then `cargo-git` **only if `vendor/` is present** (otherwise it would reach for crates.io — and
that is a policy the tool must refuse rather than attempt), then `source` (requires network) as a
last resort with `--allow-network` on the command line. **`--allow-network` existing as a flag is the
point**: the default path is provably network-free, so a security reviewer can stop reading.

### 11.1 The bootstrap, revised for "pixi must actually run"

```bash
# 0 + 1 + 2 in one shot: get git-only artifacts, including the pixi binary
git clone --depth 1 --branch pixi-sandbox-dist <url> kit && cd kit
sha256sum -c SHA256SUMS --ignore-missing

install -m755 bin/pixi-$(uname -m)-unknown-linux-musl        ~/.pixi/bin/pixi
install -m755 bin/pixi-sandbox-$(uname -m)-unknown-linux-musl ~/.pixi/bin/pixi-sandbox
export PATH="$HOME/.pixi/bin:$PATH"

# 3 — reconstruct the *environment*, then run the project, offline
pixi sandbox reconstruct --from . --mode auto      # picks R1 → R4, prints the rung (§9.4)
pixi run test                                      # a real task, from a real pixi workspace
```

`bootstrap.sh` (6 lines, in-repo, no build step) is the *only* code a user should ever have to read to get
started, and `apply.sh` in every kit is that same script plus the reconstruct step. If either ever needs a
network call that isn't `git fetch`, the design has failed — that is the sentence `doctor --strict`
enforces.

---
