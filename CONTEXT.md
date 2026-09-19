# CONTEXT.md — pixi-sandbox in five lines + temporary branch guide

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
4. **Do:** edit markdown only, keep confidence labels honest, don't commit/push unless asked. **Manage via pixi tasks** — `pixi run lint/format` not raw `cargo`/`bun`/`biome` (see AGENTS.md rule). Export `PATH=".pixi/bin:$PATH"` first; if `.pixi/envs/*` missing, restore from `compressed-env` via `pixi-unpack` (see §6).
5. **Start:** `.knowledge/index.md` → [Decision Log](.knowledge/spec/decisions.md) →
   [The Assembler Binary](.knowledge/spec/assembler-binary.md) + [Testing Strategy](.knowledge/spec/testing-strategy.md).

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
   - **Handling rule:** treat `compressed-env` as **ephemeral cache**, not source of truth. Do not add features to it, do not make it depend on `main`. Final `pixi-sandbox` binary's `publish` verb will **overwrite** `pixi-sandbox-dist` (and `pixi-sandbox-bin`) with properly structured `bin/` pair `pixi`+`pixi-unpack`+`pixi-sandbox-<triple>`, `dist-manifest.json`, `SHA256SUMS`, chunk metadata, and correct `assemble` shim. Until M1 lands, keep `compressed-env` as-is for airlock restores, but document any new env in `pixi.toml` + `pixi.lock` on `main`, then repack manually if needed. After M1, delete `compressed-env` or let CI overwrite it — `sandbox.lock.json` on `main` becomes the pin.
