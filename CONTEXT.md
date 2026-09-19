# CONTEXT.md — pixi-sandbox in five lines

1. **What:** a public GitHub Action that packs pixi environments + vendored crate graphs onto an orphan branch,
   so a git-only machine reconstructs a working `pixi run` environment via one clone-and-run command.
2. **State:** design corpus only (OKF v0.2 bundle in `.knowledge/`, 58 concepts); **D1–D21 decided** — M0
   closed. v1 artifact: one Rust binary as both the action's CI engine and the kit's shipped assembler; next is
   M1 (build the crate).
3. **Evidence that already ran here:** the 183-line `assemble.sh` prototype, nine measured outcomes with stubbed
   drivers (`.knowledge/workflows/action-run.md`); everything needing rustc/pixi is CI-shaped and unproven.
4. **Do:** edit markdown only, keep confidence labels honest, run the validator (`python3 val.py`, expect
   `ALL CLEAN`), don't commit/push unless asked.
5. **Start:** `.knowledge/index.md` → [Decision Log](.knowledge/spec/decisions.md) →
   [The Assembler Binary](.knowledge/spec/assembler-binary.md) + [Testing Strategy](.knowledge/spec/testing-strategy.md).
