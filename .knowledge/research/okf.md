---
type: Source Notes
title: "Open Knowledge Format"
description: The spec this bundle implements: v0.2 bundle shape, required front matter, trust families, reserved names, conformance rules.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, okf, meta]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["11"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: wwwgitbookcom-blog-what-is-okf-open-knowledge-format, resource: https://www.gitbook.com/blog/what-is-okf-open-knowledge-format, title: www.gitbook.com/blog/what-is-okf-open-knowledge-format }
  - { id: witscodecom-open-knowledge-format, resource: https://witscode.com/open-knowledge-format, title: witscode.com/open-knowledge-format }
  - { id: explainxai-blog-google-open-knowledge-format-okf-ai-, resource: https://explainx.ai/blog/google-open-knowledge-format-okf-ai-agents-2026, title: explainx.ai/blog/google-open-knowledge-format-okf-ai-agents- }
  - { id: document360com-blog-open-knowledge-format, resource: https://document360.com/blog/open-knowledge-format/, title: document360.com/blog/open-knowledge-format/ }
  - { id: wwwtryvizupcom-blog-open-knowledge-format-structurin, resource: https://www.tryvizup.com/blog/open-knowledge-format-structuring-knowledge-for-ai-agents, title: www.tryvizup.com/blog/open-knowledge-format-structuring-know }
  - { id: githubcom-googlecloudplatform-knowledge-catalog, resource: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md, title: GoogleCloudPlatform/knowledge-catalog — blob/main/okf/SPEC.md }
  - { id: githubcom-googlecloudplatform-knowledge-catalog, resource: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md, title: GoogleCloudPlatform/knowledge-catalog — blob/main/okf/SPEC.md }
---

# Open Knowledge Format

## 11. Open Knowledge Format (OKF)

*(the "Google open knowledge base format" in the request.)*

**What.** OKF is an **open, vendor-neutral specification from Google Cloud** — a directory of markdown
files with YAML front matter, cross-linked into a graph an AI agent can traverse deliberately instead of
doing similarity search. Published **v0.1 on 2026-06-12/13** by the Data Cloud / Data Analytics team
(Sam McVeety, Amir Hormati) in the public `GoogleCloudPlatform/knowledge-catalog` repo at `okf/SPEC.md`;
**current spec is v0.2** [1](https://www.gitbook.com/blog/what-is-okf-open-knowledge-format),
[2](https://witscode.com/open-knowledge-format), [3](https://explainx.ai/blog/google-open-knowledge-format-okf-ai-agents-2026),
[4](https://document360.com/blog/open-knowledge-format/). It is described as formalising Andrej Karpathy's
"LLM wiki" idea — a markdown knowledge base an agent reads and maintains like code
[2](https://witscode.com/open-knowledge-format), [5](https://www.tryvizup.com/blog/open-knowledge-format-structuring-knowledge-for-ai-agents).
*(Not the Open Knowledge Foundation — unrelated name collision
[2](https://witscode.com/open-knowledge-format).)*

**Bundle shape — read from the spec itself:**

```
path/to/bundle/
  index.md          # Optional. Directory listing for progressive disclosure.
  log.md            # Optional. Chronological history of updates.
  <concept>.md      # a concept at the bundle root
  <subdirectory>/   # groups concepts
    index.md
    <concept>.md
```
A **bundle** is the unit of distribution; a **concept** is one markdown file, and its **Concept ID is the
file path with `.md` removed** — i.e. *the path is the identity*. Distribute as **a git repo
(recommended: history, attribution, diffs)**, a **tarball/zip**, or a subdirectory of a bigger repo.
`index.md` and `log.md` are **reserved** names at any level and MUST NOT be concept files
[SPEC §2–§3](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md).

**Concept document = front matter + body.** `type` is **the only always-required key**, and a concept
carrying only `type` is fully conformant; recommended `title`, `description`, `resource` (canonical URI of
the underlying asset), `tags`; recommended-example type values `BigQuery Table`, `BigQuery Dataset`,
`API Endpoint`, `Metric`, `Playbook`, `Reference`, `Attested Computation`; **type values are not centrally
registered**, and consumers MUST tolerate unknown types and unknown extra keys
[SPEC §4.1](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md).
Body is free markdown, but structure is preferred over prose, with **conventional headings `# Schema`,
`# Examples`, `# Computation`** [SPEC §4.2].

**v0.2's contribution: trustworthiness metadata.** The spec's stated motivation is that corpora are now
*continuously written and maintained by agents*, so a consumer needs five answers front-and-center:
provenance ("what was this created from, and how was it verified?"), trust ("how much should I trust
it?"), freshness ("is it still true?"), lifecycle ("is it the current version?"), and attestation ("was
this number produced the way we said it must be?") [SPEC §1]. Concretely:

| Family | Front matter | Notes |
|---|---|---|
| Provenance | `sources: [{id, resource, title, author, usage_count, last_modified}]`, `usage_window` | `resource` is **required within an entry**; OKF records **credibility signals, not a verdict**; per-claim attribution uses **markdown footnotes keyed to `sources` entries** [SPEC §5.1] |
| Generation | `generated: { by: reference_agent/gemini-2.5-pro \| human:ahormati \| process:ci, at: 2026-06-30T14:00:00Z }` | **Actor convention** `<producer>/<version>` / `human:<id>` / `process:<id>`; **every OKF timestamp is ISO 8601 with explicit UTC offset** (an Aug-2026 commit made this mandatory) [SPEC §2, §5] |
| Trust / lifecycle | `verified`, lifecycle fields | derived **trust tiers**: unverified / machine-confirmed / human-reviewed; absence never causes rejection [SPEC §2, §5.3, §11] |
| Computation | `type: Attested Computation` + `# Computation` + `executor` / `receipt` / attester | an **executor** runs a sanctioned computation and returns a **receipt** (not stored in the bundle); an **attester** is deterministic, *no-LLM* code that inspects the receipt and returns a verdict [SPEC §2, §10] |

**Explicit non-goals:** no fixed taxonomy of concept types, no prescribed storage/serving/query
infrastructure, not a replacement for Avro/Protobuf/OpenAPI (OKF *references* them), and no packaging
standard for the code an executor points at — "OKF fixes the interface, not the packaging"
[SPEC §1]. "No schema registry, no central authority, no required tooling. If you can `cat` a file, you
can read OKF; if you can `git clone` a repo, you can ship it." [SPEC preamble]

**Positioning:**

| Format | Scope | Reader |
|---|---|---|
| **OKF** | a whole knowledge base of typed, linked concepts | any agent/tool, across orgs |
| `llms.txt` | one pointer file at a site root | crawlers/LLMs ("a pointer, not the content") |
| `AGENTS.md` / `CLAUDE.md` | one repo's instructions | the repo's coding agent |

[2](https://witscode.com/open-knowledge-format). vs RAG: OKF "represents relationships between concepts"
as a traversable graph, is human+AI readable, and needs no embeddings; many stacks run both — OKF for
relational context, retrieval for the long tail
[1](https://www.gitbook.com/blog/what-is-okf-open-knowledge-format). Google shipped a **reference producer**
(a BigQuery enrichment agent that drafts concept docs for tables/views) and a **reference consumer** (a
single-file static HTML visualizer, no backend), plus three sample bundles (GA4 e-commerce, Stack
Overflow, Bitcoin datasets) [3](https://explainx.ai/blog/google-open-knowledge-format-okf-ai-agents-2026),
[5](https://www.tryvizup.com/blog/open-knowledge-format-structuring-knowledge-for-ai-agents).
The repo currently sits at ~9.2k stars / 784 forks ✅ verified locally via the GitHub API.

---
