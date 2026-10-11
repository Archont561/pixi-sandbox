---
id: ADR-18
title: "`[workflow]` table presence, not mere field defaults, is the opt-in to safer policy (issue #79, task-53)"
status: accepted
tags:
  - decision
---

# ADR-18 — `[workflow]` table presence, not mere field defaults, is the opt-in to safer policy (issue #79, task-53)

**Decision.** An optional `[workflow]` table in `pixi-sandbox.toml` carries generated-publisher
CI policy (`push_paths`, `permissions`, `concurrency`, `timeouts`, `pixi_version`,
`setup_pixi_cache`) so a consumer never hand-edits the generated workflow. No `[workflow]`
table at all renders byte-identically to the pre-task-53 template — the migration path for
every existing consumer. Once the table is *present at all* (even empty), two fields move to a
safer default unless given an explicit value: `setup_pixi_cache` defaults to `false`, and
`push_paths` defaults to a derivation from the known transport inputs (the config itself,
`pixi.toml`, `pixi.lock`, and whichever of `package.json`/`bun.lock`/`Cargo.toml`/`Cargo.lock`
actually exist) instead of the unfiltered `push: branches: [main]` trigger. The other three
fields (`permissions`, `concurrency`, individual `timeouts`) carry no such defect in the status
quo and stay fully per-field opt-in regardless of the table's presence.

**Why.** `setup_pixi_cache`'s default is a correctness fix, not a preference: pixi-sandbox owns
the native `pixi install` while packing, and an action cache keyed on the consumer manifest can
restore a solve the pack will not reuse, silently publishing from a stale environment. An
unfiltered push trigger is a cost and correctness hazard of the same shape (the issue's own
words: "every push to `main` repacks the transport and force-pushes an orphan branch, even a
README typo"). Grouping these two under "the table's presence is itself the opt-in" means a
consumer who writes `[workflow]` only to pin `pixi_version` or set a `timeout-minutes` does not
silently also gain a `paths:` filter they never asked for and might not expect — an unexpectedly
narrowed trigger that could silently skip a real publish-worthy change is a worse failure mode
than a stale cache, so it needs its own, visible field in the same table rather than hiding
behind a different field's edit. Equally, scoping `permissions`/`concurrency`/`timeouts` to
their own explicit keys means a consumer who wants only the two correctness fixes is not forced
to also adopt a stricter permissions model or a job timeout they have not reviewed.

**Evidence.** Fixture tests hold both halves directly: `workflow_policy::absent_workflow_table_is_none`
and `generated_workflow::workflow_policy::default_options_add_no_new_yaml` prove the
byte-identical absent case; `workflow_policy::resolved_push_paths_derives_from_present_transport_inputs`
proves the table's mere presence (no explicit `push_paths`) is enough to switch on the
derivation, scoped to manifests that actually exist on disk;
`init_with_workflow_policy_reproduces_byte_identically_with_no_hand_edits` proves a consumer
with every field configured regenerates with `init --check` clean and a second `init` produces
identical bytes — no hand-edit-and-reapply ritual survives. `xtask lint-generated-workflow`
runs actionlint across the full combination table (each block alone and all of them together).

**What would change it.** A consumer who wants per-field opt-in for cache/push-paths too (no
precedent for this surfacing yet); or evidence the "safer once present" default itself
surprises consumers in practice, which would argue for splitting `[workflow]` into stricter
sub-tables (`[workflow.safety]` always-on-if-present vs. `[workflow.preferences]` per-field)
rather than one flat table with two specially-defaulted fields.
