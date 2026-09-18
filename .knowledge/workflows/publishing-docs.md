---
type: Playbook
title: "Publishing the Docs"
description: Astro Starlight + GitHub Pages + the in-repo mirror: the config that works, the traps that fail silently, and CI YAML - all measured in this sandbox.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, docs, astro]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["8"] }
sources:
  - { id: examplecom, resource: https://example.com, title: example.com }
---

# Publishing the Docs

## 8. Publishing the docs: Astro Starlight + GitHub Pages (and the airlock mirror)

This repo *is* its documentation: 4 563 lines of markdown across five files that currently render only on GitHub. A site earns its
keep for one reason here — **searchable offline reading on the target** (Pagefind ✅, zero CDN assets ✅) — and a
second, unexpected one: it is the only full dependency build this sandbox can complete end to end, i.e. the
cheapest dogfood tier there is (§7.1 D1.5).

### 8.1 Measured in this sandbox, 2026-09-19

| Step | Result |
|---|---|
| `npm i astro@latest @astrojs/starlight@latest` | ✅ `astro 7.3.3` + `starlight 0.42.2`; **227 MB** `node_modules` in **33 s** |
| `npm i @astrojs/mdx astro-mermaid starlight-links-validator remark-github-blockquote-alert` | ✅ +143 packages in 12 s |
| `npx astro build` over this repo's markdown (then: 5 root docs + 2 deliberate probe copies — since OKF the source is `.knowledge/**`) | ✅ **4.6 s**, **8 HTML pages** (5 docs, 2 probe copies, `404.html`), `dist/` = **5.7 MB** (`_astro/` 3.7 MB, largest chunk 648 KB) |
| same build with the network blackholed (`HTTP(S)_PROXY=http://127.0.0.1:9`, `npm_config_offline=true`) | ✅ **identical output** ⇒ no network once `node_modules` exists — the airlock property that matters |
| external asset refs in the HTML (`<script src="https…">`, stylesheets, fonts) | **0** ✅ — only `<link rel="canonical">` pointing at our own `site`+`base` |
| search | ✅ `starlight:pagefind` indexed those 8 pages locally; the binary came from the npm optional dep `@pagefind/linux-x64` ✅, **not** a GitHub release asset (blocked ❌) — that single difference is what makes offline search real |
| mermaid | `astro-mermaid` logged `Sätteri transformed mermaid block in …/WORKFLOWS.md` per diagram and emitted `<pre class="mermaid">` + client JS ⇒ renders in-browser, **0** network refs; for a zero-JS site the build-time alternative is `rehype-mermaid@3.0.0` 🚧 (exists ✅, output not measured) |
| Node | astro 7 needs `node >=22.12.0` ✅ (box: v22.22.3 ✅) and `nodejs.org` is ❌ ⇒ **Node is a pin, not an install**; on a node-20 box the pair is `astro@5.18` + `starlight@0.37` (peer `^5.5.0`, engines `18.20.8 \|\| ^20.3.0 \|\| >=22.0.0` ✅) |
| `*.github.io`, `docs.astro.build`, `starlight.astro.build` from the airlocked box | ❌ `000` ⇒ **a Pages URL is unreachable in precisely the environment this project serves** (§8.5) |
| `@astrojs/sitemap` (auto-added by Starlight) | ⚠️ `No pages found! sitemap-index.xml not created` ✅ for a collection-only site — harmless, but check it if you expect a sitemap |
| packing the result | ⚠️ **`zstd` is absent on this box** ⇒ the default codec for `docs pack` must be gzip unless the *build* env adds it (`pixi add zstd`) |

### 8.2 Five things that broke before it worked — each one cheap to avoid

1. **`title:` frontmatter is required.** Root markdown synced as-is died with
   `docs → `problem.md` data does not match collection schema: title: Required` ✅. Derive it in the sync step (first
   `# ` heading), don't edit 5 files by hand.
2. **`docsLoader({ base, pattern })` is gone.** 0.42's signature is `docsLoader({ generateId })` ✅, and passing
   `base` did **not** error: it logged `The collection "docs" does not exist or is empty` and built a one-page
   site with **exit 0** ✅. Every "Making the Docs Globbable" guide is stale, and silent-empty is the worst
   failure mode a docs pipeline can have ⇒ the build must assert a page count (§8.3, `docs-check`).
3. **Never set `astro.config.markdown` next to Starlight.** Costliest probe of the session, because it fails
   silently: with `markdown: unified({ remarkPlugins: [githubAlerts] })` the build succeeded, `> [!IMPORTANT]`
   leaked as literal text ✅, **and Starlight's own `:::caution` stopped rendering too** ✅ — the override
   replaces Starlight's markdown pipeline, so every callout in the site degrades to a plain blockquote. Remove
   the key and native asides return: `class="starlight-aside starlight-aside--caution"` with `<p>Caution</p>` ✅.
   If a custom markdown block is unavoidable, the form that keeps both worlds is
   `markdown: unified({ gfm: true, directives: true, remarkPlugins: [ … ] })` ✅ measured — `directives: true` is
   what asides depend on, and *even then* `remark-github-blockquote-alert@2.1.0` left `[!IMPORTANT]` literal ✅, so
   the reliable route is conversion, not configuration.
   Astro 7 invites the mistake: it deprecates `markdown.remarkPlugins` and notes that
   `@astrojs/markdown-remark` "is no longer installed by default now that Sätteri is the default Markdown
   processor" ✅. **Consequence for us:** don't wire a GitHub-alert plugin at all — alerts are a *GitHub
   renderer* feature, so the sync step maps `> [!X]` onto native asides, and no plugin churn is involved.
4. **Do not rename the docs to `.mdx`.** The same content with an `.mdx` extension died with
   `Cannot compile a raw node (raw HTML) to MDX/JSX output … (mdxjs-rs:raw-html)` ✅, triggered by our `<name>`
   placeholders and `<br/>` inside mermaid labels. Keep `.md`; add `@astrojs/mdx` only for hand-written
   interactive pages.
5. **Our anchor convention is portable; the link *plumbing* is not.** The site generated exactly the ids our
   links already reference (`4-architecture`, `41-two-crates-one-binary`, `#14-corrections--method-notes` ✅ —
   dots dropped, em-dash/`↔` → `--`), so no anchor text changes. What breaks is path shape, and
   `starlight-links-validator` (which **fails the build** ✅) cleared it in two measured steps:

   | Probe state | Findings |
   |---|---|
   | markdown globbed from the **repo root** (`glob({ base: '..' })`) | **104** × `invalid hash` — the validator can't map sources living outside the Astro project ✅ |
   | same files **copied into** `docs/src/content/docs/` | **19** × `invalid link`, hashes resolving now ✅ |
   | + rewrite `](FILE.md#a)` → `](<base>/file/#a)` in sync | **0** — `· All internal links are valid. ·` ✅ |

   So `docs-sync` has two jobs, not one: copy *into* the project, and rewrite cross-file links into the site's
   route space **including the `base` prefix** (that third line is the whole fix ✅). And the fact that our
   hashes validated cleanly afterwards is the useful result: the slug rule these 5 files use is not
   GitHub-only, so `val.py` and Starlight's validator can coexist — ours runs with zero deps, theirs is the
   gate in CI.

> [!WARNING]
> **A dot-directory means dotfiles must be opted into.** Once the canonical markdown is `.knowledge/**/*.md`, the
> default glob semantics (`dot: false` in fast-glob/picomatch, plus the usual `**/.*` ignore patterns) match
> **nothing** ⚠️ *inferred*, which reproduces the silent-empty failure above with a brand-new trigger. Two
> defences, both cheap: pass `dot: true` in `docs-sync`, and keep the page-count assertion in `docs check` so an
> empty collection can never be green again.

### 8.3 Layout: one source of truth, two renderers

```
pixi-sandbox/
├── README.md                   # the only root markdown: a pointer into the bundle
├── .knowledge/                 # canonical: OKF v0.2 concepts (index.md + log.md reserved, never published)
└── docs/                       # Astro project (~7 files), never hand-edited
    ├── astro.config.mjs        # site + base + starlight + astro-mermaid + links-validator — NO `markdown` key (§8.2)
    ├── src/content.config.ts   # docsLoader() + docsSchema(); content is COPIED in, not globbed
    └── package.json, bun.lock  # → one more ecosystem row in §0's table
```

`docs-sync` is a script (≈40 lines, tested by "build + validator ⇒ 0 findings" ✅) because four rewrites happen
together: copy `.knowledge/**/*.md` (skipping `index.md`/`log.md`, whose OKF role is navigation) →
`docs/src/content/docs/`; lift `title:` from front matter (OKF has it already ✅); map alerts to asides
(`NOTE→note`, `TIP→tip`, `IMPORTANT→caution`, `WARNING→danger`, `CAUTION→danger`); rewrite
`](FILE.md#a)` → `](/pixi-sandbox/file/#a)`. Doing it at build time instead of in the prose is the point: the
markdown stays idiomatic GitHub-flavoured *and* the site renders natively.

```toml
# pixi.toml
[tasks]
docs-sync  = { cmd = "bun scripts/docs-sync.ts", inputs = [".knowledge/**/*.md"] }
docs-build = { depends-on = ["docs-sync"], cmd = "cd docs && bun install --frozen-lockfile && bunx astro build" }
docs-check = { depends-on = ["docs-build"], cmd = "cd docs && test $(find dist -name '*.html' | wc -l) -ge $(ls ../*.md | wc -l)" }
docs-serve = { depends-on = ["docs-build"], cmd = "cd docs && bunx serve dist" }        # local reading, 0 CDN refs ✅
```

> [!TIP]
> `bun` is first-class here: `withastro/action` auto-detects the package manager from the lockfile and accepts
> `package-manager: bun@latest` (npm/pnpm/yarn/deno too, versions allowed ✅), `node-version` defaults to 24 ✅,
> `out-dir` to `dist` ✅, and `cache = true` (Astro build cache under `node_modules/.astro` ✅) which fits the
> 10 GB cache budget in `problem.md §14`. Since the docs build is the one thing this box *can* run, `pixi run
> docs-check` is the natural first CI job to adopt — it exercises lockfile-pinned installs, a real build, and
> artifact publishing with the same three verbs a kit uses.

### 8.4 Publish: the Pages half (for readers with a browser *and* a network)

```yaml
# .github/workflows/docs.yml — action SHAs resolved via api.github.com on 2026-09-19 ✅
name: Docs
on:
  push: { branches: [main], paths: ["**.md", "docs/**", "bun.lock"] }
  workflow_dispatch:
permissions: { contents: read, pages: write, id-token: write }   # pages + id-token are required to deploy ✅
concurrency: { group: pages, cancel-in-progress: true }
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@<sha>                                     # repo convention: pin by SHA
      - uses: withastro/action@3eafd002e65cc31b4f0eae0bb05450d521562247  # v6.1.3 ✅ builds + uploads dist ✅
        with: { path: docs, package-manager: bun@latest }
  deploy:
    needs: build
    runs-on: ubuntu-latest
    environment: { name: github-pages, url: ${{ steps.deployment.outputs.page_url } }
---
    steps:
      - { id: deployment, uses: actions/deploy-pages@368f82528645a54fb793d4d04e342629a3f51346 }   # v5 ✅
```

Three project-page requirements, each easy to miss: set **both** `site: 'https://<owner>.github.io'` and
`base: '/pixi-sandbox'` (either alone 404s every asset ✅ — and §8.2 item 5 shows the *link* layer has to know the
base too; the guide writes `base` **without** a trailing slash ✅); Settings → Pages → **Source: GitHub Actions**
— this workflow deploys, so **no `gh-pages` branch exists** and §7.2's branch hygiene stays intact ✅; and
`public/.nojekyll` is *optional* now — the current Astro guide no longer asks for it (it exists to protect the
older `gh-pages`-branch + Jekyll path), and shipping the empty file is harmless insurance if that method is ever
resumed ✅ (measured: it lands at `dist/.nojekyll`). `withastro/action`'s README is the canonical workflow
above ✅, with an official `withastro/github-pages` template repo if a fresh start is easier.

> [!WARNING]
> A custom domain is **two** changes, not one: add `public/CNAME`, point `site` at `https://example.com`, **and
> remove `base`** — the guide then says "update all your page internal links to remove the `base` prefix" ✅. For
> this repo that means `docs-sync` must read `base` from `[docs]` (single source of truth) rather than
> hardcoding `/pixi-sandbox/`, or the clean 0-findings state of §8.2 quietly regresses to 19. `site` must be one
> of the two accepted forms (`https://<user>.github.io`, or the `<random>.pages.github.io` of a private org
> page) ✅ — inventing a third yields mis-absolute canonical URLs.

| Where it runs | Command | Why there |
|---|---|---|
| `lint` job (every PR) | `pixi run docs-check` | a broken internal link becomes a build failure — it found 104 here before the sync step existed ✅ |
| `docs.yml` (main) | `withastro/action` → `deploy-pages` | Pages serves people off-network; the `paths:` filter keeps it off the 2 000 min/month budget ✅ |
| `dist` job (existing) | `pixi-sandbox docs pack && docs publish` | the only route that reaches an airlock (§8.5) |

### 8.5 The airlock half: the site as a build product, not a URL

Because `*.github.io` is unreachable from an airlock ❌, Pages alone would serve exactly the readers who never
need a kit. So the same `dist/` travels by the transport this project already has:

| Piece | Mechanism | Cost, measured |
|---|---|---|
| `docs build` | `astro build`, here (D1.5 ✅) or in CI | 4.6 s + install |
| `docs pack` | `tar -czf` of `docs/dist` (5.7 MB; gzip while `zstd` is missing here ⚠️) | ~2 s over git at 2.8 MB/s ✅ |
| `docs publish` | orphan branch `pixi-sandbox-docs`, digest in `dist-manifest.json` — the same disposable-blob pattern as `-dist`/`-vendor` (`problem.md §10.1`) | one force-push |
| `reconstruct --with-docs` | extract to `.pixi/docs/`, register a `docs` task that serves it ✅ | zero downloads |

The loop closes in a way worth saying out loud: **docs arrive on the target through the same mechanism as the
environments, and stay searchable offline** ✅. On a box with no browser, the mirrored `*.md` sources are in the
kit anyway and read fine in a pager ⚠️ (search degrades to `grep`) — an honest floor, not a gap. And the
`docs-*` verbs are the *safe* place to try kit machinery first: a 5.7 MB artifact, no conda, no cargo, and a
build this sandbox can already reproduce.

```bash
pixi-sandbox docs build     # sync → install (only if the lockfile changed) → astro build
pixi-sandbox docs pack      # tar.gz + sha256 into sandbox.lock.json
pixi-sandbox docs publish   # → pixi-sandbox-docs orphan branch, digest recorded
pixi-sandbox docs check     # build + links validator (the 104→19→0 audit) as a gate
```

`[docs] enabled = false` is the default (`problem.md §6`): a team that decides "GitHub renders our markdown fine"
pays nothing for any of this, and `pixi-sandbox explain docs` says so rather than leaving the section a mystery.

---

If you only remember one line from this file: **`pixi-pack` packs environments, not your crate graph — so
"can we skip `cargo vendor`?" is answered by whether the airlocked box compiles, and the answer is usually
"ship the built binary, skip the vendor".**
