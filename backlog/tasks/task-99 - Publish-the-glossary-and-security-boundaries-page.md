---
id: TASK-99
title: Publish the glossary and security boundaries page
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 08:45'
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
- [x] #1 Every term in the audit vocabulary list is defined once on the page, each linking to the page that carries the full detail.
- [x] #2 The page states the security boundaries explicitly: what is verified, what the transport guarantees, and what the tool does not provide.
- [x] #3 Restore, configuration, and CLI pages link to the glossary instead of re-defining terms inline.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: glossary.mdx published under Overview and wired into the sidebar. Nineteen terms defined in one table, each linking the page with the full detail; a 'guarantees stated once' section (byte integrity, check order, failed-run-changes-nothing, no credentials, budgets-before-push) linking verification-guarantees; and a four-item 'does not provide' strip (no signature chain, no host provisioning, no toolchain, no inert payload). AC#3 satisfied by glossary links added to restore.mdx, reference/cli.mdx, and reference/configuration.mdx intros; the CLI reference pages keep the link when task-102 splits them. Build 21 pages zero warnings; biome 12 files green; link-check ledger now also counts the four forward links to the task-102 CLI pages the glossary names.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The vocabulary has one canonical home: nineteen defined terms each linking their detail page, the guarantees stated once, and the four explicit non-guarantees — linked from restore, the CLI reference, and the configuration reference.
<!-- SECTION:FINAL_SUMMARY:END -->
