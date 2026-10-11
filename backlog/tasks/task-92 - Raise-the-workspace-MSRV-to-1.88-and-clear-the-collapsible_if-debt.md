---
id: TASK-92
title: Raise the workspace MSRV to 1.88 and clear the collapsible_if debt it switches on
status: Done
assignee: []
created_date: '2026-10-11 12:17'
updated_date: '2026-10-11 17:15'
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
prints the exact collapse for every one. The sites sit in the ADR-13 per-file oracle, the manifest
reader and the sandbox-config validator — load-bearing modules — which is exactly why AGENTS.md
forbids folding the cleanup into the dependency change that exposed it.

The bump is only warranted while `self_update` is a dependency, and TASK-90's doc-14 verdict decides
that; hence the dependency above. A reject verdict closes this task as not-needed rather than
leaving it dangling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria

<!-- AC:BEGIN -->
- [x] #1 `rust-version` in the workspace `Cargo.toml` is the highest MSRV among normal dependencies (1.88 today) and the README badge states the same number; no other file in the tree contradicts either.
- [x] #2 All six `collapsible_if` sites above are collapsed with semantics unchanged, and `cargo clippy --workspace --all-targets -- -D warnings` is clean at the new MSRV.
- [x] #3 Every existing test passes at the same count or higher (`pixi run --frozen test`), and the invalid-input tests that reach the collapsed branches in `sandbox_config.rs` and `manifest.rs` still do — the error paths are covered, not just the happy path.
- [x] #4 One focused `refactor` commit after TASK-90 resolves; if the verdict is a rejection, the task is closed with that reason recorded instead of the bump being carried out.
<!-- AC:END -->

## Definition of Done

<!-- DOD:BEGIN -->
- [x] #1 pixi run --frozen fmt
- [x] #2 pixi run --frozen lint
- [x] #3 pixi run --frozen test
<!-- DOD:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Carried out after TASK-90's adopt verdict (ADR-27: `self_update` supplies the release
transport, our `ownership`/`replace` stay), which is what makes the bump warranted.

**AC#1.** `Cargo.toml:13` is `rust-version = "1.88"` — the highest MSRV among normal dependencies
(`self_update 1.3` and `rstest-bdd 0.6.0` both declare 1.88; nothing declares higher). Two places
the description did not name also had to agree: the docs-site badge
`docs/src/components/Badges.astro:15` carried the identical `Rust-1.85%2B` shield and now reads
1.88 alongside `README.md:7`, and the `self_update` comment block in `Cargo.toml` still narrated
the *pre-verdict* state ("Lifting `rust-version` is TASK-92 … A reject verdict removes the line")
— reworded to record ADR-27 and this lift. A `grep` over tracked sources finds no `1.85`
left: the remaining hits are history (`CONTEXT.md`, spike docs, this task's own description) and
third-party vendored metadata, none of which states the workspace's MSRV. The `rstest-bdd`
comment (`Cargo.toml:68`, "MSRV 1.88, below the workspace toolchain") was re-checked and stays
true as written.

**AC#2 — the debt was nineteen sites, not six.** The description's list came from a probe that
stopped compiling at the first failing crate: with `pixi-sandbox-core` red, nothing downstream
gets linted, so a `-p`-scoped enumeration under-reports. Collapsing the six let the workspace
build proceed and clippy surfaced the rest of the *same lint, same class* — all required by this
AC's workspace-clean bar, none of them unrelated cleanup. All nineteen collapses, clippy's
original line numbers:

- `pixi-sandbox-core` — the six above, exactly where listed.
- `pixi-sandbox` (lib): `tool_fetch.rs:67` (pin mismatch `bail!`), `upgrade/mod.rs:329` and
  `:334` (the step-summary/output writers).
- `pixi-sandbox` (bin): `commands/doctor.rs:240`, `:246`, `:257` (verify/restored/budget
  `bail!`s), `:265` (the host-requirements check — a three-level nest collapsed to one
  bool-and-let chain), `commands/restore.rs:153` (vendor install guard), `diagnostics.rs:110`
  (empty-detail guard).
- test targets: `pixi-sandbox/tests/e2e.rs:25` (text-file filter),
  `pixi-sandbox/tests/actions.rs:28` (BSD checksum line parse),
  `pixi-sandbox-core/tests/verify.rs:381` (three-level rewrite nest),
  `xtask/src/repo_checks/agents_md.rs:37` (repo-map resolution).

Every collapse is mechanical: the nested `if`'s condition joins the outer `if let` with `&&`
(parenthesised where the operand is a `||` chain, per clippy's own suggestion), nesting and
`continue`/`bail!`/`return` positions preserved, and every error string is byte-identical. No
`#[allow]` was added anywhere.

**AC#3.** `pixi run --frozen test` → **986 passing / 1 skipped** (42 git / 222 core / 526
pixi-sandbox / 196 xtask) — identical to the pre-change baseline, which is what a semantics-free
refactor must produce. The named invalid-input tests that reach the collapsed branches were also
run as a focused set (`cargo nextest run -p pixi-sandbox-core -E '…'`): 13 tests, 13 passed —
`a_blank_concurrency_group_is_rejected`, `a_zero_timeout_is_rejected`,
`a_whitespace_pixi_version_is_rejected`, `refused_libc_specs_name_the_accepted_forms` (5 cases),
`libc_parsing_never_panics_and_names_its_input`, `parsing_rejects_a_forged_or_malformed_list`,
`a_files_manifest_with_a_bad_path_entries_or_digest_is_refused`. One coverage nuance, recorded
rather than papered over: the `manifest.rs:253` site is *skip* logic, not an error path, so no
invalid-input test reaches it — its direct unit coverage is `blobs(None)`
(`tests/manifest.rs:71`), and the `Some`-filtered path is exercised black-box through `--envs`
in `tests/cli.rs`. Membership and ordering of `blobs()` output are unchanged.

**Gates.** `pixi run --frozen fmt` clean; `cargo clippy --workspace --all-targets --offline -- -D
warnings` clean at the new MSRV; `pixi run --frozen lint` 11/11; `pixi run --frozen test` 986/1;
`convco check` clean over the commit.

**Process note worth keeping.** Three `sandbox_config.rs` edits issued in parallel landed as one;
the workspace clippy gate caught it immediately (two sites still nested) and the edits were
redone sequentially. Same-file edits go one at a time; the gate that caught it is the one this
task turned on.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:SUMMARY:BEGIN -->
The workspace declares the MSRV it actually needs. `rust-version` is 1.88 — the highest among
normal dependencies, owed by `self_update 1.3` (adopted as ADR-27) and `rstest-bdd 0.6.0`
alike — and the README and docs badges say the same. The MSRV lift switched clippy's
`collapsible_if` on, and the debt it flagged is cleared in full: nineteen nested-`if` pairs across
ten files collapsed into let-chains, every error text byte-identical, suite count unchanged at
986 passing / 1 skipped. The description's six-site list was an under-count produced by a probe
that stopped at the first failing crate — the workspace-wide gate this task's own AC demands was
what found the rest, which is the argument for letting `--workspace --all-targets` and not a
per-crate probe define "done" for an MSRV chore.
<!-- SECTION:SUMMARY:END -->

## Comments

<!-- COMMENTS:BEGIN -->
created: 2026-10-11 12:17

2026-10-11 17:15 — done as one `refactor` commit after TASK-90 resolved with an adopt verdict;
the rejection branch of AC#4 did not apply.
<!-- COMMENTS:END -->
