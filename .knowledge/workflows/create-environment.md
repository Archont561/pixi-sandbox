---
type: Playbook
title: "Creating and Packing an Environment"
description: Step-by-step: pixi init, add packages with platforms, tasks, cargo init, sandbox init, kit build - on a networked box.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, packing]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["1"] }
sources:
  - { id: githubcom-rust-lang-cargo, resource: https://github.com/rust-lang/cargo/issues/10729, title: rust-lang/cargo — issues/10729 }
---

# Creating and Packing an Environment

## 1. Workflow A — creating an environment (on a networked box)

### 1.1 Day 0 of a new project

```bash
pixi init myproj && cd myproj                       # pixi.toml (+ empty pixi.lock)
pixi add --platform linux-64 --platform osx-arm64 \
    rust=1.95 python=3.12 bun nodejs=24 git         # conda-forge; ✅ bun is on conda-forge (1.3.11)
pixi add --pypi "rich>=13"                          # PyPI deps live in the SAME pixi.lock ✅
pixi task add build "cargo build --release"
pixi task add test  "cargo test --locked"
pixi task add lint  "bun run --bun eslint ."
cargo init --name myproj                             # or: an existing crate in this dir
```

Then the two lines that make this repo sandbox-portable — and *only* these:

```bash
pixi sandbox init            # writes pixi-sandbox.toml; optional, see below
pixi sandbox kit build       # detects lockfiles → packs + vendor + node + bin/* + channel/
```

`init` is optional **by design**: `kit build` derives components from the lockfiles it finds
(`spec/architecture.md §4.4`). You write `pixi-sandbox.toml` when CI needs a fixed env list, when you want
`[platforms] default` to differ from the host, or when a `known-gaps` entry must be recorded
(`spec/toolchain-resolution.md §7.4`). Nothing in the local flow requires it.

### 1.2 Adding a dependency later (the everyday case)

```bash
pixi add serde_json                       # ❌ wrong tool: that's a *crate*, not a conda package
cargo add serde_json && cargo generate-lockfile   # ✅ crates go through cargo
pixi add rust=1.96                        # ✅ toolchain bumps go through pixi
bun add zod                               # ✅ JS dep → updates package.json + bun.lock
git add pixi.lock Cargo.lock vendor .cargo/config.toml && git commit
pixi sandbox vendor sync                  # MUST run after Cargo.lock changes (§4.3)
pixi sandbox kit build && pixi sandbox dist push   # republish (Workflow C does this in CI)
```

> [!WARNING]
> Order matters in one specific place: `cargo add` **before** `vendor sync`. With
> `[source.crates-io] replace-with = "vendored-sources"` active, `cargo add` fuzzy-matches inside
> `vendor/` and can "succeed" against the wrong crate ✅ [cargo#10729](https://github.com/rust-lang/cargo/issues/10729).
> `pixi sandbox vendor disable` is the escape hatch the tool prints instead of letting you lose an hour.

### 1.3 What a workspace looks like after a kit build

```
myproj/
├── pixi.toml  pixi.lock            # the two files pixi needs to be a workspace
├── Cargo.toml Cargo.lock          # the two files cargo needs to be a package
├── vendor/  .cargo/config.toml    # ONLY if the target must compile (decision in §4.3)
├── package.json bun.lock
├── pixi-sandbox.toml              # optional
├── sandbox.lock.json              # committed: what the kit contains + digests
└── .gitignore                     # /sandbox/  /target/  (artifacts → dist branch)
```

---
