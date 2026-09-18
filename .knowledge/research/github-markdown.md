---
type: Source Notes
title: "GitHub enhanced Markdown"
description: The alert/admonition/table/mermaid subset GitHub renders natively, and what it does with front matter.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, markdown]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["8"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: macmdviewercom-blog-github-markdown-guide, resource: https://macmdviewer.com/blog/github-markdown-guide, title: macmdviewer.com/blog/github-markdown-guide }
  - { id: markdowneditoronlinecom-blog-gitlab-markdown, resource: https://markdowneditoronline.com/blog/gitlab-markdown, title: markdowneditoronline.com/blog/gitlab-markdown }
  - { id: blogmarkdowntoolscom-posts-markdown-for-github-compl, resource: https://blog.markdowntools.com/posts/markdown-for-github-complete-workflow-guide, title: blog.markdowntools.com/posts/markdown-for-github-complete-wo }
  - { id: stackoverflowcom-questions-50544499, resource: https://stackoverflow.com/questions/50544499/github-flavored-markdown-how-to-make-a-styled-admonition-box-in-a-gist, title: stackoverflow.com/questions/50544499/github-flavored-markdow }
  - { id: allmarkdowntoolscom-github-markdown, resource: https://allmarkdowntools.com/github-markdown, title: allmarkdowntools.com/github-markdown }
  - { id: allmarkdowntoolscom-github-markdown, resource: https://allmarkdowntools.com/github-markdown, title: allmarkdowntools.com/github-markdown }
---

# GitHub enhanced Markdown

## 8. GitHub enhanced Markdown

Two layers exist and people conflate them: **GFM**, a formal CommonMark superset with 5 extensions
(tables, task lists, strikethrough, autolinks, tag filtering), and **GitHub-proprietary renderer features**
— alerts, math, Mermaid, footnotes, emoji — which are **not in the GFM spec** and only work on GitHub
[1](https://macmdviewer.com/blog/github-markdown-guide).

| Feature | Syntax | Renders where | Gotcha |
|---|---|---|---|
| Alerts | `> [!NOTE]` / `[!TIP]` / `[!IMPORTANT]` / `[!WARNING]` / `[!CAUTION]` | README, issues, PR comments (⚠️ partial on wikis) | launched Dec 2023; **not** GFM; casing upper in GH docs, lower in GitLab [1](https://macmdviewer.com/blog/github-markdown-guide), [2](https://markdowneditoronline.com/blog/gitlab-markdown) |
| Tables | pipes + `:---:` alignment | everywhere | no merged cells — drop to `<table>` for `colspan` [1](https://macmdviewer.com/blog/github-markdown-guide) |
| Task lists | `- [ ]` / `- [x]` | README static; **interactive** in issues/PRs [1](https://macmdviewer.com/blog/github-markdown-guide) | — |
| Collapsible | `<details><summary>…</summary>` + blank line + markdown | README/issues/wikis [1](https://macmdviewer.com/blog/github-markdown-guide) | blank line after `</summary>` is required; `open` attr expands |
| Footnotes | `[^1]` … `[^1]: text` | README/issues [1](https://macmdviewer.com/blog/github-markdown-guide) | **not supported in wiki pages** |
| Mermaid | fence with info string `mermaid` | native since 2022 [4](https://blog.markdowntools.com/posts/markdown-for-github-complete-workflow-guide) | also GeoJSON/STL; GitLab adds PlantUML/Kroki [2](https://markdowneditoronline.com/blog/gitlab-markdown) |
| Math | `$…$`, `$$…$$`, or a `math`-info fence (MathJax on GH; KaTeX on GitLab) | everywhere [1](https://macmdviewer.com/blog/github-markdown-guide) | escape currency `\$` |
| Theme-aware images | `![x](img.svg#gh-light-mode-only)` / `#gh-dark-mode-only` | README files | ✅ both pixi and pixi-pack banners use it; inside pixi's *docs site* the equivalent fragments are `#only-light`/`#only-dark` (mkdocs) ✅ verified locally |
| Diff highlighting | fence with info string `diff`, lines prefixed `+`/`-` | everywhere | — |
| Autolinks | bare URL | issues/PRs | — |
| `@mentions`, `#123` refs | shortcuts | issues/PRs/comments, **not wikis** [4](https://blog.markdowntools.com/posts/markdown-for-github-complete-workflow-guide) | — |
| YAML front matter | `---` block | **hidden** in `.md` files (GitLab renders it as a box) [2](https://markdowneditoronline.com/blog/gitlab-markdown) | matters for OKF/`index.md` bundles |

**Sanitisation:** GFM's inline HTML is "aggressively sanitized", so styling hacks (colors, `<style>`)
don't work; the accepted fallbacks for emphasis are bold/italics or emoji (`:warning:`)
[5](https://stackoverflow.com/questions/50544499/github-flavored-markdown-how-to-make-a-styled-admonition-box-in-a-gist).
`<sub>/<sup>`, `<kbd>`, `<img align=…>` are in the allowlist
[3](https://allmarkdowntools.com/github-markdown).

> [!TIP]
> For a repo whose deliverable *is* documentation: put the **TL;DR in the first blockquote**, use a
> `<details>` block for raw transcripts, a **mermaid** block for architecture, and alerts for
> "must not do this" rules — they're the only visually distinct callouts that survive GitHub's sanitizer.
> Everything above this document already uses exactly that vocabulary.

---
