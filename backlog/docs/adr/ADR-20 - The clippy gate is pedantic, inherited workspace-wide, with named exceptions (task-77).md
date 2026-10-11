---
id: ADR-20
title: "The clippy gate is pedantic, inherited workspace-wide, with named exceptions (task-77)"
status: accepted
tags:
  - decision
---

# ADR-20 — The clippy gate is pedantic, inherited workspace-wide, with named exceptions (task-77)

**Decision.** The lint gate is `clippy::pedantic` at `warn`, declared once in
`[workspace.lints.clippy]` and inherited by all four crates through `[lints] workspace = true`;
the per-crate lint scripts' `-D warnings` makes the level binding. `clippy::nursery` stays
**off**. Every deliberate exception carries a reason: workspace-wide, only
`literal_string_with_formatting_args` is allowed (doc-12 Finding 6: every hit is an
intentional placeholder template like `.replace("{version}", …)` or a shell parameter
expansion like `${PREFIX:-sandbox}` asserted inside a Rust string literal — "fixing" them
would corrupt the string being asserted); every other exception is per-item with a recorded
reason (function-scoped `too_many_lines` allows naming their owner, per-item documentation
allows where the doc would be noise), never a blanket suppression.

**Why.** The gate used to run clippy at the default level, which is why 637 pedantic
findings accumulated invisibly (doc-12 measured 469 on 2026-10-06; the tree grew 3,425+
lines since doc-9, all under that blind gate). The findings are unusually tractable — the
majority are `use_self`, `must_use_candidate`, `missing_errors_doc` and mechanical rewrites,
and `cargo clippy --fix` applies the machine-applicable share — so the debt is a gate
problem before it is a code problem: decide the policy once and the gate holds the line for
free. Pedantic without nursery is the reviewed line: nursery is unstable and opinionated
(`significant_drop_tightening` and friends fire on code that is already clear), and the one
nursery signal worth having — `cognitive_complexity` — measured **zero** workspace-wide, so
enabling the group would buy noise, not signal.

**Evidence.** The 2026-10-09 refresh of doc-12's scan (`cargo clippy --workspace
--all-targets -- -W clippy::pedantic -W clippy::nursery`, de-duplicated on
`(lint, file, line, column)`): 637 unique findings, 435+ under `src/` — per crate
pixi-sandbox-core 239, pixi-sandbox 228, xtask 100, pixi-sandbox-git 69. doc-12's top lints
(469 findings) plus the growth from TASK-82/87. `clippy::too_many_arguments`,
`clippy::type_complexity`, `clippy::large_enum_variant` and `clippy::cognitive_complexity`
all measure zero, which is what makes "raise the gate, then clear the debt in reviewed
per-crate commits" a bounded task rather than a rewrite. After the sweep,
`pixi run --frozen lint` passes on all four crates at the raised bar with the 924/1 suite
green before and after every commit.

**What would change it.** A pedantic lint that proves to be a systematic false positive
across this codebase (not one-off cases) graduates to a workspace-level allow with a measured
reason; a nursery lint that earns its keep (a real tangle it catches) is enabled
individually rather than by group; a crate that genuinely cannot meet a pedantic rule
records a per-item reason, not a module-level suppression.
