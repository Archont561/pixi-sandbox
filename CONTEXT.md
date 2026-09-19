# CONTEXT.md — pixi-sandbox in five lines + temporary branch guide

1. **What:** a public GitHub Action that packs pixi environments + vendored crate graphs onto an orphan branch,
   so a git-only machine reconstructs a working `pixi run` environment via one clone-and-run command.
2. **State:** design corpus (OKF v0.2 bundle in `.knowledge/`, 58 concepts); **D1–D22 decided** — M0 closed, **M1 closed** (binary + action + reusable wrapper). Crate layout: `crates/pixi-sandbox` bin + `crates/pixi-sandbox-core` lib (renamed from `sandbox-core` per Cargo convention D22). v1 artifact: one Rust binary as both action's CI engine and kit's shipped assembler; next is M1.5 (real `pixi-pack`/`cargo vendor` end-to-end in alpine). Action primary (`action.yml` composite, marketplace), reusable wrapper (`reusable-publish.yml` `workflow_call`) enforces runner matrix = platform coverage per D22.
3. **Evidence that already ran here:** the 183-line `assemble.sh` prototype with nine stubbed-driver outcomes
   (`.knowledge/workflows/action-run.md`), and — since the 2026-09-19 host change — the **real** pipeline under
   the §16.8 probe: `pixi` 0.80.0 → `pixi-pack` 0.7.11 → `file://` channel → `pixi install/run` →
   `pixi-unpack`, all end-to-end on this box (`.knowledge/research/pixi-pack.md`). **M1 gate now green in CI:** L1+L2 `cargo test --offline` replays T1-T9 oracle vectors against compiled `reconstruct` (14 tests), CLI goldens 5 tests, `clippy -- -D warnings`, `fmt`, `biome`, `actionlint` (both `action.yml` + `reusable-publish.yml`). The §16.8 probe ran with `pixi` 0.80.0 from
   conda-forge; the box now also has a persistent `pixi` on PATH from the official installer script.
4. **Do:** edit markdown only, keep confidence labels honest, don't commit/push unless asked. **Manage via pixi tasks** — `pixi run lint/format` not raw `cargo`/`bun`/`biome` (see AGENTS.md rule). Export `PATH=".pixi/bin:$PATH"` first; if `.pixi/envs/*` missing, restore from `compressed-env` via `pixi-unpack` (see §6).
5. **Start:** `.knowledge/index.md` → [Decision Log](.knowledge/spec/decisions.md) →
   [The Assembler Binary](.knowledge/spec/assembler-binary.md) + [Testing Strategy](.knowledge/spec/testing-strategy.md) + [Action Shape D22](/spec/action-shape.md#d22-action-is-primary-reusable-workflow-is-wrapper-2026-09-19).

6. **Temporary env branch `compressed-env` (will be overwritten):**
   - **What it is:** orphan branch `aa2a6e1` (15 files, 530 MiB, chunked ≤45 MiB) — manual prototype of final `pixi-sandbox-dist` / `pixi-sandbox-bin`. Contains `pixi-bin.tar.gz` (pixi 0.81.0), `cargo-vendor.tar.gz` (124 crates), `env-dev.tar.gz.part_00..08` (398 MiB, rust 1.98.1 + pixi-pack/unpack), `env-docs` (bun 1.3.11, biome), `env-utils` (lefthook, convco, actionlint). File type gotcha: `env-*.tar.gz` are plain tar, not gz.
   - **How to restore (airlock-safe, no network):**
     ```bash
     git checkout compressed-env -- cargo-vendor.tar.gz pixi-bin.tar.gz env-*.tar.gz.part_* env-*.tar.gz
     for f in *.part_00; do base="${f%.part_00}"; cat "${base}.part_"* > "$base"; done
     mkdir -p .pixi/bin .cargo && tar -xzf pixi-bin.tar.gz -C .pixi/bin && tar -xzf cargo-vendor.tar.gz -C .cargo/
     # bootstrap pixi-unpack if bin/ only has pixi:
     python3 - << 'PY' # zipfile + zstandard → bin/pixi-unpack from channel/linux-64/pixi-unpack-*.conda
     import zipfile,tarfile,zstandard,pathlib; ...
     PY
     /tmp/pixi-unpack env-dev.tar.gz -o .pixi/envs --env-name dev
     /tmp/pixi-unpack env-docs.tar.gz -o .pixi/envs --env-name docs
     /tmp/pixi-unpack env-utils.tar.gz -o .pixi/envs --env-name utils
     rm -f *.tar.gz *.part_*
     ```
     Correct CLI is `pixi-unpack <file> -o <parent> --env-name <name>`, not `pixi-unpack unpack --output-dir`. Never `tar -v | head` (SIGPIPE truncates vendor). Full steps measured in `.knowledge/research/pixi-pack.md §16.9` and `.knowledge/environment/inventory.md §7`.
   - **Handling rule:** treat `compressed-env` as **ephemeral cache**, not source of truth. Do not add features to it, do not make it depend on `main`. Final `pixi-sandbox` binary's `publish` verb will **overwrite** `pixi-sandbox-dist` (and `pixi-sandbox-bin`) with properly structured `bin/` pair `pixi`+`pixi-unpack`+`pixi-sandbox-<triple>`, `dist-manifest.json`, `SHA256SUMS`, chunk metadata, and correct `assemble` shim. **M1 now closed** (binary + action + reusable wrapper green), but keep `compressed-env` for airlock restores until M1.5 (real packagers end-to-end) lands, then delete it or let CI overwrite — `sandbox.lock.json` on `main` becomes the pin. Document any new env in `pixi.toml` + `pixi.lock` on `main`, then repack manually if needed.
