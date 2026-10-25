# Per-workspace Cargo vendor trees — transport design (TASK-131)

Design record for [TASK-131](../../tasks/task-131%20-%20Carry-per-workspace-Cargo-vendor-trees-in-one-transport.md).
The seams below are the AC#1 agreement: manifest shape, restored layout, config behaviour and
test boundaries. It deliberately mirrors the TASK-129 design, because the nested-workspace
identity model (slug, directory, per-project roots) is already ratified — this task adds a
second payload kind under the same identity, it does not invent a new one.

Read [ADR-6](../adr/ADR-6%20-%20Vendored%20cargo%20crates%20live%20under%20.pixi-sandbox-vendor.md)
first: the root tree's location, blob treatment and loose/tarball modes are fixed and do not
change here. ADR-33 extends ADR-6 to several lockfiles rather than superseding it.

## Scope in one line

One transport carries one root vendor tree **plus** one vendor tree per declared nested
workspace that has its own `Cargo.lock`; each tree is verified, restored and cargo-wired into
the project that owns its lockfile, and root-only transports stay byte-identical apart from
the manifest schema field.

## What the current code gives us

- `Manifest.vendor: Option<Vendor>` — one tree at `.pixi-sandbox/vendor/`, every file a verified
  blob, loose or per-crate tarball mode, lock hash, crate count and recorded size.
- `vendor::vendor_tree` runs `cargo vendor --locked --versioned-dirs`, refuses duplicate crate
  sources per lockfile first, and returns a `Vendor` whose blobs the pack coordinator records.
- Restore materialises the tree staging-first (verify-before-write, ADR-7), then writes the
  cargo source-replacement snippet into a sandbox `CARGO_HOME` plus pixi activation hooks
  (`--cargo-config auto`, the default), or the project's `.cargo/config.toml` (`write`), stdout
  (`print`) or nothing (`none`).
- Schema 4 already carries nested *environments* under `envs/workspaces/<slug>/<env>/` and
  records each nested project in `workspaces[]` with its slug, manifest and directory.
- `SandboxConfig::validate_workspaces_on_disk` currently refuses a nested `Cargo.lock` with a
  named TASK-131 diagnostic. That refusal is the seam this task opens.

## Settled: manifest shape (schema 5)

The root tree keeps its field and its directory. Nested trees are an additive list:

```rust
/// One nested workspace's vendored crate tree (schema 5, TASK-131).
pub struct WorkspaceVendor {
    /// Transport identity, identical to the workspace's WorkspaceEntry.slug.
    pub slug: String,
    /// The nested pixi.toml, relative to the project root: `subproject/pixi.toml`.
    pub manifest: String,
    /// The nested project's directory, relative to the project root: `subproject`.
    pub directory: String,
    /// The tree itself, reusing the Vendor struct unchanged.
    pub vendor: Vendor,
}
```

```rust
#[serde(default, skip_serializing_if = "Vec::is_empty")]
pub workspace_vendors: Vec<WorkspaceVendor>,
```

- `Vendor.directory` for a nested tree is `.pixi-sandbox/vendor/workspaces/<slug>`, the exact
  sibling of the env convention `envs/workspaces/<slug>/…`.
- Reusing `Vendor` means its blob rules, mode string, lock hash, count and size need no new
  machinery — validation, `all_blobs` and `payload_bytes` gain a second loop, not new semantics.
- Schema bumps 4 → 5. Every new transport writes 5, root-only included (the schema-4
  precedent); readers keep accepting 1–4 and refuse 5 with the existing "understands 1..=N"
  diagnostic naming the build the consumer needs. The manifest is not `deny_unknown_fields`,
  but the bump is still required: an older binary that ignored the new key would *verify
  successfully while never checking the nested tree's blobs* — the schema gate is what stops a
  silent partial verification.
- A transport without nested vendors omits the key entirely; apart from `schema`, its bytes are
  identical to today's, asserted by a test rather than assumed.

## Settled: transport and restored layout

| Payload | In the transport | Restored to |
| --- | --- | --- |
| Root vendor (unchanged) | `.pixi-sandbox/vendor/` | `<project>/.pixi-sandbox/vendor/` |
| Nested workspace vendor | `.pixi-sandbox/vendor/workspaces/<slug>/` | `<project>/<directory>/.pixi-sandbox/vendor/` |

The nested destination is the owning workspace's own `.pixi-sandbox/`, beside where its
environment already lands (`<directory>/.pixi/envs/<env>`). Cargo discovers config by walking
up from its working directory, so a build run inside `subproject/` reads that project's own
wiring and never sees the root's vendor directory.

### Cargo wiring per workspace

`configure_cargo_vendor` becomes per-tree:

- **auto (default):** each tree gets `<directory>/.pixi-sandbox/cargo-home/config.toml` with a
  snippet naming *that* tree's vendor directory, and the activation hooks are written into the
  nested workspace's **own** env(s) under
  `<directory>/.pixi/envs/<env>/etc/conda/activate.d/` — the same mechanism as the root, scoped
  by where the env lives. Activating the nested env therefore exports its own `CARGO_HOME`.
- **write / print / none:** applied independently per tree with the existing meanings; `print`
  emits one labelled snippet per tree so nothing is silently merged.
- `--no-vendor` skips every tree, root included (today's semantics, extended honestly).
- `--force` applies per destination; the existing "exists without --force" refusal is kept.

## Settled: pack behaviour and the end of the Cargo.lock refusal

- The hard refusal in `validate_workspaces_on_disk` is **removed**. A nested `Cargo.lock` is
  legitimate cargo project metadata; the old refusal existed only because the transport had no
  place for the tree.
- With `--cargo-vendor`: the root tree is produced exactly as today when the root has a
  `Cargo.lock`; then each declared workspace whose directory contains a `Cargo.lock` gets its
  own `cargo vendor` invocation rooted in that directory, each with its own
  duplicate-source preflight against its own lock, written into
  `payload/vendor/workspaces/<slug>/`. A declared workspace *without* a `Cargo.lock` simply has
  no tree (nothing to vendor) — the pixi lockfile check from TASK-129 is untouched.
- Without `--cargo-vendor`: no tree is produced for anyone, matching the root contract today.
- `vendor_tree` gains a parameter for its vendor-directory relative path; the root call keeps
  `vendor/`, nested calls pass `vendor/workspaces/<slug>`. The fake-cargo wrapper test seam
  (a wrapper that copies a canned tree instead of solving) is how every per-workspace test runs
  hermetically — no registry access.

## Settled: budget, provenance and the shared crate (AC#4)

- `Manifest::payload_bytes` and the transport ceiling sum the root tree **plus every workspace
  tree**: bytes on a branch are bytes, regardless of which project owns them.
- `plan` extends each `WorkspacePlan` with `vendor_crates` / `vendor_size_bytes` (absent when the
  workspace has no lockfile), so the reviewed plan names every tree's cost before a runner
  packs it.
- **Shared crates are counted honestly, not deduplicated away.** With `--versioned-dirs`, a
  crate+version shared by two lockfiles produces two identical
  `<name>-<version>` directories under two transport paths. They are two blob lists, two
  verifications and two checkouts — the manifest records both. Git's content-addressed storage
  keeps one underlying object, so *branch storage* is smaller than the summed payload; the plan
  reports the overlap explicitly (`shared_crates`, `shared_bytes`: identical relative crate
  directories present in at least two trees) with the explanation that checkout and
  verification still pay twice. No cross-tree hardlinking or single-instance optimisation: it
  would couple two independently-restored projects and break the per-workspace destination
  rule.
- Each nested tree records its own `cargo_lock_sha256`, so provenance stays per lockfile.

## Rejected alternatives

1. **One merged vendor tree for all lockfiles.** Rejected in TASK-129's premise for envs and
   rejected here for the same reason: `cargo vendor` keys on one `Cargo.lock`, the duplicate
   source refusal is per-lockfile and meaningful, and a merged tree would need a new conflict
   policy the two-project case exists to avoid.
2. **Vendoring nested crates under the root tree with a prefix.** Would give two projects one
   shared directory and one config snippet, reintroducing cross-project coupling and making a
   partial restore ambiguous.
3. **A manifest map keyed by slug instead of a list.** The rest of the manifest uses ordered
   lists (`workspaces[]`, `envs` BTreeMap serialised deterministically); a list with the slug
   inside matches the established shape and keeps byte output stable without map-order rules.
4. **Keeping the nested-Cargo.lock refusal and gating it on a flag in the config.** The refusal
   is a pack-capability limit, not a validation rule; leaving it in `plan`/`doctor` would keep
   rejecting valid repositories that simply choose not to vendor.

## Test boundaries (ADR-10: tempdirs and fixtures, never this checkout)

1. **Manifest unit tests** — schema-5 serde of `WorkspaceVendor`; root-only byte identity apart
   from `schema`; readers keep accepting schema 4 (and older) with no nested trees; blob
   validation and `payload_bytes` cover nested trees; a bad blob path under
   `vendor/workspaces/…` is refused. Slice 1 additionally enforces: empty-slug refusal, the
   one-segment-directory rule already applied to embedded `WorkspaceRef`s, one tree per slug
   (duplicate refusal — the restore destination is slug-derived), and exact slug/manifest/
   directory matching against a declared `workspaces[]` entry (an orphan tree has no
   destination). Nested blobs join `Manifest::blobs()` under a `vendor/workspaces/<slug>`
   context, whose ownership is carried by `Cow` so static contexts stay borrowed.
2. **Vendor producer tests** — per-workspace output lands in the qualified directory through
   the fake cargo wrapper; two trees carrying the same canned crate produce two verified blob
   lists and the overlap arithmetic the plan reports; duplicate-source preflight still fires
   per workspace lockfile.
3. **Pack/restore integration** — a tempdir repo with root and nested `Cargo.lock`s packs both
   trees, the manifest names both, restore materialises each into its owning project,
   `--no-vendor` restores none, `--cargo-config write` produces one config per project, and the
   existing "exists without --force" refusal survives per destination.
4. **Offline build proof (ignored Linux test, CI gate like `test-nested-offline`)** — two tiny
   cargo projects with real vendored registry crates pack, restore inside an `unshare -n`
   namespace, and both build with `cargo build --offline` against their own trees; a cold
   negative control (tree removed) fails. This is the only tier that claims a real offline
   build; everything below it is hermetic fixture logic.

## Slice order

0. This document (the AC#1 seams artifact). ADR-33 is deliberately written last, in slice 5,
   so the decision record describes the implementation that actually shipped.
1. Schema 5 + `WorkspaceVendor` in the manifest: serde, validation, byte accounting, root-only
   identity and old-reader refusal (tests first).
2. Pack: parameterised vendor directory, per-workspace `vendor_tree`, blob recording, removal
   of the config refusal, branch-doc generation.
3. Restore: per-tree staging/materialisation and per-workspace cargo wiring across all four
   `--cargo-config` modes plus `--no-vendor`/`--force`.
4. Plan and budgets: per-tree size/crate fields, overlap reporting, transport ceiling.
5. ADR-33 text, user docs, the ignored offline build proof, full gates and the task ledger.

## Progress log

- **Slice 1 — merged locally as `2c7c5a7d`.** Schema 5 ships with stricter identity rules than
  the first draft: one tree per slug (duplicate refusal — the restore destination is
  slug-derived) and exact slug/manifest/directory matching against a declared `workspaces[]`
  entry, so an orphan tree cannot be packed. The one-segment-directory rule mirrors the
  existing `WorkspaceRef` check rather than relaxing TASK-129's policy. `Manifest::blobs()`
  returns `Cow<'_, str>` contexts so workspace-qualified owners cost an allocation only when
  they exist.
- **Slice 2 — pack side.** `vendor_tree(_with_toolchain)` takes a `vendor_rel` parameter; the
  root keeps its historical `.cargo-vendor-tmp` scratch name while workspace trees get
  `.cargo-vendor-tmp-<slug-qualified>` names. Preflight now runs per distinct project root and
  refuses only when *no* root carries a `Cargo.lock`; a workspace without one is silently not
  vendored. A subtlety the tests caught: root blob recording walks `payload/vendor`, which
  contains the nested `workspaces/` subtree — `record_tree` gained an exclusion seam so root
  and nested trees never double-record the same files. The nested-`Cargo.lock` config refusal
  is gone; branch guides gain one paragraph per workspace tree while root-only goldens stay
  byte-identical.
