---
id: TASK-99
title: Publish the glossary and security boundaries page
status: To Do
assignee: []
created_date: '2026-10-12 07:56'
labels:
  - docs
  - glossary
  - security
dependencies: []
references:
  - docs/src/content/docs/restore.mdx
  - docs/src/content/docs/reference/cli.mdx
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: high
type: docs
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The docs lean on a shared vocabulary — connected host, airlock, transport, bundle, orphan/sandbox branch, pack, doctor, publish, restore, unpack, host requirements, tools lock/embedded binary, shard and blob budget — but define it only contextually. Create the canonical definitions page: concise definitions, what each guarantee means, what the tool does not provide, and links to host requirements, budgets, and restore guidance. Modeled on how Pixi separates onboarding from concepts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every term in the audit vocabulary list is defined once on the page, each linking to the page that carries the full detail.
- [ ] #2 The page states the security boundaries explicitly: what is verified, what the transport guarantees, and what the tool does not provide.
- [ ] #3 Restore, configuration, and CLI pages link to the glossary instead of re-defining terms inline.
<!-- AC:END -->
