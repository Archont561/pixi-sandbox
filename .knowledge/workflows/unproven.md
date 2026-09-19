---
type: Open Questions
title: "What Is Still Unproven"
description: The honest gap register: every claim in this design that is inferred, with the experiment that would settle it.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, uncertainty]
status: stable
confidence: open
generated: { by: arena-agent/agent-mode, at: 2026-09-20T00:15:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["6"] }
---

# What Is Still Unproven

## 6. What is still unproven

| Claim here | Status | How it gets settled |
|---|---|---|
| `file://` local channels are supported by pixi | ✅ documented (manifest form + `pixi project channel add file:///…`) | nothing pending for *support*; M4.5 measures the *composition* with a frozen install |
| A pack's `channel/` + `repodata.json` is directly consumable by pixi as that channel | ⚠️ plausible: pixi-pack calls it "a local channel named `pixi-unpack`" ✅ but shows `conda`/`micromamba` consumers, not pixi | 5-line probe (already in `research/rev2-discovery.md §15.3`) run in CI on a networked box; the difference between "verified" and "documented" is exactly what ⚠️ means |
| `pixi build` + `pixi-build-rust` can build offline from `vendor/` | ⚠️ inferred from documented `cargo install` + `extra-input-globs`, never tried | `reconstruct-e2e` variant with `egress: off` |
| `repodata-config.disable-sharded` is needed for a tiny local channel | 🚧 | one run; if unnecessary, the generated config drops it |
| tar-only unpack leaves prefixes unrelocated | ⚠️ inferred (rattler's `Prefix::install` does the work in the other rungs ✅) | `grep -r /opt/conda .local-env/bin` after each rung — it is a 10-second experiment |
| `--offline` is respected by build backends | ✅ **documented as NOT respected** | none — the design already avoids source deps on targets |
| How to attach remark/rehype plugins to content-collection markdown in Astro 7 | ⚠️ `markdown: unified({…})` is accepted but had **no effect**, and setting `markdown` at all broke Starlight's own pipeline ✅ | avoided by design (`docs-sync` rewrites alerts → native asides); settle it only if a plugin becomes unavoidable, and then via the Sätteri/config-reference page |
| `tar -xf` (R5) on a pack whose env contains `rust` | 🚧 plausible: the recipe sets `binary_relocation: false` because *"the distributed binaries are already relocatable"* ✅ | compile *and* link a hello-world from an R5 prefix — the delta between the two results **is** the finding (`sysroot_*`/`gcc_impl_*` are prefix-relative ⚠️) |
| `rehype-mermaid` (build-time SVG, no client JS) vs `astro-mermaid` (client-side, ~1 MB of the 3.7 MB `_astro/`) | 🚧 | build the site once with each and diff `dist/`; matters only if a target reads docs without JS |
| `.nojekyll` still needed for `withastro/action` deploys | ⚠️ the current Astro Pages guide no longer mentions it ✅ (it protected the older `gh-pages`-branch method) | keep shipping the empty file (harmless ✅) and drop it if a Pages deploy proves it irrelevant |
| The action's **CI half** (`pack.sh` + `publish.sh`) against real `pixi-pack` and `cargo vendor` | 🚧 inferred — the script *decisions* are measured (nine fixtures ✅) but with stubbed drivers, so the tarball shape and vendor tree are still upstream's word, not ours | one M1.5 run on a real runner |
| Under [D21](/spec/decisions.md): the Rust assembler **compiles at all**, and reproduces the nine `assemble.sh` oracle vectors | 🚧 never compiled — this sandbox has no `rustc` route ✅, so every Rust claim is CI-shaped until a runner builds it | `cargo test` (L1/L2) + the L3 clean-container one-liner in [Testing Strategy](/spec/testing-strategy.md); the oracle vectors are the acceptance gate |
| Under [D21](/spec/decisions.md): a musl-static `pixi-sandbox` binary **runs on any Linux with zero runtime deps** (alpine included) | ⚠️ inferred from pixi-pack's own musl release assets ✅ — same shape, but our binary has never been linked | the L3 alpine-container reconstruction; a segfault/`not found` there falsifies the musl assumption |
| `assemble.ps1` walks the same ladder on Windows | 🚧 never executed — no PowerShell in this sandbox ✅ (absent) | a `windows-latest` job reconstructing a fixture kit; until then Windows kits are R5-only by policy |
| A pack built on `ubuntu-latest` unpacks on *any* `linux-64` target | ⚠️ assumed: the same-platform rule is documented, glibc/sysroot drift between runners and targets is not | reconstruct on `alpine` and a debian-oldstable container in one CI run |

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
