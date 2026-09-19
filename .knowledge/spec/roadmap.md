---
type: Design Spec
title: "Roadmap and Acceptance Gates"
description: Milestones M0-M5 with the gate that closes each one, including the docs and dogfood gates.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, roadmap]
status: stable
confidence: reasoned
generated: { by: arena-agent/agent-mode, at: 2026-09-19T23:55:00Z }
legacy: { files: [`DESIGN.md`], sections: ["15"] }
---

# Roadmap and Acceptance Gates

## 15. Roadmap and acceptance gates

| Milestone | Delivers | Gate that must pass (objective) |
|---|---|---|
| **M0 — spec** ✅ | this document, corrected research, measured constraints | you accept [§16](/spec/decisions.md#16-decision-log) D1–D22 (or overrule them) |
| **M1 — the binary + the action + reusable wrapper** ✅ | the `pixi-sandbox` Rust crate (`pixi-sandbox` bin + `pixi-sandbox-core` lib per D22 Cargo convention) — CI verbs (`pack`/`publish`/`plan`) and target verb (`reconstruct`) — plus `action.yml` (primary, marketplace, step-level) whose steps call binary, plus `reusable-publish.yml` (`workflow_call`) enforcing runner matrix = platform coverage and calling `./` internally; config+schema basic; argv goldens; musl-static P0 (`x86_64-unknown-linux-musl`) via `build.yml` matrix | L1+L2 of the [Testing Strategy](/spec/testing-strategy.md) green in CI — including **nine `assemble.sh` oracle vectors replayed against compiled `reconstruct`** (T1 full kit rung3, T2 no pixi-unpack→rung2, T3 rung2 fail→rung4 micromamba, T4 channel-shaped no driver→refuse E-UNAVAILABLE, T5 raw tar→rung5 warning, T6 SHA256 mismatch→E-INTEGRITY 4 before exec, T7 cargo metadata fail→E-VENDOR-INCOMPLETE 8, T8 missing env/driver→E-NOT-DETECTED/E-UNAVAILABLE, T9 GITHUB_PATH+promote-path idempotent); `action.yml` + `reusable-publish.yml` parse via `actionlint`; musl-static P0 built in CI (`ldd` says not dynamic + alpine smoke-run); builds on rust 1.85+ |
| **M1.5 — end-to-end with the real packagers** | one CI job: real `pixi-pack`/`cargo vendor` on `ubuntu-latest` → publish to throwaway dist branch → reconstruct in **clean alpine container** via one-liner (`pixi-sandbox reconstruct --from kit`) | `pixi list` non-empty, `pixi run <task>` exits 0, `cargo test` offline when crates are bundled, and **zero non-git egress** during reconstruction (strace/allowlist, not eyeballing). The oracle vectors are the acceptance table ✅ |
| **M2 — pack + unpack** | `pack`, `unpack`, `sandbox.lock.json` (binary verbs) | a `linux-64` pack, unpacked with **no conda/pixi on PATH**, imports `numpy` (or `rustc --version`) |
| **M3 — vendor + offline build** | `vendor sync/check/prune`, `.cargo/config.toml`, branch strategy | `cargo build --locked --offline --target x86_64-unknown-linux-musl` succeeds on a machine with **egress disabled** |
| **M4 — transport** | `dist push/pull/tag/bundle`, `dist-manifest.json`, `mirror binary`, **`bin/pixi-*` + `bin/pixi-sandbox-*` shipped** | fresh sandbox: the bootstrap in [§11.1](/spec/bootstrap.md#111-the-bootstrap-revised-for-pixi-must-actually-run) yields `pixi --version` **and** `pixi-sandbox --version` (from the mirrored kit binary, no install), `kit verify` is green, and **zero non-git network calls** were made |
| **M4.5 — reconstruction ladder** | `reconstruct --mode auto` over R1→R5, `reconstruct-report.json`, `--print-rung` (binary) | a kit built on linux-64 makes `pixi run <task>` work in **three** containers: R1 (copied `$PIXI_CACHE_DIR`), **R2 (`file://` channel + `pixi install --frozen`)**, and R5 (`tar` only). R2 green ⇒ the guarantee holds and `cache/pkgs/` becomes optional; R2 red while its pieces are documented ⇒ we file the upstream issue with the minimal repro and make R3 the promise. Either way the *decision is a CI outcome, not a taste call* |
| **M5 — kit** | `kit build/verify/apply` (binary) | kit produced on Linux **unpacks and works** on macOS-arm64 and Linux-aarch64 (per-platform tars, host selection correct) |
| **M6 — node** ❌ dropped (D20) | — | resurrect only against a measured need; `[node]` keys stay reserved so a stale config errors instead of silently no-op-ing |
| **M4.6 — docs pipeline** (independent of everything else, and the only milestone this box can execute today ✅) | `docs/` Astro project + `docs-sync` (copy → title → alerts→asides → link/base rewrite), `pixi-sandbox docs build/pack/publish/check`, Pages workflow, `pixi-sandbox-docs` mirror branch | `pixi run docs-check` green **on the airlocked box** (npm-only deps ✅) and `All internal links are valid`; the mirror branch readable after `reconstruct --with-docs` with Pagefind search working offline |
| **M7 — self-hosting + release** | CI publishes the `pixi-sandbox` binary **and** the mirrored `pixi`/`pixi-unpack` to the dist branch; `.conda` artifact; `install`; `NOTICE.md` | `pixi add <abs path>.conda` from the dist branch installs the tool that produced it, **and** `pixi-sandbox reconstruct --mode auto && pixi run test` works in a container that never touched the network except `git fetch` — the closure property in [§10.5](/spec/git-registry.md#105-self-hosting-the-branch-ships-pixi-and-the-tool) |

**Order under D21/D22 (decided 2026-09-19):** M0 → **M1** (Rust binary is action's CI surface and kit's target surface + reusable workflow wrapper enforcing runner matrix — [The Assembler Binary](/spec/assembler-binary.md), [Action Shape D22](/spec/action-shape.md#d22-action-is-primary-reusable-workflow-is-wrapper-2026-09-19)) → **M1.5** (end-to-end with real packagers; gate that can falsify design) → M2/M3 → M4.5 → M4/M7. Nine `assemble.sh` outcomes are acceptance vectors all along ([Testing Strategy](/spec/testing-strategy.md)). Numbering stable because other concepts cite ids. **This sandbox now runs L1+L2** (cargo test offline green for T1-T9); L0 design checks (actionlint, biome) remain.

Recommended order of *effort*: M1's argv layer is where correctness is won; M4 is only genuinely novel code (git plumbing, ~150 lines); M2/M3 are thin delegation and should stay boring. **M4.5 is the one milestone that can invalidate a design decision** (whether copied-cache reconstruction works), so schedule it early and cheaply: a 30-line CI job, not a polished verb.

---
