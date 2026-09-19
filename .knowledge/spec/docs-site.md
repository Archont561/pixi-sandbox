---
type: Design Spec
title: "Docs Site Design"
description: The Starlight docs site as a design object: [docs] config, four verbs, the publish-twice rule, and the measured failure modes.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, docs]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`DESIGN.md`], sections: ["9.6"] }
sources:
  - { id: archont561githubio, resource: https://archont561.github.io", title: archont561.github.io" }
confidence-note: derived from the measurements and upstream reads cited in [Bibliography](/research/sources.md)
---

# Docs Site Design

### 9.6 Docs: a Starlight site, published twice — once for browsers, once for the airlock

The markdown in this repo *is* the deliverable while the tool is unbuilt, so "publishing" is a distribution
decision that belongs in the same machinery as everything else: content-addressed, digest-pinned, and reachable
without a CDN. `[docs]` (§6) is off by default and adds four verbs — `docs build | pack | publish | check` —
which reuse the kit's transport rather than inventing one.

```toml
# ── optional docs site (Astro Starlight); off by default ─────────────────────
[docs]
enabled = false
generator = "starlight"                      # the only one measured end-to-end here ✅
source-dir = ".knowledge"                     # the OKF bundle is the canonical markdown (post-OKF: root `*.md` are gone)
site-dir = "docs"                            # the Astro project (package.json, astro.config.mjs)
sync = ["copy", "frontmatter-title", "alerts-to-asides", "rewrite-links"]
site = "https://archont561.github.io"        # required for canonical URLs ✅
base = "/pixi-sandbox"                       # required for project pages ✅ (the guide writes it without a trailing slash)
codec = "gzip"                               # ⚠️ `zstd` is not a safe target assumption (absent here ✅)
mirror-branch = "pixi-sandbox-docs"          # the airlock-facing half
serve-task = "docs"                          # registered into the reconstructed workspace
```

| Concern | Decision | Grounding |
|---|---|---|
| Generator | Astro 7 + `@astrojs/starlight` 0.42 (asides, Pagefind search, expressive-code, `editLink`) | ✅ built in this sandbox: 227 MB deps in 33 s, 8 HTML pages in 4.6 s (5 docs + 2 probe copies + `404`), `dist/` 5.7 MB |
| Canonical source | the `.knowledge/` bundle's concept files; `docs-sync` copies + rewrites (title, alerts→asides, links→`base` routes) and **skips `index.md`/`log.md`** (OKF reserved names, §3.1) | ✅ measured: without the copy the site is empty-but-green; with it, 0 link findings |
| Canonical path is a **dot-directory** | every glob in `docs-sync` must opt into dotfiles (`dot: true` for fast-glob/picomatch) ⚠️ *inferred — the empty-collection failure mode is measured ✅, the glob default is not verified here* | otherwise `[docs].source` matches nothing and the site goes silently one-page, the exact hazard this section exists to gate |
| Diagrams | `astro-mermaid`, client-side bundle, **0** external refs | ✅ measured; the mirrored `*.md` fallback keeps mermaid fenced for non-JS readers |
| Search | Pagefind, index built at build time from the npm-shipped binary | ✅ `@pagefind/linux-x64` via npm — **not** a release asset (blocked ❌ here) |
| Browsers | GitHub Pages project page via `withastro/action` + `actions/deploy-pages`, pinned by SHA | ✅ documented inputs (`package-manager: bun@latest`, `out-dir: dist`, `cache: true`); both `site` and `base` required |
| Airlock | orphan branch `pixi-sandbox-docs` + `reconstruct --with-docs` → `.pixi/docs/` + a serving task | ✅ `*.github.io` unreachable from this box ❌, so a Pages-only site would serve exactly the readers who never need a kit; 5.7 MB ≈ 2 s at measured 2.8 MB/s ✅ |
| Gate | `docs check` = build + page-count assert + the site's own link validator (fails the build) | ✅ it produced 104 findings on this repo's docs before the sync step existed — the *only* checker that follows `FILE.md#anchor` across files |

Two things this section deliberately does **not** do: no SSR/edge deployment (`output: "static"` only — there is
nothing here to scale, and a server would be a second artifact class with no consumer ⚠️), and no second docs
stack — if Starlight's plugin churn (the Astro 7 `markdown` deprecation, §17 risk row) outweighs the benefit,
`enabled = false` leaves the mirrored markdown, which is the part the target actually reads ✅.

Recipe, CI YAML and the probe log: [Publishing the Docs §8](/workflows/publishing-docs.md#8-publishing-the-docs-astro-starlight--github-pages-and-the-airlock-mirror)
and [Docs Pipeline, Measured in the Airlock §20](/research/docs-pipeline.md#20-docs-site-pipeline-measured-inside-the-airlock).

---
