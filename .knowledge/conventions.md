---
type: Bundle Conventions
title: "Bundle conventions: labels, extensions, maintenance"
description: How this bundle encodes confidence, what its front-matter extensions mean, how links and legacy section numbers resolve, and how an agent should edit it.
resource: https://github.com/Archont561/pixi-sandbox
tags: [meta, conventions, trust]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
sources:
  - { id: okf-spec, resource: https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md, title: Open Knowledge Format v0.2 specification, author: team:googlecloud-data-analytics }
  - { id: repo, resource: https://github.com/Archont561/pixi-sandbox, title: Archont561/pixi-sandbox }
legacy: { files: [`DESIGN.md`], sections: ["preamble"] }
---

# Status of the project

**Design, not implementation.** No source files exist in this repository, deliberately: the sandbox this
corpus was authored in can reach GitHub, npm and PyPI but not crates.io, prefix.dev or conda channels, so any
Rust written there could not be compiled or tested, and `pixi` itself cannot be installed to validate a single
flag. The specification is finished enough to review, correct, and hand to a machine that *can* build it;
[Requirement Traceability](/overview/requirements.md) shows what each of the stated requirements was answered
with, and [What Is Still Unproven](/workflows/unproven.md) is the register of what nobody has executed yet.

# Labels → trust tiers

The corpus has always carried a three-value evidence vocabulary, and OKF's trust fields (§5.2, §5.3) are the
same idea formalised, so the two are kept in lockstep:

| In the prose | `confidence:` | OKF tier derived from `verified` | Meaning |
|---|---|---|---|
| **✅** | `verified` | `machine-confirmed` | read from upstream source, a registry response, or a command executed in this sandbox |
| **⚠️** | `reasoned` | `unverified` | a reasonable inference from verified facts; the gap is named in the sentence, not hidden |
| **🚧** | `open` | `unverified` | needs a real machine, a decision from you, or both — usually paired with `status: draft` semantics |
| mixed | `mixed` | as recorded | a section where some claims are measured and others are not |

`verified` is only set for content that a process or a human actually confirmed against its source. One
concept — [The Rust Toolchain Is a Conda Package](/research/conda-forge-rust.md) — is additionally
`human:archont561`-verified, because that fact came from you rather than from a probe. Everything else that is
`machine-confirmed` was confirmed by reading upstream code/registries or by running a command here.

# Front-matter extensions used here

`type` is the only required key (§4.1); this bundle also uses the recommended `title`, `description`,
`resource`, `tags`, plus the v0.2 trust/lifecycle/provenance families and two producer-defined keys:

- `confidence:` — the table above. Consumers MUST NOT reject documents for unknown keys, so ignoring it is legal.
- `legacy:` — `{ files, sections }`, the pre-OKF root document(s) and section numbers a concept was converted
  from. It exists so that older citations (`DESIGN §7.4`, "WORKFLOWS §4.5") stay resolvable, and so a reader can
  diff the bundle against the history of this repository's earlier drafts.

`stale_after:` is set on the research concepts that quote registry versions (pixi `0.81.0`, pixi-pack `0.7.11`,
`astro 7.3.3`, conda-forge `rust 1.98.1` ✅ at time of writing). Treat a stale research concept as a question to
re-run, not as a fact to correct by hand — the reproduction commands live in
[Reproducing These Measurements](/environment/reproducing.md).

# Links and section numbers

All cross-links are bundle-relative (`/area/concept.md`), the recommended form in §6.1, and anchors follow
GitHub's heading slug rule (lowercase, punctuation dropped, spaces → `-`; `+` and `—` therefore become double
hyphens). Broken links are tolerated by consumers by design — a link to a concept nobody has written yet is a
legitimate state of a bundle, and this one has a few in the 🚧 places.

Bare `§N` mentions inside a body refer to the section numbering **of the source document** the concept came
from, not to a heading in the current file. Since each concept preserves its source's heading text, the anchor
still resolves; `legacy.sections` names the numbering system. Do not renumber headings — re-anchoring fifty
documents by hand is not worth a tidier outline.

# Reserved names and the docs site

`index.md` and `log.md` are reserved (§3.1): they are navigation, not knowledge, so `docs-sync` skips them when
building the Starlight site ([Docs Site Design](/spec/docs-site.md)). Every directory here has an `index.md`
written for progressive disclosure: an agent that should not read the whole corpus can read the root index, pick
an area, and stop.

# Editing rules

1. One concept per file, and a concept is one *thing*: a decision, a measurement, a playbook, a section of the
   spec. Split rather than append when a file starts answering two questions.
2. Update `generated.at` whenever content changes; add a `verified` entry only when something actually
   confirmed it against a source (§5.2 — writing and confirming are different acts).
3. Record a new external dependency of a claim in `sources:` and attribute specific claims with footnotes keyed
   to those `sources` ids (§5.1). No credibility scores — signals only.
4. Add the concept to its area `index.md`, and add one dated line to the nearest `log.md` (§9).
5. Never fork prose into a second file. If the docs site needs a rewrite (title, callouts, routes), generate it
   in `docs-sync` — the bundle is the single source of truth.
6. Run the validator before committing: `python3 val.py` (the file lives at the **workspace root, outside the
   repo** — `../val.py` from here — because the bundle is markdown-only until
   [the decision log](/spec/decisions.md) is accepted). It checks front matter and its shapes, `type` presence,
   reserved-name usage, every internal link *and anchor*, reachability from an `index.md`, code-fence parity,
   `confidence` values, and stale references to the removed pre-OKF root documents. It runs with **zero
   dependencies** (no PyYAML: it carries its own parser for the two front-matter shapes above), which is why it
   works in a sandbox like this one ⚠️ and why a snapshot restore that loses it must simply re-create it from this
   list — the checks are part of the contract, not the file.
