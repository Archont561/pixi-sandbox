---
id: doc-12
title: 'Source Audit: Pedantic Lint Debt and the Long-Function Remainder'
type: specification
created_date: '2026-10-06 20:22'
---

# Source Audit: Pedantic Lint Debt and the Long-Function Remainder

## Status

**Audit only — no code changed.** This document records a measured scan of the four Rust crates
performed with `.agents/skills/refactor/SKILL.md`'s identify-then-fix process, stopping after
"identify". It exists to close the one hole its predecessor named and never filled, and to hand a
successor task a list of findings that a tool can re-derive rather than a list of opinions.

The successor task is **task-77**. Everything in "Sequencing" below is proposed, not done.

## Why this audit exists

`backlog/docs/plans/repo-wide-refactor-dry-kiss-solid/doc-9` (Complete; tasks 55–61 and 64) says
this about its own method, at line 30:

> No `cargo clippy --all-targets -- -D warnings` pedantic pass was run as part of this scan (the
> default lint set is already clean per `AGENTS.md`); running `clippy::pedantic` once is listed as
> a cheap follow-up in **Workstream 4**.

There is no Workstream 4 in doc-9. The string appears exactly once in the file — in the sentence
promising it. The pedantic pass was deferred to a section that was never written, and so was never
run. This audit runs it.

That matters more now than it did on 2026-10-03, because the tree grew **+3,425 source lines in the
three days since** doc-9 measured it (16,734 → 20,159), all of it under a gate that cannot see any
of the findings below.

## Method (reproducible)

```sh
cargo clippy --workspace --all-targets --message-format=json \
  -- -W clippy::pedantic -W clippy::nursery
```

Raw output: 883 diagnostics. `--all-targets` compiles the `pixi-sandbox` library and binary
separately, so every finding in a shared module is reported twice; de-duplicating on
`(lint, file, line, column)` gives **469 unique findings, 435 of them under `src/`**. All counts in
this document are the de-duplicated ones.

Supporting measurements, each a separate script over the same tree: per-file and per-crate line
counts; a normalized 8-line sliding-window hash for duplicate detection (comments and blank lines
stripped, `#[cfg(test)]` modules excluded, minimum 160 characters to avoid matching boilerplate);
and a `#[cfg(test)]`-stripping pass before counting `unwrap`/`expect`/`panic!`.

Baseline to protect: **`pixi run --frozen test` → 654 passed, 1 skipped** (2026-10-06, commit `db2d446`), up
from the 556/1 doc-9 finished at. Per the session skill's rule, no task drawn from this audit may
lower that number.

## Scope boundaries — what this audit deliberately does not own

1. **`generated/github_workflow.rs::render_github_workflow` (476 lines) and
   `generated/relock_workflow.rs::render_relock_workflow` (347 lines)** are the first and second
   largest functions in the workspace — 823 lines, 39% of all long-function debt. They are
   **already owned by task-76**, which moves the shell they emit into real subcommands and will
   shrink both renderers as a side effect. Re-cutting them here would double-book the work and
   create a merge conflict with a task that has a harder acceptance bar. **Excluded.**
2. **doc-9's Part A1 (the `Platform` type) and A2 (the `repo_checks` split) are done and verified
   standing**: `crates/pixi-sandbox-core/src/platform.rs` exists (165 lines) and
   `crates/xtask/src/repo_checks/` is 13 files. Not re-proposed.
3. **doc-9's non-goals carry forward unchanged**: no new crate dependencies (the vendored tree
   cannot fetch a proc-macro crate without a full relock cycle), no schema/transport/git-access
   changes, no hand-edits to rendered workflow YAML.

---

## Finding 1 — the gate cannot see any of this (root cause)

| Where a lint policy could live | Current contents |
|---|---|
| `Cargo.toml` `[workspace.lints]` | **absent** |
| `crates/pixi-sandbox-core/Cargo.toml` `[lints]` | absent |
| `crates/pixi-sandbox-git/Cargo.toml` `[lints]` | absent |
| `crates/pixi-sandbox/Cargo.toml` `[lints]` | absent |
| `crates/xtask/Cargo.toml` `[lints]` | absent |
| each crate's `package.json` `lint` script | `cargo clippy -p <crate> --all-targets -- -D warnings` |

The gate is four crate-scoped clippy runs at the **default** lint level. `-D warnings` makes the
default set non-negotiable, which is why the tree is clean against it — and also why 469 findings
accumulated invisibly. This is the single highest-leverage item in the audit: it is not "fix 469
things", it is "decide the policy once, then the gate holds the line for free."

There is no evidence of a deliberate decision to stay at the default level. `.knowledge/decisions.md`
(D1–D15) contains no lint-level decision, and `AGENTS.md`'s Style section asks only for "clippy clean
with `-D warnings`". The absence looks like an omission, not a choice — but a task acting on this
audit should confirm that rather than assume it, and record the answer as a decision either way.

## Finding 2 — 469 unique findings, and 261 of them are machine-applicable

| Applicability | Count | What it means for sequencing |
|---|---|---|
| `MachineApplicable` | **261** | `cargo clippy --fix` applies these; review is reading a diff, not writing one |
| `None` (no suggestion) | 158 | docs and structure — genuine human work |
| `MaybeIncorrect` | 39 | suggestion exists but needs judgement |
| `Unspecified` / `HasPlaceholders` | 11 | case by case |

By crate:

| Crate | Unique findings | Src lines | Density |
|---|---|---|---|
| `pixi-sandbox` | 188 | 9,928 | 1 per 53 lines |
| `pixi-sandbox-core` | 182 | 3,440 | **1 per 19 lines** |
| `pixi-sandbox-git` | 65 | 1,260 | 1 per 19 lines |
| `xtask` | 34 | 5,508 | 1 per 162 lines |

The density inversion is the interesting part. `xtask` — the crate doc-9 called its "single biggest
gap" — is now **the cleanest by a factor of eight**, because tasks 56 and 61 rebuilt it. The debt
has moved into the two library crates that carry the public API surface.

Top lints (unique):

| Count | Lint | Character |
|---|---|---|
| 77 | `must_use_candidate` | public API hygiene |
| 74 | `use_self` | mechanical |
| 58 | `missing_errors_doc` | doc debt on `Result`-returning public functions |
| 28 | `too_long_first_doc_paragraph` | doc shape |
| 23 | `map_unwrap_or` | mechanical |
| 18 | `missing_const_for_fn` | mechanical |
| 16 | `redundant_pub_crate` | visibility hygiene |
| 16 | `doc_markdown` | doc shape |
| 14 | `redundant_closure_for_method_calls` | mechanical |
| **13** | **`too_many_lines`** | **structural — see Finding 3** |
| 12 | `missing_panics_doc` | doc debt |
| 11 | `option_if_let_else` | nested-conditional smell |
| 10 | `literal_string_with_formatting_args` | **false positives — see Finding 6** |
| 9 | `cast_possible_truncation` | **correctness-adjacent — see Finding 4** |
| 8 | `cast_precision_loss` | correctness-adjacent |
| 8 | `needless_pass_by_value` | avoidable clones |
| 7 | `items_after_statements` | readability |
| 7 | `return_self_not_must_use` | API hygiene |

Top files under `src/`:

| Count | File |
|---|---|
| 48 | `pixi-sandbox-core/src/platform.rs` |
| 46 | `pixi-sandbox-core/src/verify.rs` |
| 44 | `pixi-sandbox-git/src/shell.rs` |
| 18 | `pixi-sandbox/src/user_tools.rs` |
| 17 | `pixi-sandbox/src/commands/support.rs` |
| 16 | `pixi-sandbox-core/src/files_manifest.rs` |
| 16 | `pixi-sandbox-core/src/shard.rs` |
| 15 | `pixi-sandbox/src/standalone.rs` |

`platform.rs` topping the list is worth naming out loud: **the newest module in the workspace, created
by doc-9's own flagship refactor (task-55), carries the highest pedantic debt in it** — 48 findings in
165 lines. Not because it is bad code, but because it is a fresh pure-data API, and a fresh pure-data
API is precisely the shape `must_use_candidate`, `use_self` and `missing_errors_doc` fire on. This is
evidence for the policy fix in Finding 1 rather than against task-55: a gate that saw these would have
caught them at authoring time, for free, instead of letting them land and be rediscovered three days
later by an audit.

## Finding 3 — 13 long functions, 5 of which are doc-9's unexecuted remainder

`clippy::too_many_lines` (threshold 100), de-duplicated and sorted:

| Lines | Function | Status |
|---|---|---|
| 476 | `generated/github_workflow.rs::render_github_workflow` | **task-76** — excluded here |
| 347 | `generated/relock_workflow.rs::render_relock_workflow` | **task-76** — excluded here |
| 192 | `pixi-sandbox-core/src/verify.rs::verify_env_restored` | new; not in doc-9 |
| 182 | `commands/pack.rs::assemble_artifacts` | new; the helper task-57 extracted *into* |
| 179 | `commands/doctor.rs::print_human` | **doc-9 A3, never executed** |
| 142 | `commands/pack.rs::write_branch_docs` | new |
| 137 | `tests/e2e.rs::fixture_doctor_publish_and_restore_is_the_complete_offline_proof` | test |
| 121 | `commands/restore.rs::register_user_tools` | **doc-9 A3, never executed** |
| 112 | `xtask/src/workflow.rs::lint_generated_workflow` | new |
| 111 | `commands/doctor.rs::run` | **doc-9 A3, never executed** |
| 110 | `commands/restore.rs::run` | **doc-9 A3, never executed** |
| 101 | `xtask/src/main.rs::run_args` | dispatcher — see below |
| 101 | `commands/tools/update.rs::refresh` | **doc-9 A3, never executed** |

Two independent methods agreeing is the strongest signal in this audit. doc-9 listed
`restore::run`, `restore::register_user_tools`, `doctor::print_human`, `doctor::as_json` and
`tools/update.rs` by hand on 2026-10-03 using a brace-depth heuristic. Task-57 then executed
**only `pack::run`** — and `pack::run` is correspondingly the one entry that has dropped off the
list. Clippy, knowing nothing about doc-9, independently flags exactly the five that were left
behind. doc-9's A3 is not "done"; it is 1/6 done, and this is the receipt.

Two entries deserve a caveat rather than a refactor:

- **`xtask/src/main.rs::run_args` (101)** is a `match` over every subcommand. It is one line over
  threshold and splitting a dispatcher makes it worse, not better. Allow it in place.
- **`tests/e2e.rs::...` (137)** is a single end-to-end narrative test. Its length *is* the point;
  the repo's own testing guidance values one readable offline proof over six fragmented ones.
  Allow it in place.

`commands/tools/update.rs` carries a second, distinct smell doc-9 also flagged (A4) and nobody
acted on: **413 of its 763 lines (54%) are an inline `#[cfg(test)]` module**, in a crate whose
established convention is a `tests/self_update_*.rs` sibling per concern. Workspace-wide, inline
test modules are 4,445 of 20,202 src lines (22%), so this file is at nearly 2.5× the house rate.

## Finding 4 — the one correctness-adjacent cluster

Nine `cast_possible_truncation` and three `cast_sign_loss`. Most are provably safe (ELF header
fields with compile-time-constant values, a test assertion). Three are not obviously safe, and all
three sit in the same place — **the arithmetic that decides whether a transport fits**:

| Site | Expression |
|---|---|
| `pixi-sandbox-core/src/shard.rs:122` | `std::cmp::min(COPY_BUFFER as u64, limit - written) as usize` |
| `pixi-sandbox-core/src/transport_budget.rs:232` | `(value * MIB as f64).round() as u64` |
| `pixi-sandbox/src/commands/pack.rs:523` | `Ok(bytes as u64)` |

These are flagged as *worth reading*, not as known bugs. `shard.rs:122` is the shard copy loop;
`transport_budget.rs:232` converts a float MiB budget to bytes; both feed the budget checks that
`doctor --verify --budget-config` enforces. A silent truncation here would not crash — it would
produce a wrong size verdict, which is the quietest possible failure mode in this codebase. An hour
of reading three expressions is cheap insurance; if all three are safe, a comment saying *why* is
the deliverable.

## Finding 5 — what is already healthy (do not touch)

Audits that only list problems are misleading about where a codebase stands. Measured and clean:

| Check | Result |
|---|---|
| `clippy::too_many_arguments` | **0** |
| `clippy::cognitive_complexity` (nursery) | **0** |
| `clippy::type_complexity` | **0** |
| `clippy::large_enum_variant` | **0** |
| `clippy::struct_excessive_bools` / `fn_params_excessive_bools` | **0** |
| `clippy::module_name_repetitions` | **0** |
| Duplicated 8-line blocks (normalized, ≥160 chars) | **18 distinct / 36 occurrences** in 20,159 lines |
| `unwrap`/`expect`/`panic!` outside `#[cfg(test)]` | **65** — `AGENTS.md`'s rule holds |

Zero `cognitive_complexity` findings alongside 13 `too_many_lines` findings is a precise and
flattering diagnosis: **the long functions here are long because they are long sequences, not
because they are tangled.** They are flat pipelines of named steps, which is the easy case for
Extract Method and the reason this refactor is low-risk. Nothing in this codebase needs
untangling; some of it needs paragraph breaks.

Likewise 18 duplicate blocks across 20k lines means doc-9's DRY work held. The remaining
clusters are in `pixi-sandbox-git/src/shell.rs` (9) and the two workflow renderers (8, task-76's
territory) — the former being, per `AGENTS.md`, deliberately "all git access in one place".

## Finding 6 — false positives a naive `-D clippy::pedantic` would churn

All 10 `literal_string_with_formatting_args` hits are **intentional** and must not be "fixed":

- `tests/actions.rs`, `commands/tools/update.rs:182`, `repo_checks/workflow_shape.rs:235` use
  `.replace("{version}", …)` / `"{target}"` / `"{block}"` — placeholder templates, not format
  strings.
- `commands/init.rs:669,731` and `tests/cli.rs:67,74` assert on **shell** parameter expansion
  (`${PIXI_SANDBOX_USER_TOOLS:-register}`, `${PREFIX:-sandbox}`), which is Bash syntax inside a
  Rust string literal.

A task that enables pedantic wholesale without an `allow` for this lint will either churn these
into unreadable escapes or, worse, "fix" a shell default-value expansion into a Rust format
argument. This is called out here so the successor task can budget an `allow` with a reason
comment instead of discovering it mid-refactor.

---

## Sequencing (proposed; one focused conventional commit each)

1. **Decide and encode the lint policy** (Finding 1). Add `[workspace.lints.clippy]` with
   `pedantic` and the deliberate `allow`s (at minimum `literal_string_with_formatting_args`, each
   with a reason), have all four crates inherit it, and record the choice in
   `.knowledge/decisions.md` as a new D-number. **Do this first**: it is what makes every later item
   permanent rather than cosmetic.
2. **Sweep the 261 machine-applicable findings** with `cargo clippy --fix`, reviewed as a diff, in
   per-crate commits. No behaviour change; tests must be identical before and after.
3. **Pay the doc debt** — 58 `missing_errors_doc`, 28 `too_long_first_doc_paragraph`, 12
   `missing_panics_doc`, 16 `doc_markdown`. Mostly `pixi-sandbox-core`'s public API.
4. **Finish doc-9's A3**: `doctor::print_human`, `doctor::run`, `restore::run`,
   `restore::register_user_tools`, `tools::update::refresh`. Extract Method only, one function per
   commit, allowing `main.rs::run_args` and `e2e.rs`'s narrative test in place.
5. **Read the three cast sites** (Finding 4); fix or comment, with a test if a bound is now explicit.
6. **Move `commands/tools/update.rs`'s 413 inline test lines** to a `tests/` sibling, matching the
   crate's own `self_update_*.rs` convention.

Items 1–2 are the cheap, high-leverage half. Items 4–6 are the half that needs judgement.

## Acceptance strategy (inherited from doc-9, unchanged)

1. `pixi run --frozen test` ≥ 654 passed, 1 skipped before *and* after every commit.
2. `pixi run --frozen lint` stays clean, including the newly-raised bar.
3. One focused conventional commit per step; `convco` clean.
4. No test added or changed points at this repository or a real `HOME` (D10).
5. Nothing here touches a rendered workflow, the manifest schema, the transport format, or
   git-access strategy.
