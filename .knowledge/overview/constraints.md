---
type: Design Constraints
title: "The sandbox facts that shape every design choice"
description: The measured environment constraints (no crates.io, no prefix.dev, no release assets, git only) restated as design forcing functions, with links to the measurements.
resource: https://github.com/Archont561/pixi-sandbox
tags: [overview, constraints]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["3"] }
---

# The sandbox facts that shape every design choice

## 3. Constraints that shape the design

Measured in [the sandbox measurements](/environment/index.md); restated because every one of them becomes a requirement:

| Constraint | Design consequence |
|---|---|
| egress = **SNI allowlist behind a TLS-MITM proxy**; non-allowlisted hosts get an instant reset | Reachability must be a **first-class runtime concept** (`doctor`, per-artifact `requires-hosts`), not tribal knowledge. |
| `github.com`, `api.github.com`, `codeload.github.com` open; **`release-assets`/`raw`/`objects.githubusercontent.com` closed** | Clone/fetch/push yes; prebuilt-asset download no → publish artifacts **as git objects**, not as release assets. |
| crates.io, prefix.dev, conda channels, `pixi.sh`, `rustup.rs` closed | `pixi`/`cargo` **cannot self-install**; "install from git" must mean *fetch bytes from a ref*, and any build-from-git path must be **fully vendored**. |
| `bun` installable via `npm i bun` (postinstall pulls `@oven/bun-<plat>` from the npm registry) ✅ verified | The Node half is **prototypable in-sandbox**; the Rust half is CI-only. Test strategy ([§14](/spec/testing.md#14-testing-strategy-under-a-no-compiler-here-constraint)) is built around that asymmetry. |
| 2 vCPU, **3.8 GiB RAM, no swap**, 20 GiB disk | Keep deps tiny; never assume a big `target/` build is possible locally; **pack, don't compile**, when shipping to sandboxes. |
| `ulimit -n 1024`, `core size 0`, no TTY, `LC_CTYPE=POSIX` | Don't fan out thousands of fds; no colour/tty-dependent behaviour; never print non-ASCII without setting a UTF-8 locale; expect no crash diagnostics. |
| Patchset cap ≈128 MB / 10k files; `target/`, `vendor/`, `node_modules` are snapshot-excluded | **Large artifacts must not live on the working branch** → orphan branch + `--depth 1 --single-branch` fetch pattern; also why `vendor` gets a `strategy` switch. |
| GitHub Actions: 6 h/job, 256 matrix combos, **10 GB cache/repo**, 2 000 min/mo on private | Matrix stays ≤ 6 platforms × few jobs; `cache-write` only on `main`; packs go to git, not to artifacts (90-day retention + quota recalc lag). |
| `pixi-pack` accepts **named platforms** (lockfile v7) and is a *separate binary*; old docs used `pixi-pack pack --manifest-file` | CLI surface must be version-pinned and flag-checked at runtime, not hard-coded from memory. ✅ verified from v0.7.11 source. |
| pixi external subcommands: `pixi-<verb>` on PATH is dispatched ✅ verified in `crates/pixi_cli/src/command_info.rs` | Ship the binary **named `pixi-sandbox`** and `pixi sandbox …` works with zero registration. Free ergonomics. |
| a fresh sandbox has **no `pixi` binary at all**, and `pixi.sh` is closed ✅ | The branch is a **toolchain registry**, not just a payload store: `bin/pixi-<triple>` and `bin/pixi-sandbox-<triple>` ride along as git objects with recorded upstream digests ([§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool)). Without this, a kit is a pile of prefixes; with it, `pixi run` works. |
| **we cannot know the shape of the target repo in advance** (foreign workspaces, `pyproject.toml` vs `pixi.toml`, ephemeral agent branches) | Everything must be **derived from files present**, not from config we asked someone to write → tiered `Inventory` ([§4.4](/spec/architecture.md#44-discovery-the-inventory)) and a `plan` that prints which tier each fact came from, so a guess is never mistaken for a guarantee. |

---
