---
type: Problem Statement
title: "The one idea: a git ref is an artifact registry"
description: The problem statement and the single mechanism that solves it: pin portable artifacts on orphan branches of this repo so a sealed box can reconstruct everything it needs.
resource: https://github.com/Archont561/pixi-sandbox
tags: [overview, core-idea]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["1"] }
sources:
  - { id: pixish-install, resource: https://pixi.sh/install, title: pixi.sh/install }
---

# The one idea: a git ref is an artifact registry

## 1. Problem, and the one idea that solves it

You want to work in a locked-down environment where **only git is reliably open**, and still get a
reproducible, three-ecosystem dev setup:

* **pixi** as the environment manager, because conda-forge gives you `rust`, `python`, **and `bun`** as
  ordinary packages — including their native/system libraries, which is the part pip and npm cannot do.
* **`pixi-pack`** to freeze those environments into portable archives.
* **`cargo vendor`** to do the same for Rust *source* deps, so a build needs no registry.
* optionally **`node_modules`** snapshotted the same way.
* delivered by **a Rust binary named `pixi-sandbox`**, ergonomic, installable from a git repo.

**The core idea: treat a git ref as an immutable, content-addressed artifact registry.**
Not "git as a source host that then goes and fetches from crates.io" — the *artifacts themselves*
(`environment.tar`, `vendor.tar.zst`, `node_modules.tar.gz`, the `pixi-sandbox` binary) live in an
**orphan branch** of the same repo, addressed by SHA-256, and are consumed by `git clone --depth 1
--branch <dist>`. Everything else in the design follows from that.

Why this and not the obvious alternatives:

| Alternative | Why it fails here |
|---|---|
| GitHub Releases assets | Host is `release-assets.githubusercontent.com` → **TLS reset** (measured). Even `pixi` itself cannot be fetched this way. |
| `ghcr.io` OCI mirrors / container base image | `ghcr.io` → blocked (measured). |
| `curl -fsSL https://pixi.sh/install \| bash` | `pixi.sh` → blocked (measured). |
| Conda channel + `pixi add rust` | `prefix.dev`, `conda.anaconda.org` → blocked (measured). |
| apt + `rustup` | `sh.rustup.rs` blocked; port 80 to `deb.debian.org` refused → `apt-get update` cannot work. |
| git-LFS | Needs the LFS *batch API* on a separate endpoint + `git-lfs` binary; not installed here, and it multiplies failure modes. Keep as an **option**, never the default. |
| Submodules (one repo per artifact) | `git submodule` needs network to *another* ref and still gives no integrity story; orphan branch + manifest is strictly simpler. |

**Net effect:** one protocol (`git`), one integrity story (SHA-256 in a committed manifest), one place to
audit (`git log`), and a repo that is self-describing offline. The tool's job is to make all of that
*automatic and idempotent*, so the human types `pixi sandbox kit build` and gets a directory that can
rebuild the world on a machine with no network.

---
