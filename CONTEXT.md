# CONTEXT.md — pixi-sandbox in five lines

1. **What:** a public GitHub Action that packs pixi environments + vendored crate graphs onto an orphan branch,
   so a git-only machine reconstructs a working `pixi run` environment via one clone-and-run command.
2. **State:** design corpus only (OKF v0.2 bundle in `.knowledge/`, 58 concepts); **D1–D21 decided** — M0
   closed. v1 artifact: one Rust binary as both the action's CI engine and the kit's shipped assembler; next is
   M1 (build the crate).
3. **Evidence that already ran here:** the 183-line `assemble.sh` prototype with nine stubbed-driver outcomes
   (`.knowledge/workflows/action-run.md`), and — since the 2026-09-19 host change — the **real** pipeline under
   the §16.8 probe: `pixi` 0.80.0 → `pixi-pack` 0.7.11 → `file://` channel → `pixi install/run` →
   `pixi-unpack`, all end-to-end on this box (`.knowledge/research/pixi-pack.md`). Only the sealed-container
   L3 run and the Oracle-vs-binary gate remain unproven. The §16.8 probe ran with `pixi` 0.80.0 from
   conda-forge; the box now also has a persistent `pixi` on PATH from the official installer script.
4. **Do:** edit markdown only, keep confidence labels honest, don't commit/push unless asked.
5. **Start:** `.knowledge/index.md` → [Decision Log](.knowledge/spec/decisions.md) →
   [The Assembler Binary](.knowledge/spec/assembler-binary.md) + [Testing Strategy](.knowledge/spec/testing-strategy.md).
