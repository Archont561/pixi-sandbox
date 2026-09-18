---
type: Design Spec
title: "Roadmap and Acceptance Gates"
description: Milestones M0-M5 with the gate that closes each one, including the docs and dogfood gates.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, roadmap]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["15"] }
---

# Roadmap and Acceptance Gates

## 15. Roadmap and acceptance gates

| Milestone | Delivers | Gate that must pass (objective) |
|---|---|---|
| **M0 — spec** ✅ | this document, corrected research, measured constraints | you accept [§16](/spec/decisions.md#16-decision-log) D1–D15 (or overrule them) |
| **M1 — walking skeleton** | `inventory`/`environments`/`platforms`/`plan`/`doctor`/`completions`; config+schema; argv goldens; **no writes** | `pixi sandbox inventory --json` enumerates **three foreign fixtures** (a bare `pixi.toml`, a `pyproject.toml`-flavoured workspace with `[tool.pixi.environments]`, and a multi-env repo with a deliberately missing platform) with **zero config files present**; `pixi sandbox doctor --json` correct on *this* sandbox (it must report `prefix.dev` blocked, `bun` installable, `win-64` gap); 100% of argv goldens pass; builds on rust 1.85+ and inside `pixi run cargo build` |
| **M2 — pack + unpack** | `pack`, `unpack`, `sandbox.lock.json` | a `linux-64` pack, unpacked by `tar -xf` **with no conda/pixi on PATH**, imports `numpy` (or `rustc --version`) |
| **M3 — vendor + offline build** | `vendor sync/check/prune`, `.cargo/config.toml`, branch strategy | `cargo build --locked --offline --target x86_64-unknown-linux-musl` succeeds on a machine with **egress disabled** |
| **M4 — transport** | `dist push/pull/tag/bundle`, `dist-manifest.json`, `mirror binary`, **`bin/pixi-*` shipped** | fresh sandbox: the bootstrap in [§11.1](/spec/bootstrap.md#111-the-bootstrap-revised-for-pixi-must-actually-run) yields `pixi --version` **and** `pixi sandbox --version`, `kit verify` is green, and **zero non-git network calls** were made (asserted by `strace`/egress-allowlist in CI, not by eyeballing) |
| **M4.5 — reconstruction** | `reconstruct --mode auto` over R1→R5, `reconstruct-report.json`, `--print-rung` | a kit built on linux-64 makes `pixi run <task>` work in **three** containers: R1 (copied `$PIXI_CACHE_DIR`), **R2 (`file://` channel + `pixi install --frozen`)**, and R5 (`tar` only). R2 green ⇒ the guarantee holds and `cache/pkgs/` becomes optional; R2 red while its pieces are documented ⇒ we file the upstream issue with the minimal repro and make R3 the promise. Either way the *decision is a CI outcome, not a taste call* |
| **M5 — kit** | `kit build/verify/apply`, embedded `apply.sh`/`ps1` | kit produced on Linux **unpacks and works** on macOS-arm64 and Linux-aarch64 (per-platform tars, host selection correct) |
| **M6 — node** (optional) | `node sync/check/pack/unpack`, determinism | same tree → same sha256 across two runs; `bun.lockb` fails with the migration command |
| **M4.6 — docs pipeline** (independent of everything else, and the only milestone this box can execute today ✅) | `docs/` Astro project + `docs-sync` (copy → title → alerts→asides → link/base rewrite), `pixi-sandbox docs build/pack/publish/check`, Pages workflow, `pixi-sandbox-docs` mirror branch | `pixi run docs-check` green **on the airlocked box** (npm-only deps ✅) and `All internal links are valid`; the mirror branch readable after `reconstruct --with-docs` with Pagefind search working offline |
| **M7 — self-hosting + release** | CI publishes `pixi-sandbox` **and** the mirrored `pixi` to the dist branch; `.conda` artifact; `install`; `NOTICE.md` | `pixi add <abs path>.conda` from the dist branch installs the tool that produced it, **and** `pixi sandbox reconstruct --mode auto && pixi run test` works in a container that never touched the network except `git fetch` — the closure property in [§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool) |

Recommended order of *effort*: M1's argv layer is where correctness is won; M4 is the only genuinely
novel code (git plumbing, ~150 lines); M2/M3 are thin delegation and should stay boring. **M4.5 is the
one milestone that can invalidate a design decision** (whether copied-cache reconstruction works), so
schedule it early and cheaply: a 30-line CI job, not a polished verb.

---
