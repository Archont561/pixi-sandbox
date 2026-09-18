---
type: Measurement
title: "Docs Pipeline, Measured in the Airlock"
description: Probe log for the Starlight build: install size and timing, offline reproducibility, the four silent failures, link-validator counts.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, docs, measurement]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
verified: { by: process:sandbox-measurement, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["20"] }
stale_after: 2026-12-19T00:00:00Z
sources:
confidence-note: derived from the measurements and upstream reads cited in [Bibliography](/research/sources.md)
---

# Docs Pipeline, Measured in the Airlock

## 20. Docs-site pipeline, measured inside the airlock

Prompt: *"search also for Astro Starlight and GitHub Pages to add context about publishing pixi-sandbox docs."*
Because npm and GitHub are the two reachable package/artifact hosts here ✅, this was testable rather than
researchable — every line below is from a build run in this sandbox on 2026-09-19, not from a blog post.

**What works (✅ measured).** `npm i astro @astrojs/starlight astro-mermaid starlight-links-validator @astrojs/mdx`
all resolve from this box (227 MB + 143 pkgs); `astro build` renders 8 pages from *this repo's own markdown* in
4.6 s into 5.7 MB; the output has **zero** external `<script src>`/stylesheet/font references; Pagefind's binary
comes from the npm optional dep `@pagefind/linux-x64` ✅ (release-asset hosts are blocked ❌ — had it been a
download-on-first-run tool, offline search would have been fiction); re-running the build with the network
blackholed (`HTTP(S)_PROXY=http://127.0.0.1:9`, `npm_config_offline=true`) produced identical output; GitHub
Pages support is one action, `withastro/action` (`v6.1.3` = `3eafd002…` ✅ via API), whose documented inputs are
`path`, `node-version` (default 24), `package-manager` (auto-detected; `npm`/`pnpm`/`yarn`/`bun`/`deno` ✅ — so
our bun lockfile is fine), `build-cmd`, `cache` (default true), `out-dir` (default `dist`), plus
`permissions: { pages: write, id-token: write }` and `actions/deploy-pages` (`v5` = `368f8252…` ✅).

**What bit (✅ measured, all recorded in `lockfile-digest-map.md §8.2`).** (1) content without `title:` frontmatter fails the
schema; (2) Starlight 0.42's `docsLoader({ generateId })` no longer accepts `base`/`pattern`, and the
stale "glob the repo root" recipe yields an **empty collection with exit 0** — a silent one-page site; (3)
Astro 7 deprecates `markdown.remarkPlugins` (Sätteri is now the default renderer, `@astrojs/markdown-remark` not
installed by default) and — the sneaky one — setting `astro.config.markdown` next to Starlight kills *its*
pipeline too, so `:::caution` stopped rendering as `starlight-aside--caution` and `> [!IMPORTANT]` leaked as
literal text; the fix is to not set it and to convert alerts to asides in a sync step; (4) our markdown as
`.mdx` dies on `mdxjs-rs:raw-html` (our `<name>` placeholders), so `.md` stays; (5)
`starlight-links-validator` — which fails the build — reported 104 `invalid hash` findings for root-globbed
content, 19 `invalid link` after copying into the project, and **0** after the sync step rewrote
`](FILE.md#a)` → `](<base>/file/#a)`; incidentally it generated ids identical to our GitHub-slug convention
(`41-two-crates-one-binary` ✅), which retro-validates the anchor rule these docs use.

**What the sandbox itself contributed.** `*.github.io` ❌ `000` ⇒ **a Pages-only site is unreachable from an
airlock**, which is the argument for `pixi-sandbox docs publish` mirroring `dist/` into the `pixi-sandbox-docs`
orphan branch (5.7 MB ≈ 2 s at measured throughput ✅) and `reconstruct --with-docs` unpacking it to `.pixi/docs/`.
`docs.astro.build`/`starlight.astro.build` ❌ too, so all Astro/Starlight facts above came from the npm registry
metadata, the GitHub API, and doing it — not from docs pages. And `@astrojs/mermaid` (recommended by many blogs)
**does not exist**: registry 404 ✅; `astro-mermaid@2.1.0` and `rehype-mermaid@3.0.0` do ✅.

**Unresolved 🚧**: whether `rehype-mermaid` (build-time SVG, no client JS) is worth swapping in for
`astro-mermaid` (client render, ~1 MB of the 3.7 MB `_astro/`); whether `astro check` (needs `@astrojs/check` +
typescript) earns its install in CI; and whether `sitemap`/`robots` matter at all for a repo-internal docs site
(Starlight auto-adds `@astrojs/sitemap`, which warned `No pages found!` for collection-only content ✅).
