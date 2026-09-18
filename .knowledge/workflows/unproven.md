---
type: Open Questions
title: "What Is Still Unproven"
description: The honest gap register: every claim in this design that is inferred, with the experiment that would settle it.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, uncertainty]
status: stable
confidence: open
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
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

---
