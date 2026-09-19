---
type: Open Questions
title: "What Is Still Unproven"
description: The honest gap register: every claim in this design that is inferred, with the experiment that would settle it.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, uncertainty]
status: stable
confidence: open
generated: { by: arena-agent/agent-mode, at: 2026-09-19T12:00:00Z }
verified:
  - { by: process:sandbox-measurement, at: 2026-09-19T11:30:00Z }   # §16.8 probe settles the pack-is-a-channel row
legacy: { files: [`WORKFLOWS.md`], sections: ["6"] }
---

# What Is Still Unproven

## 6. What is still unproven

| Claim here | Status | How it gets settled |
|---|---|---|
| `file://` local channels are supported by pixi | ✅ documented (manifest form + `pixi project channel add file:///…`) | nothing pending for *support*; M4.5 measures the *composition* with a frozen install |
| A pack's `channel/` + `repodata.json` is directly consumable by pixi as that channel | ✅ **measured 2026-09-19** — the §16.8 probe in [pixi-pack](/research/pixi-pack.md) pointed a consumer project at a real pack's extracted `channel/`, and `pixi add libcurl --offline` → `pixi install --offline` → `pixi run icuinfo` all worked with no channel access | the remaining delta is M1.5's *sealed-container* run of the same probe (`--offline` used here; `--frozen` + no-network container is the stronger claim) |
| `pixi build` + `pixi-build-rust` can build offline from `vendor/` | ⚠️ inferred from documented `cargo install` + `extra-input-globs`, never tried | `reconstruct-e2e` variant with `egress: off` |
| `repodata-config.disable-sharded` is needed for a tiny local channel | 🚧 | one run; if unnecessary, the generated config drops it |
| tar-only unpack leaves prefixes unrelocated | ⚠️ inferred (rattler's `Prefix::install` does the work in the other rungs ✅) | `grep -r /opt/conda .local-env/bin` after each rung — it is a 10-second experiment |
| `--offline` is respected by build backends | ✅ **documented as NOT respected** | none — the design already avoids source deps on targets |
| How to attach remark/rehype plugins to content-collection markdown in Astro 7 | ⚠️ `markdown: unified({…})` is accepted but had **no effect**, and setting `markdown` at all broke Starlight's own pipeline ✅ | avoided by design (`docs-sync` rewrites alerts → native asides); settle it only if a plugin becomes unavoidable, and then via the Sätteri/config-reference page |
| `tar -xf` (R5) on a pack whose env contains `rust` | 🚧 plausible: the recipe sets `binary_relocation: false` because *"the distributed binaries are already relocatable"* ✅ | compile *and* link a hello-world from an R5 prefix — the delta between the two results **is** the finding (`sysroot_*`/`gcc_impl_*` are prefix-relative ⚠️) |
| `rehype-mermaid` (build-time SVG, no client JS) vs `astro-mermaid` (client-side, ~1 MB of the 3.7 MB `_astro/`) | 🚧 | build the site once with each and diff `dist/`; matters only if a target reads docs without JS |
| `.nojekyll` still needed for `withastro/action` deploys | ⚠️ the current Astro Pages guide no longer mentions it ✅ (it protected the older `gh-pages`-branch method) | keep shipping the empty file (harmless ✅) and drop it if a Pages deploy proves it irrelevant |
| The action's **CI half** (`pack.sh` + `publish.sh`) against the *whole* flow: `pixi-pack` packing, `cargo vendor`, artifact mirroring to the branch | 🚧 partial — the script *decisions* are measured (nine fixtures ✅) and the **driver half is now real**: the §16.8 probe ([pixi-pack](/research/pixi-pack.md)) packed a real project with pixi-pack 0.7.11 and reconstructed offline from the pack's channel ✅. Still unrun: the action's own `pack`/`publish` verbs end-to-end on a real runner (branch push, digest-pin commit, kit assembly) | one M1.5 run on a real runner |
| Under [D21](/spec/decisions.md): the Rust assembler **compiles at all**, and reproduces the nine `assemble.sh` oracle vectors | 🚧 the crate is not written (M1); **the "no rustc here" excuse is gone** — conda-forge `rust` 1.98.1 installs and runs on the current host (inventory §6), so mere compilation is no longer CI-shaped; what still needs the runner is the *oracle gate* (L1/L2 `cargo test` + the L3 clean one-liner) | `cargo test` (L1/L2) + the L3 clean-container one-liner in [Testing Strategy](/spec/testing-strategy.md); the oracle vectors are the acceptance gate |
| Under [D21](/spec/decisions.md): a musl-static `pixi-sandbox` binary **runs on any Linux with zero runtime deps** (alpine included) | ⚠️ inferred from pixi-pack's own musl release assets ✅ — same shape, but our binary has never been linked | the L3 alpine-container reconstruction; a segfault/`not found` there falsifies the musl assumption |
| `assemble.ps1` walks the same ladder on Windows | 🚧 never executed — no PowerShell in this sandbox ✅ (absent) | a `windows-latest` job reconstructing a fixture kit; until then Windows kits are R5-only by policy |
| A pack built on `ubuntu-latest` unpacks on *any* `linux-64` target | ⚠️ assumed: the same-platform rule is documented, glibc/sysroot drift between runners and targets is not | reconstruct on `alpine` and a debian-oldstable container in one CI run |
| `*.tar.gz` is really gzipped | ❌ **falsified 2026-09-19**: `compressed-env` branch's `env-*.tar.gz` are plain tar (magic `channel\0`), `tar -tzf` fails | assembler must sniff magic, not extension; doc chunked naming `.tar.part_*` vs `.tar.gz.part_*` | 
| `pixi-unpack unpack --output-dir` CLI | ❌ **stale**: real 0.7.11 binary has no `unpack` subcommand; correct is `pixi-unpack <file> -o <parent> --env-name <name>` | generate help from binary in CI, not copy old README |
| Kit with only `pixi` binary sufficient | ❌ **falsified**: dev env contains `pixi-unpack`, but need `pixi-unpack` to unpack dev env — chicken-egg; `compressed-env` prototype shipped only pixi, required manual `.conda` zstd extraction via python | ship pair `pixi`+`pixi-unpack` in `bin/` (D19 `ship-pixi` already says pair); assembler fallback manual extraction |
| `tar -x -v | head` safe | ❌ **falsified**: SIGPIPE truncates extraction after 20 lines → only 2 crates | never pipe tar extract to pager; separate list vs extract |
| Chunked `.part_*` reassembly via `cat` | ✅ **measured 2026-09-19**: `cat env-dev.tar.gz.part_* > env-dev.tar.gz` restores 398 MiB from 9×45 MiB chunks, git transports 530 MiB branch | document in `kit/assemble` shim and `dist-manifest.json` chunking field |

> [!NOTE]
> **Settled by D19 + D21** (2026-09-19): "can a machine with only git open a kit?" — the question that motivated
> the bootstrap ceremony, the seed cache and `--self-test`. D19 first answered it by removing the compiled tool
> (the script ran in this sandbox ✅,
> [measured](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox)); D21 then brought the tool
> back *without* reopening the question — the assembler binary ships **inside** the kit as a mirrored,
> digest-verified blob, so the target still compiles nothing and installs nothing
> ([The Assembler Binary §3](/spec/assembler-binary.md)). The remaining risk is narrower and registered above:
> does *our* musl binary actually run everywhere (🚧, needs the L3 run).
---
