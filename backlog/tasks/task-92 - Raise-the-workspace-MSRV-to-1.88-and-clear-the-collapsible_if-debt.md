---
id: TASK-92
title: Raise the workspace MSRV to 1.88 and clear the collapsible_if debt it switches on
status: To Do
assignee: []
created_date: '2026-10-11 12:17'
labels:
  - msrv
  - lint
  - refactor
dependencies:
  - TASK-90
references:
  - Cargo.toml
  - README.md
  - crates/pixi-sandbox-core/src/files_manifest.rs
  - crates/pixi-sandbox-core/src/host_requirements.rs
  - crates/pixi-sandbox-core/src/manifest.rs
  - crates/pixi-sandbox-core/src/sandbox_config.rs
priority: low
type: chore
ordinal: 92000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`self_update 1.3` declares `rust-version = "1.88"`, so the workspace's declared 1.85 no longer
describes what building `pixi-sandbox` needs: a toolchain between 1.85 and 1.87 fails inside cargo
on the dependency's rustc requirement, not inside our code. The declaration stayed at 1.85 when the
crate landed (PR #135) because raising it is not a one-line change — `clippy` reads `rust-version`
and switches on its MSRV-gated lints, and `collapsible_if` then fires on six sites in
`pixi-sandbox-core` that `-D warnings` turns into build failures:

- `files_manifest.rs:158` — the digest check inside `if let Some(digest) = &entry.h`
- `host_requirements.rs:170` — the `LibcFloor::parse` check inside `if let Some(spec) = &self.libc`
- `manifest.rs:253` — the `only` filter in blob selection
- `sandbox_config.rs:633` — the `concurrency.group` validation
- `sandbox_config.rs:647` — the `timeouts` minutes bound
- `sandbox_config.rs:657` — the `pixi_version` validation

Each is the let-chain form the 2024 edition and Rust 1.88 admit (`if let … && … {}`), and clippy
prints the exact collapse for every one. The sites sit in the D13 per-file oracle, the manifest
reader and the sandbox-config validator — load-bearing modules — which is exactly why AGENTS.md
forbids folding the cleanup into the dependency change that exposed it.

The bump is only warranted while `self_update` is a dependency, and TASK-90's doc-14 verdict decides
that; hence the dependency above. A reject verdict closes this task as not-needed rather than
leaving it dangling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [ ] #1 `rust-version` in the workspace `Cargo.toml` is the highest MSRV among normal dependencies (1.88 today) and the README badge states the same number; no other file in the tree contradicts either.
- [ ] #2 All six `collapsible_if` sites above are collapsed with semantics unchanged, and `cargo clippy --workspace --all-targets -- -D warnings` is clean at the new MSRV.
- [ ] #3 Every existing test passes at the same count or higher (`pixi run --frozen test`), and the invalid-input tests that reach the collapsed branches in `sandbox_config.rs` and `manifest.rs` still do — the error paths are covered, not just the happy path.
- [ ] #4 One focused `refactor` commit after TASK-90 resolves; if the verdict is a rejection, the task is closed with that reason recorded instead of the bump being carried out.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [ ] #1 pixi run --frozen fmt
- [ ] #2 pixi run --frozen lint
- [ ] #3 pixi run --frozen test
<!-- DOD:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 12:17
<!-- COMMENTS:END -->
