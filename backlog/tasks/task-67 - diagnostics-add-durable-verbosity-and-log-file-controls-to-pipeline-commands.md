---
id: TASK-67
title: 'diagnostics: add durable verbosity and log-file controls to pipeline commands'
status: To Do
assignee: []
created_date: '2026-10-04 13:19'
labels:
  - cli
  - diagnostics
  - airlock
dependencies: []
references:
  - 'https://github.com/Archont561/pixi-sandbox/issues/93'
  - crates/pixi-sandbox/src/cli.rs
  - crates/pixi-sandbox/src/commands/pack.rs
  - crates/pixi-sandbox/src/commands/doctor.rs
  - crates/pixi-sandbox/src/commands/publish.rs
priority: high
type: enhancement
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Issue #93 reports that pack, doctor, and publish failures cannot be diagnosed when an airlocked operator cannot reach the GitHub Actions log CDN. Define one consistent CLI diagnostics contract for the pipeline: useful verbosity levels and a durable log file that captures command context and the causal error chain without changing normal output or leaking credentials.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 pack, doctor, and publish accept one consistent documented diagnostics interface, including repeatable verbosity or equivalent levels and an explicit log-file destination
- [ ] #2 The log captures phase boundaries, invoked helper identity, relevant non-secret paths, and the complete causal error chain while default output remains backward-compatible
- [ ] #3 Secrets, tokens, authenticated remote URLs, and sensitive environment values are redacted or never recorded
- [ ] #4 A log file is flushed on both success and failure and refuses unsafe destination behavior rather than silently losing the only diagnostic copy
- [ ] #5 Black-box CLI tests cover accepted/rejected flags, success and nested failure logs, redaction, and isolated HOME/tempdir behavior
- [ ] #6 CLI reference and airlock troubleshooting docs explain when to use verbosity versus a durable log file; formatting, lint, and the full suite pass
<!-- AC:END -->
