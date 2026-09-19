---
type: Correction Log
title: "Corrections and Method Notes"
description: Claims that were retracted mid-research, and the rules of thumb that emerged for working in this sandbox.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, method]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["14"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: anacondaorg-channels-conda-forge, resource: https://anaconda.org/channels/conda-forge/packages/bun/overview, title: anaconda.org/channels/conda-forge/packages/bun/overview }
  - { id: prefixdev-channels-conda-forge, resource: https://prefix.dev/channels/conda-forge/packages/bun, title: prefix.dev/channels/conda-forge/packages/bun }
  - { id: prefixdev-channels-conda-forge, resource: https://prefix.dev/channels/conda-forge/packages/bun, title: prefix.dev/channels/conda-forge/packages/bun }
---

# Corrections and Method Notes

## 14. Corrections & method notes

Keeping these in the file, because each one changed a design decision.

| # | Wrong claim in my first pass | What is actually true | How it was settled | Design impact |
|---|---|---|---|---|
| **C1** | "`bun` is not packaged on conda-forge — only an open staged-recipes request from 2023." | **`bun` *is* a conda-forge package**: `conda install conda-forge::bun` / `pixi add bun`, latest published **1.3.11**, 43 files, last updated **2026-03-18**, feedstock `conda-forge/bun-feedstock`. | User-supplied link → verified on **both** registries: [anaconda.org/channels/conda-forge/packages/bun](https://anaconda.org/channels/conda-forge/packages/bun/overview) and [prefix.dev/channels/conda-forge/packages/bun](https://prefix.dev/channels/conda-forge/packages/bun). | Toolchain resolution can treat `bun` as a **first-class conda package** — no npm detour needed for pixi users. Two real caveats survive: **no `win-64` subdir** (linux-64, linux-aarch64, osx-64, osx-arm64 only) and a **~1-month lag** behind upstream (1.3.14/1.4.0/1.4.1 sit in open feedstock PRs while bun.sh ships 1.4.2). |
| **C2** | "npm's `bun` package can't work in a blocked sandbox because its postinstall downloads from GitHub releases." | Its postinstall downloads `@oven/bun-<platform>` **from `registry.npmjs.org`** (verified in `node_modules/bun/install.js`). Since that host is allowlisted here, **bun installs and runs in this sandbox**. | `npm i bun@1.4.2` → 76 MB `node_modules/bun/bin/bun.exe`; `--version` = `1.4.2`; `bun install` resolved 4 packages, wrote a text `bun.lock`; `bun ci` = "no changes". | The Node/`node_modules` half of the tool is **prototypable locally**, not CI-only. Only the pixi/cargo half needs CI. |
| **C3** | "bun's resolver is broken by the egress proxy" (a 5-minute wrong diagnosis). | My test spec was invalid: `md5` has **7 versions, latest 2.3.0**; `2.3.5`/`^1.3.5` genuinely don't exist. `npm` rejected them with the same `notarget` error. | Fetched both the full and the **abbreviated** packument (`Accept: application/vnd.npm.install-v1+json`, the doc bun actually requests): both returned `200` with all 7 versions intact — the proxy filters nothing here. | Re-run against a **known-good target** before attributing a failure to infrastructure. Encoded as a `doctor` heuristic: probe with a pinned real artifact, never a synthetic name. |

| **C4** | "a `pixi-pack` tarball unpacked with `tar -xf` gives you *a prefix with no repodata index*, so the target can't re-solve." | The tarball **is** a channel: `pixi-pack.json` + `environment.yml` + `channel/<subdir>/*.conda` **with `repodata.json` per subdir** ✅, and pixi-pack calls it "a local channel" ✅. Pixi separately documents **`file://` channels** ✅ and counts them as available under `--offline` ✅ ⇒ the target *can* solve offline against it. Separately: `pixi-unpack` is not `tar` — it runs rattler's `Prefix::install` and writes `conda-meta/history` ✅, so a `tar -xf` fallback also loses relocation. | README + `src/unpack.rs` read in this session; pixi configuration docs. | `spec/docs-site.md §9.4`'s ladder was rebuilt around this (rung 2 = `file://` channel, rung 3 = `pixi-unpack`, `tar` demoted to a rescue), and `[mirrors]` was **removed** from the ladder as a misuse of a remote-mirror feature ✅ | 

| **C5** | "`env-*.tar.gz` from `compressed-env` branch are gzipped" | They are **plain tar** (magic `channel\0`) — `tar -tzf` fails `not in gzip format`, `tar -tf` succeeds. Only `pixi-bin.tar.gz` and `cargo-vendor.tar.gz` are real gzip. | `python3 -c open(...).read(10)` + `tar -tf` vs `tar -tzf` on restored branch `aa2a6e1`. | Kit naming must not lie: use `.tar.part_*` or `.tar.gz` truthfully; assembler must auto-detect gzip vs plain tar (sniff magic, not extension). Docs that say `tar -xzf` for a plain tar will fail on hosts without `zstd` handling. |
| **C6** | "`pixi-unpack unpack <file> --output-dir <dir>` is correct CLI" | Real 0.7.11 binary has **no `unpack` subcommand** — usage is `pixi-unpack [PACK_FILE] -o <out> --env-name <name>`. `-o` is parent dir, env becomes subdir. Old README in `compressed-env` branch is stale. | `/tmp/pixi-unpack --help` and `unpack --help` both show flat args; successful unpack required `-o .pixi/envs --env-name dev`. | Action's README and `kit/assemble` shim must be generated from measured binary help, not copied from old docs. CI should test help text snapshot. |
| **C7** | "`tar -xzf cargo-vendor.tar.gz -C .cargo/ -v | head` fully extracts" | Piping tar verbose to `head` sends SIGPIPE after 20 lines → tar aborts, leaving only 2 crates (`adler2`, `rusty-fork`). `cargo build --offline` then fails `no matching package named flate2`. | `ls .cargo/vendor | wc -l` = 2 after piped run, 124 after unpiped `tar -xzf`. | Never pipe tar extract to pager in scripts; use `tar -tf | head` for listing, separate `tar -xf` for extraction. Assembler must not use `| head` pattern. |
| **C8** | "Kit with only `pixi` binary is sufficient to reconstruct" | Dev env contains `pixi-unpack` and `pixi-pack` conda packages, but you need `pixi-unpack` to unpack dev env — **chicken-egg**. `compressed-env` only shipped `pixi`, not `pixi-unpack`, forcing manual conda `.conda` (zip → `pkg-*.tar.zst`) extraction via python `zstandard`. | Manual extraction via `zipfile` + `zstandard.ZstdDecompressor` → `bin/pixi-unpack` 16 MiB, then used to unpack all envs. | D21 kit's `bin/` must ship **pair**: `pixi-<triple>` + `pixi-unpack-<triple>` (+ `pixi-pack` for builder). `ship-pixi` flag already says pair — `compressed-env` prototype violated it. Assembler must include fallback manual extraction path or require pair. |

**Method rules that came out of this:**

1. **Registry availability must be checked *at the registry*, never via a search snippet.** Feedstock
   requests get merged and issues go stale silently; "not available" statements in an issue from 2023 are
   not evidence for 2026. Check `prefix.dev` / `anaconda.org` package pages (or `repodata.json`) directly.
2. **Distinguish three different failures** that all look like "install failed": *policy* (TLS reset,
   `http=000` — see `environment/network-model.md §4.2`), *availability* (package/version not in the channel —
   C1/C3), and *lag* (exists upstream, not yet built — bun 1.4.x). The tool needs a distinct error class
   and a distinct remediation for each.
3. **Version lag is a platform constraint, not a nuisance.** conda-forge `bun` = 1.3.x while npm/`bun.sh`
   = 1.4.2 means "install bun from conda-forge" and "install bun from npm" can yield different runtimes in
   the same org — so the packager must record *which source* produced a binary, not just its version
   (`generated.by`/provenance, cf. §11 OKF).
4. **File extensions lie — sniff magic.** `*.tar.gz` may be plain tar; `*.conda` is zip containing zst. Use `file` or magic bytes, not extension, in assembler. Documented in `pixi-pack` tar layout but easy to forget when splitting into `.part_*`.
5. **Never pipe `tar -x -v` to `head`.** Use `tar -t` for listing, `tar -x` without `-v` for extraction. SIGPIPE truncates extraction silently.
6. **Bootstrap needs pair.** Any kit that expects to unpack via `pixi-unpack` must ship `pixi-unpack` binary itself, not only inside the env it unpacks. This is D19's `bin/` pair rule — `compressed-env` prototype missed it and required manual python+zstd workaround.

---
