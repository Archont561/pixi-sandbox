---
type: Decision Log
title: "Decision Log D1-D18"
description: Every design decision, the alternative rejected, why, and its status (decided / open / corrected).
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, decisions]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["16"] }
sources:
  - { id: githubcom-googlecloudplatform-knowledge-catalog, resource: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md, title: GoogleCloudPlatform/knowledge-catalog — blob/main/okf/SPEC.md }
---

# Decision Log D1-D18

## 16. Decision log

| # | Decision | Alternatives rejected | Why |
|---|---|---|---|
| D1 | Artifacts ride **orphan git branches**, addressed by sha, pinned by `dist-manifest.json` | Releases/GHCR/apt/`curl\|sh` (all blocked); LFS (extra binary + endpoint); raw file URLs (blocked) | Turns the one open protocol into the delivery mechanism; `--depth 1`+partial clone keeps it cheap |
| D2 | Ship the binary **named `pixi-sandbox`** → `pixi sandbox …` | a separate `pixi-sandbox-plugin` + manual PATH edits | pixi's `find_external_subcommand` looks for exactly `pixi-<verb>` ✅ verified; zero-registration ergonomics, and `pixi --list` discovers it |
| D3 | **Zero-HTTP, zero-async**: orchestrate existing CLIs via `Cmd` | reqwest/tokio "so we can download directly" | the sandbox proves *why*: any host we hard-code becomes a policy question; tiny tree (clap/thiserror/serde/toml/sha2/hex) makes `cargo vendor` cheap and the audit surface small |
| D4 | **Plan/apply split with `Action` objects** | execute commands inline | the only way to get behavioural tests without pixi/cargo present; makes dry-run and `--json` first-class instead of decorative |
| D5 | `sandbox.lock.json` **committed on main**, payloads on dist | one big committed `sandbox/` dir | keeps the human branch reviewable and the patchset under the ~128 MB cap (measured) |
| D6 | Verification is always allowed offline; **generation is refused** without `--allow-network` | "try and fail with a TLS error" | a sandboxed agent must not burn 5 minutes discovering what `doctor` can state in 50 ms |
| D7 | `unpack` fallback chain down to **`tar -xf` + local channel** | require pixi/conda on the target | a kit that needs the package-manager-to-install-the-package-manager is circular; pixi-pack's own docs bless the local-channel path ✅. **Extended by D15**: after requirement (e) arrived, this became rung **R3** — the *guarantee*, not the promise |
| D8 | `bun` primary source = **conda-forge**, fallback = **npm** | npm-only (my first, wrong, read) | conda-forge `bun` **exists** (1.3.11) ✅ corrected after review; npm is kept as fallback precisely because it works even in a GitHub-only sandbox (C2) and because the channel lags |
| D9 | `vendor.strategy = "branch"` default, `"commit"` opt-in | always commit `vendor/` | 4 crates of deps is a few MB — fine for this repo; a `reqwest`-sized graph is not. Same knob serves both, and `skip` exists for "we only verify" |
| D10 | Degraded platforms produce **`omitted` entries + exit 5**, never silence | skip with a warning; fail the whole run | `bun` has no `win-64` ✅ — partial kits must be *legible*, and a per-platform failure must not poison an all-platform run |
| D11 | Docs also an **OKF bundle** (`type:` front matter, `index.md`, `log.md`, path-as-identity) | a single README, or a wiki | the primary audience is an agent with a context budget; OKF needs no tooling ✅ [SPEC](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md); `generated.by/at` mirrors §9.1's provenance, so docs and artifacts use one vocabulary |
| D12 | **Detect, then ask** — an `Inventory` built in tiers L0–L3, config only as override; **arbitrary environments need no config edit** | config-required verbs (my §5 draft); "just edit `pixi-sandbox.toml`" | the sandbox's whole point is foreign repos and ephemeral branches; a tool that can't say "here are the 4 environments and the platforms each locks" is a config-file wrapper, not a helper |
| D13 | **Component selection is derived from lockfiles** (`Cargo.lock`→vendor, `pixi.lock`→env, JS lockfile→node, built binary→self), with `--components` to override | always bundle everything; require the user to enumerate | bundling `vendor/` for a Python-only repo is pure weight; forgetting it for a Rust repo is a broken kit; the marker files are already the answer, and "not applicable" must never be an error |
| D14 | **The kit ships the drivers**: `bin/pixi-<triple>` + `bin/pixi-sandbox-<triple>` (+ optionally `pixi-pack`/`pixi-unpack`), and `NOTICE.md` | depend on pixi being installed; depend on `pixi.sh`/release assets (blocked ✅); rebuild pixi from source in CI | "all pixi tasks keep working" is only true if pixi exists on the target, and here the only trusted host is the branch; rebuild-from-source is a `kit`-size multiplier and buys nothing a mirrored binary + digest doesn't |
| D15 | **Reconstruction is a ladder** — R1 copied cache → **R2 the pack's `channel/` as a `file://` channel** → R3 `pixi-unpack` (rattler `Prefix::install` ✅) → R4 `environment.yml` + micromamba ✅ → R5 `tar`-only — selected by probe, printed with `--print-rung` | promise one method; promise "full pixi offline"; ship `mirrors`-based resolution as if it were local | after reading pixi's own config docs, **two rungs are documented and three are not**, and they degrade in exactly one direction (solving ability). A ladder makes that gradation the contract; `[mirrors]` was demoted out of it because its documented meaning is *exact-copy remote channel* ✅, not "a directory" |
| D18 | **Docs: Starlight+Pages *and* a mirrored copy in an orphan branch**, `[docs] enabled = false` by default | README-only / site-only / a second docs stack (MkDocs, Sphinx) | search + offline reading on the target is the only real payoff, and `*.github.io` is unreachable from an airlock ✅ so the mirror (`pixi-sandbox-docs` branch, 5.7 MB ≈ 2 s) is what makes the site exist for the people this project is for; bonus: it is the one build this box can run, so it is the cheapest dogfood tier ✅ ([§9.6](/spec/docs-site.md#96-docs-a-starlight-site-published-twice--once-for-browsers-once-for-the-airlock)) |
| D17 | **Dogfood by tiers**: D0 fixtures + D1 transport run on the airlocked box with no toolchain; D2 needs a manually seeded `pixi`/`pixi-pack` binary in a branch (verified via the GitHub asset digest ✅); D3 compilation is delegated to CI via throwaway `pixi-sandbox-wip-<topic>` dist branches; D4 = the `dogfood` CI job promotes only when released-binary == expected behaviour | pretend the local box can build it / forbid binaries entirely | the bare box has **no route to `rustc`** (npm `rustup@1.0.10` / PyPI `rustup@1.29.0.1` are wrappers around a blocked host ✅) — although conda-forge's `rust` *is* the toolchain ✅, so a **pack** can carry a compiler for users while *we* delegate D3 to CI; a tier ladder with one explicit seed is the honest loop, and it doubles as the user-facing onboarding test |
| D16 | **`vendor` is conditional on `[kit] targets-compile`; by default a kit ships the built binary via `pixi-pack --inject <own .conda>` ✅** | always vendoring; expecting `pixi-pack` to carry the crate graph | a pack is conda packages + wheels ✅ — `.crate` files are not in it and `--inject` won't take them; 400 crates of build-time source for a target that only runs a binary is pure weight, while `--inject`-ing the project's own `.conda` is the documented pattern for "you build the project itself and want it in the environment" ✅ |

---
