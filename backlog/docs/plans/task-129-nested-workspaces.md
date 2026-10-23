# Nested Pixi workspaces — transport design (TASK-129)

Design record for [TASK-129](../../tasks/task-129%20-%20Bundle-explicitly-declared-nested-Pixi-workspaces.md)
and GitHub issue [#194](https://github.com/Archont561/pixi-sandbox/issues/194). Nothing here is
implemented yet. The decisions below were ratified by the owner on 2026-10-23 during the design
grilling; each one records what was rejected as well, so a later session cannot quietly reopen it.

Read [ADR-32](../adr/ADR-32%20-%20A%20transport%20branch%20is%20one%20bundle%20for%20one%20platform%20and%20its%20payload%20stays%20self-contained.md)
first — the branch topology is fixed, so everything below extends the payload *inside* one branch.

## Scope in one line

A bundle may declare nested Pixi workspaces; `plan` reports them, `pack` carries their
environments, and `restore` materialises each into the nested project's own `.pixi/envs/<env>`, so
`pixi run --manifest-path python/pixi.toml …` finds a populated prefix and never solves or fetches.

## What the current code already gives us

| Fact | Where | Consequence |
| --- | --- | --- |
| `native_pack::preflight(project, environment, platform)` / `write(…)` read `<project>/pixi.lock` | `crates/pixi-sandbox/src/native_pack/writer.rs` | a nested workspace is just another *project root*; no packer rewrite |
| Blobs are namespaced `envs/<name>/pack/…`, and restore derives relative paths from that prefix | `commands/restore.rs` | the per-environment unit is already the axis nesting needs |
| The per-file oracle canonicalises the destination to `SENTINEL` | `core/src/files_manifest.rs` | one oracle per environment keeps working **once each destination is fed in** |
| `Manifest.envs` is `BTreeMap<String, Env>`; `Manifest.platform` and `Manifest.vendor` are single values | `core/src/manifest.rs` | the env map generalises; `vendor` does not |
| `ensure_destinations_available` checks only root env dirs and the vendor dir | `commands/restore.rs` | it must widen to every declared destination before anything is written |
| Restore is sequential and aborts on the first failure, keeping staged packs as evidence | `commands/restore.rs` | the existing contract, not a transaction |

## Settled: wire shape

**Blob layout.** Root unchanged; nested under one fixed segment:

```text
.pixi-sandbox/envs/<env>/pack/…                     # today, byte-identical
.pixi-sandbox/envs/workspaces/<slug>/<env>/pack/…   # new
```

`workspaces/` is a fixed, checkout-safe segment: a `python:default` or `python/default` id inside
the branch would be illegal on NTFS, and prefix-anchored blob matching means the segment cannot
collide with a root environment. `plan` therefore refuses a root environment literally named
`workspaces` in a bundle that declares nested workspaces — a diagnostic instead of a silent
overlap.

**Manifest.** `Env` gains an optional workspace descriptor (nested manifest path + destination
directory; the fingerprint already exists per env). `Manifest` gains an optional `workspaces` list
carrying the declared set for reporting and verification. A root-only bundle serialises exactly as
today.

**Restore destination.** Each nested environment lands in its own project's Pixi env dir:

```text
<project>/.pixi/envs/<env>                     # root workspace, unchanged
<project>/<workspace dir>/.pixi/envs/<env>     # nested
```

That is the layout `pixi run --manifest-path …` reads, which is what makes the delegated task
offline. Nesting everything under the root `.pixi/envs` was rejected: it needs `PIXI_ENV_DIR`
redirection and would not survive the consumer's own `--frozen` checks.

**Guarantees and where they live.**

| Guarantee | Extension |
| --- | --- |
| Verify before write (invariant 1) | Per environment; blob sets are disjoint by construction |
| Per-file oracle (ADR-13) | Same algorithm, one oracle per env; each destination is fed to the sentinel substitution |
| Markers / "`pixi install` is a no-op" (ADR-5) | Written per prefix from that workspace's own fingerprint |
| Refuse before writing | `ensure_destinations_available` covers every declared destination |
| Budgets (ADR-18) | Nested bytes count against the same bundle ceiling; `plan` reports them before packing |

**CLI.** `restore --envs` and `doctor --verify-restored --envs` accept `<workspace>/<env>` ids; a
bare `default` keeps meaning the root environment, so existing invocations and scripts stay valid.
`plan --json` gains `workspaces[]` (manifest path, environments, resolved package set, approximate
size) and stays the single source of the workflow matrix — `publish-sandbox.yml` keeps one-line
steps because `pack` already receives `--config`.

**Conflict refusal (part of AC#3).** A nested `pixi.toml` that is a *member of the root Pixi
workspace* resolves its environments from the root `.pixi/envs`; declaring it separately would
duplicate or fight over a prefix. `plan` detects the root `[workspace]` section and refuses such a
declaration with a named remedy.

## Ratified decisions (2026-10-23)

### 0. The premise is unproven — prove it first

The owner has **not** verified that the reporting project's subprojects cannot simply be declared
`[workspace]` members of the root `pixi.toml`. A member layout resolves everything from one
lockfile and one `.pixi/envs/`, which would reduce TASK-129 to documentation plus a plan-time
diagnostic.

**Ruling:** slice 0 is a fixture-only proof that the member route cannot serve the case (conflicting
channels, versions or platforms). If the member route works, TASK-129 is closed as a docs change
before any transport code is written. No transport work starts until slice 0 is recorded.

### 1. Config schema is bumped to 2

`Bundle` is `#[serde(deny_unknown_fields)]` (`sandbox_config.rs:95`), so a config carrying
`workspaces = […]` makes every older binary refuse the *whole* config. The owner chose the loud
failure: `CONFIG_SCHEMA` becomes `2`, `schema_supported` keeps reading schema 1, and a schema-2
config on an older binary is refused with the version requirement named.

Rejected: accepting the bare parse failure (indistinguishable from a typo), and relaxing
`deny_unknown_fields` (turns typos into silently ignored configuration).

### 2. Manifest schema is 4 for every new transport

Not "only when nested". Uniform producer output: every new `pack` writes `schema: 4`, root-only
bundles included, so there is one writer path and one reader matrix rather than two spellings of
the same transport.

Accepted cost: every consumer on a schema-3-era binary must upgrade before it can read a transport
produced after this lands, even one with no nested workspaces. Readers stay able to read schemas
1–3 (ADR-24/31), and `tests/manifest.rs` grows fixtures for 4.

Rejected: schema 4 only-when-nested (two writer paths, cheapest migration) and staying at 3
additively (an old binary restores the root environments, skips nested ones and still prints
`restore complete`).

### 3. Restore keeps the sequential abort

First failed environment stops the run, earlier siblings stay installed, staged packs are retained
as evidence, exit non-zero — the behaviour root-only restore already has, now specified for a
bundle with several environments.

Rejected: all-or-nothing rollback (new behaviour that root-only restores do not have either, so it
needs its own decision) and "preflight-complete then best effort" (still needs the abort during
install, so it adds a second policy for a narrower failure window).

### 4. A nested Rust workspace with its own lockfile is refused, not carried

`Manifest.vendor` is a single `Option<Vendor>` with one `cargo_lock_sha256`. TASK-129 refuses such a
declaration with a named diagnostic and points at the follow-up task; per-workspace vendor entries
would touch ADR-6, `--no-vendor` semantics and the budget/provenance model, which is its own task.

Rejected: per-workspace vendor entries inside this task, and requiring one repository-wide
lockfile (contradicts the feature's premise).

### 5. AC#6 asserts both observables

The nested project's `pixi install --frozen --offline` is a **no-op** (proves the prefix is
complete and deterministic), and the delegated `pixi run --manifest-path … <task>` succeeds inside
the denied-egress namespace (proves the user-visible outcome). The no-op is the assertion that
discriminates; the denied-egress run alone can pass on a fixture whose only channel is a local
directory, which is exactly the false proof the AC must not allow.

## Hazards found while grilling (folded into the slices)

| Hazard | Where | Handling |
| --- | --- | --- |
| `pack` preflights `refuse_non_wheel_pypi` and `native_pack::preflight` against **one** `root` for **all** `--envs` | `commands/pack.rs` | the workspace selector resolves each environment to its own project root; `--envs` entries keep one id syntax everywhere (`<workspace>/<env>`), resolved through the manifest, never a path join |
| `validate_slug` forbids `/`, so a slash exists only in CLI text | `sandbox_config.rs:620` | parse → validate → look up in the manifest's declared set → rebuild the destination from the **manifest's** descriptor; user text never becomes a path component |
| Restore stages into `work/pack-<environment>` and `clean_work_dir` iterates the same names | `restore.rs:391`, `:557` | staging identity becomes workspace-qualified; root `default` and nested `default` must not share a directory |
| `verify_restored` derives the destination from the env name (`project/.pixi/envs/<name>`) | `verify.rs:239` | destination resolution moves into core, shared by restore, `doctor --verify-restored` and the prefix-candidate logic |
| TASK-129 and TASK-130 both claim a plan-time size estimate | task files | TASK-129 owns the environment/workspace estimate; TASK-130 owns the provider estimate and cites it |


## Evidence plan

A fixture project with a root `pixi.toml` and a nested `python/pixi.toml`, packed and restored into
a tempdir, then `pixi run --manifest-path python/pixi.toml <task>` executed inside the denied-egress
namespace with the local channel as the only source. That single run is AC#6. The native-runner
airlock tier stays a CI claim, never inferred from the fixture.

## Platform specificity (input to TASK-130, recorded here for one place of truth)

| Ecosystem | Payload bytes platform-specific? | What varies |
| --- | --- | --- |
| Rust crates.io `.crate` | No — source tarballs are identical for every target; per-arch assembly ships inside one crate | the build: compiler, `-sys` crates, build scripts |
| `cargo vendor` tree | No — `cargo vendor --locked --versioned-dirs` reads the target-inclusive `Cargo.lock` | only if lockfiles differ per platform (they normally do not) |
| Go modules | No — module zips and `go.sum` are platform-independent | cgo needs a C toolchain; `GOOS`/`GOARCH` are compile-time |
| NuGet | Mixed — `runtime.<rid>.*` packages and per-RID native assets are platform-specific | `runtimes/<rid>/native/…` selection at restore |
| Maven | Mixed — classifier JARs (`-linux-x86_64`, `-natives-*`) are platform-specific; many JARs bundle all natives; plugins are Java but may shell out to a compiler | classifier selection; the JDK itself is per-platform and comes from the Conda environment |

Consequence: Rust and Go providers can ship **one** vendor tree per branch (ADR-32 holds), while
NuGet and Maven providers must resolve per-platform inputs rather than mirror `Cargo.lock`'s
single-lock model.
