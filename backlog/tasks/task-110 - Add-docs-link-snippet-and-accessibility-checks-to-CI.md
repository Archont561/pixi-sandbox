---
id: TASK-110
title: 'Add docs link, snippet, and accessibility checks to CI'
status: Done
assignee: []
created_date: '2026-10-12 07:56'
updated_date: '2026-10-12 10:00'
labels:
  - docs
  - ci
  - accessibility
  - testing
dependencies:
  - TASK-96
references:
  - .github/workflows/docs.yml
  - docs/src/components/
documentation:
  - >-
    backlog/docs/plans/docs-refactor/doc-18 -
    Documentation-Refactor-Roles-Lifecycle-and-Reference-Split.md
priority: low
type: docs
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
The audit accessibility findings are desktop-only observations, not a WCAG audit — validate before and after the refactor. Wire checks into the docs workflow: internal link integrity after the re-homing, command-snippet tests where practical, keyboard operation of navigation/search/theme/copy buttons, visible focus in both themes, contrast of badges/diagram labels/callouts, and responsive behavior of the transport diagram and wide tables.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 An internal link check runs in docs.yml and fails on a dead link after the IA re-home.
- [x] #2 Command snippets are tested where practical — at minimum the commands the first-bundle guide tells users to run.
- [x] #3 Keyboard/focus/contrast/responsive checks are documented and run, covering nav groups, search, theme control, copy buttons, badges, diagram labels, and callouts in both themes.
- [x] #4 Print/no-script usability of the operator checklist is verified.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-10-12: four bun scripts under scripts/, each a pixi task, each wired into .github/workflows/docs.yml between `pixi run docs build` and the Pages upload — the same gate locally and in CI, no node_modules and no browser required. AC#1 — docs-link-check.mjs walks every built HTML file and verifies each internal link and fragment resolves inside the same build (base-prefixed, relative, and same-page forms; the favicon defect it exposed is fixed by shipping docs/public/favicon.svg). It is green at 2 122 links / 28 pages / 0 failures. AC#2 — docs-snippet-check.mjs extracts every ```bash fence from the 28 MDX pages, splits them into logical commands, and for each `pixi-sandbox …` invocation walks the subcommand chain against the real CLI's `--help` output, expanding clustered shorts (-vv) and skipping clap builtins: 63 commands and 115 flags verified. It builds the CLI with `cargo build -q -p pixi-sandbox` when no binary exists (the default pixi environment carries the Rust toolchain); validated locally against a stub derived from crates/pixi-sandbox/src/cli.rs, which also confirmed the docs' command surface matches the source. AC#3 — docs-a11y-check.mjs runs structure (one h1, no skipped heading levels, html lang, alt text incl. the img-alt-as-link-name rule, accessible names on every link and button — verified for search, cancel, menu, and the copy buttons — table headers, skip link), :focus-visible rules, the print stylesheet, the sandbox-diagram collapse rule, overflow-x for wide tables, the operator checklist's script-free markup (AC#4), and WCAG contrast for the site's color pairs in BOTH themes (light tokens merged over the base map so var() chains resolve). The gate found four real issues, fixed in the preceding commit: 52 decorative <Icon> call sites lacked aria-hidden (astro-icon does not add it), white-on-#10b981 (2.54:1) and white-on-#3b82f6 (3.68:1) moved to #047857/#1d4ed8, stage captions moved off gray-3-on-gray-6 (4.49:1 dark) to gray-2, and the missing favicon. The browser-dependent half is documented as a protocol in project/repository.mdx (keyboard walk through nav groups/search/theme/copy with visible focus in both themes; 375/768/1280/1440 responsive checks; print and no-JS passes of the restore page), with the honest caveat that the homepage badge strip is third-party shields.io imagery. AC#4 — the checker asserts the checklist region carries no <script> and no client island, holds 7 CSS-drawn (not glyph) tick boxes, avoids page breaks when printed, and links its detailed sections; the manual protocol adds the print and JS-disabled passes. Also committed: docs-frontmatter-check.mjs (the scratch YAML-colon guard made permanent), biome.json now includes scripts/**/*.mjs and lint-docs checks `docs scripts`. Build 28 pages; all four gates exit 0; biome 21 files clean.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
The docs workflow now runs four quality gates before publishing — frontmatter, internal links (2 122 checked, 0 failures), accessibility/print/no-script/contrast (both themes), and command snippets (63 commands, 115 flags) — each reproducible locally as a pixi task; the gates found and fixed four real defects (decorative icons without aria-hidden, two sub-4.5:1 color pairs, gray-3 captions on gray-6, and the missing favicon), and the browser-only checks are documented as a manual protocol.
<!-- SECTION:FINAL_SUMMARY:END -->
