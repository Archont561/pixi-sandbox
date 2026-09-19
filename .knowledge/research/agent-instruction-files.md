---
type: Source Notes
title: "AGENTS.md and CONTEXT.md conventions"
description: How agent-facing instruction files are laid out, sized and scoped, and how they relate to an OKF bundle.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, agents]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["9"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: prompt-architectscom-blog-311-the-agents-md-standard, resource: https://prompt-architects.com/blog/311-the-agents-md-standard-explained, title: prompt-architects.com/blog/311-the-agents-md-standard-explai }
  - { id: wwwmorphllmcom-agents-md-guide, resource: https://www.morphllm.com/agents-md-guide, title: www.morphllm.com/agents-md-guide }
  - { id: docsfactoryai-cli-configuration, resource: https://docs.factory.ai/cli/configuration/agents-md, title: docs.factory.ai/cli/configuration/agents-md }
  - { id: wwwscriptbyaicom-agents-md-guide, resource: https://www.scriptbyai.com/agents-md-guide, title: www.scriptbyai.com/agents-md-guide }
  - { id: agent-readydev-how-to-write-an-effective-agents-md, resource: https://agent-ready.dev/how-to-write-an-effective-agents-md, title: agent-ready.dev/how-to-write-an-effective-agents-md }
  - { id: githubcom-orgs-community, resource: https://github.com/orgs/community/discussions/191257, title: orgs/community — discussions/191257 }
  - { id: amitraycom-claude-md-vs-agents-md-memory-md-skills-m, resource: https://amitray.com/claude-md-vs-agents-md-memory-md-skills-md-context-md-guide-2026/, title: amitray.com/claude-md-vs-agents-md-memory-md-skills-md-conte }
  - { id: mcgarrahorg-ai-agent-context-files-in-practice-, resource: https://mcgarrah.org/ai-agent-context-files-in-practice/, title: mcgarrah.org/ai-agent-context-files-in-practice/ }
  - { id: mcgarrahorg-ai-agent-context-files-in-practice-, resource: https://mcgarrah.org/ai-agent-context-files-in-practice/, title: mcgarrah.org/ai-agent-context-files-in-practice/ }
---

# AGENTS.md and CONTEXT.md conventions

## 9. `AGENTS.md` and `CONTEXT.md` conventions

### 9.1 AGENTS.md

*It is a **filename convention**, not a spec.* agents.md calls it "a simple, open format for guiding
coding agents", and its FAQ: "No. `AGENTS.md` is just standard Markdown. Use any headings you like; the
agent simply parses the text you provide." As of 2026-08-28 the project publishes **no spec, no schema, no
version tags**, and it is now a project of the **Agentic AI Foundation (Linux Foundation)**
[1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained).

**Who reads it:** OpenAI Codex, Cursor, Zed, Warp, opencode, Amp, Factory, GitHub Copilot's cloud agent +
code review, VS Code, Google Jules — each vendor's own docs are the authority, and the site's compatibility
wall overstates things [1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained).
**Claude Code does not read it** (it reads `CLAUDE.md`); documented bridges: a `CLAUDE.md` containing
`@AGENTS.md`, or `ln -s AGENTS.md CLAUDE.md` [1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained).
Use **all-caps** `AGENTS.md` (Warp requires it; other tools accept only that)
[1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained).

**Mechanics worth encoding in a repo:**
* Nested files: **nearest file wins**; Codex concatenates root→cwd and stops at `project_doc_max_bytes`
  = **32 KiB** default; Cursor needs `.cursor/rules/*.mdc` (with `globs`/`alwaysApply`) for conditional
  loading while `AGENTS.md` is the plain alternative; Amp loads subtree files lazily on read; Zed is
  single-file [1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained),
  [5](https://www.morphllm.com/agents-md-guide). Codex also supports `AGENTS.override.md`
  [5](https://www.morphllm.com/agents-md-guide).
* **It is context, not configuration** — "instructions are treated as context, not enforced configuration,
  with no guarantee of strict compliance. If a rule must hold every time, enforce it in CI, a pre-commit
  hook, or your agent's hook system, and use the file to explain *why*" [1](https://prompt-architects.com/blog/311-the-agents-md-standard-explained).
* Recommended sections (community consensus, not spec): overview, exact build/test/lint commands,
  repository map, conventions, testing rules, generated-file policy, security/secret boundaries, PR
  expectations, definition-of-done [3](https://docs.factory.ai/cli/configuration/agents-md),
  [2](https://www.scriptbyai.com/agents-md-guide), [4](https://agent-ready.dev/how-to-write-an-effective-agents-md).
  Write **verifiable** instructions; avoid taste words ("clean", "elegant"); keep versions only when they
  change implementation choices [4](https://agent-ready.dev/how-to-write-an-effective-agents-md).

### 9.2 CONTEXT.md (and the file hierarchy)

`CONTEXT.md` is an emerging, looser convention with **two audiences**: newcomers and the agent. Its content
is *what the codebase **is*** — architecture layers and data flow, domain model (entities, PKs, FKs,
cascade rules, often an ASCII ERD), annotated file paths, and explicit conventions ("thin controllers;
`snake_case` columns vs `camelCase` TS models; always parameterized queries"), treated as a **living
document** [3](https://github.com/orgs/community/discussions/191257).

Layering as commonly described [1](https://amitray.com/claude-md-vs-agents-md-memory-md-skills-md-context-md-guide-2026/):

| File | Role | Update cadence | Priority |
|---|---|---|---|
| `CLAUDE.md` | tool identity/behaviour | rarely | highest (Claude) |
| `AGENTS.md` | architecture + conventions (open, cross-tool) | rarely | high |
| `CONTEXT.md` | **present**: session state, current goals, tribal knowledge | rewritten per session (20–40 lines ideal) | session-scoped |
| `MEMORY.md` | **past**: append-only decision log | per session (append) | reference |
| `SKILLS.md` / `SKILL.md` | task-specific expertise, loaded on demand | when a skill evolves | task-scoped |

and the split with GitHub's own files: `copilot-instructions.md` = *how the assistant should behave*,
`instructions/*.instructions.md` = path-scoped rules, `skills/*/SKILL.md` = reusable workflows,
`CONTEXT.md` = **the foundational layer** [3](https://github.com/orgs/community/discussions/191257).

**The pragmatic multi-agent pattern actually used in repos:** one canonical `AGENTS.md`, and every other
tool's file reduced to a **one-line pointer** ("Read `AGENTS.md` for all project context.") — because
"OpenAI Codex auto-discovers `AGENTS.md` by convention, which means one fewer stub file", and the name is
self-documenting [2](https://mcgarrah.org/ai-agent-context-files-in-practice/). Measured reliability of the
pointer trick: Claude Code follows references reliably; Kiro's `#[[file:AGENTS.md]]` *injects* content
(best case); **Copilot reads its instructions file but does not follow file references** (it only picks up
`AGENTS.md` via workspace indexing); Gemini's CLI follows pointers [2](https://mcgarrah.org/ai-agent-context-files-in-practice/).
The conclusion in the field: "context *persistence* is the real problem, not context format" and "there is
still no cross-agent standard — if the ecosystem converges elsewhere, migration is renaming one file"
[2](https://mcgarrah.org/ai-agent-context-files-in-practice/).

> [!NOTE]
> **Relationship to OKF (§11):** `AGENTS.md`/`CLAUDE.md` are per-repo *instruction* files; OKF is a
> whole **knowledge base** format (typed, cross-linked concept files). They're complementary, and an OKF
> bundle can simply live inside a repo that also has an `AGENTS.md`
> [2](https://witscode.com/open-knowledge-format), [3](https://explainx.ai/blog/google-open-knowledge-format-okf-ai-agents-2026).

---
