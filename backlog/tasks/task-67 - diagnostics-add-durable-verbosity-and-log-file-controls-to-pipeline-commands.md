---
id: TASK-67
title: 'diagnostics: add durable verbosity and log-file controls to pipeline commands'
status: Done
updated_date: '2026-10-04'
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
- [x] #1 pack, doctor, and publish accept one consistent documented diagnostics interface, including repeatable verbosity or equivalent levels and an explicit log-file destination
- [x] #2 The log captures phase boundaries, invoked helper identity, relevant non-secret paths, and the complete causal error chain while default output remains backward-compatible
- [x] #3 Secrets, tokens, authenticated remote URLs, and sensitive environment values are redacted or never recorded
- [x] #4 A log file is flushed on both success and failure and refuses unsafe destination behavior rather than silently losing the only diagnostic copy
- [x] #5 Black-box CLI tests cover accepted/rejected flags, success and nested failure logs, redaction, and isolated HOME/tempdir behavior
- [x] #6 CLI reference and airlock troubleshooting docs explain when to use verbosity versus a durable log file; formatting, lint, and the full suite pass
<!-- AC:END -->

## Implementation Notes
<!-- SECTION:NOTES:BEGIN -->
2026-10-04 — implemented test-first through the black-box CLI seam. The initial tests proved all three commands rejected the requested interface. `pack`, `doctor`, and `publish` now share `-v`/`-vv` plus `--log-file PATH`: one `-v` mirrors stable phase boundaries to stderr, two include reviewed context, and a log file records full detail regardless of verbosity while leaving ordinary stdout byte-identical.

The durable logger uses create-new semantics, flushes after every line, and turns an unwritable log into a command failure rather than claiming evidence exists. It records explicit fields instead of argv or environment dumps; configured remotes are represented as `<configured>`. A final redaction boundary strips URL userinfo and token/password query values from nested Git/process errors. Pack records validation, tool resolution and the resolved pixi-pack/pixi-unpack paths, assembly, and manifest writing; doctor records manifest loading, both verification modes, the standalone probe, and report rendering; publish records transport validation, manifest loading, GitProtocol publication, and local remote inspection.

Black-box tests cover help/flag availability, success and nested failure, default-output compatibility, `-vv`, create-new refusal with preservation of existing evidence, real Git error redaction, and isolated HOME/USERPROFILE/tempdirs. The CLI reference documents the shared contract and the airlock guide explains transferring the bounded log outside the transport. `pixi run --frozen lint`, the docs build, and the full suite pass: 619 passed / 1 skipped (612/1 before this task; 609/1 session baseline).
<!-- SECTION:NOTES:END -->

## Final Summary
<!-- SECTION:SUMMARY:BEGIN -->
Pipeline diagnostics no longer depend on the Actions log CDN. Operators can use `-v` or `-vv` interactively and can create a separately transferable `--log-file` containing phase boundaries, helper identity, safe local context, and the complete causal error chain. Logs are fail-closed, flush continuously, never overwrite earlier evidence, and redact credentials. Normal command output is unchanged when verbosity is not requested.
<!-- SECTION:SUMMARY:END -->
