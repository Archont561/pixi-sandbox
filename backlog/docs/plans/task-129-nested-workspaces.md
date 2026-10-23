# Nested Pixi workspaces — transport design (TASK-129)

Design record for [TASK-129](../../tasks/task-129%20-%20Bundle-explicitly-declared-nested-Pixi-workspaces.md)
and GitHub issue [#194](https://github.com/Archont561/pixi-sandbox/issues/194). Nothing here is
implemented yet: the sections marked **settled** are recorded conclusions, the ones marked **open**
still need the owner's ruling before slice 1 starts.

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

## Open decisions

### 1. Schema policy

- **Recommended:** the writer emits `schema: 4` **iff** the bundle declares nested workspaces;
  root-only transports stay schema 3 and byte-identical.
- Rationale: readers refuse anything newer (`schema_supported`), so an older binary facing a nested
  transport refuses loudly instead of restoring only the root envs and printing `restore complete`.
  The alternative — keep 3, add fields additively — costs nothing to root-only bundles but turns a
  real hazard into silence.
- Not ratified yet. If the owner chooses "stay at 3", the reader must instead *detect* unknown
  nested content and refuse, which is strictly more code.

### 2. Restore failure policy

- **Recommended:** keep today's sequential contract — abort on the first failed environment, leave
  earlier siblings installed, retain staged packs as evidence, exit non-zero.
- The alternative (all-or-nothing across a bundle) is new behaviour that the current restore does
  not have for root environments either; it deserves its own decision rather than riding in on
  this task.

### 3. Per-workspace Cargo vendor

`Manifest.vendor` is a single `Option<Vendor>`. A nested Rust workspace with its own `Cargo.lock`
therefore cannot be carried today. Options: (a) one vendor entry per workspace, mirroring `envs`;
(b) require a single repository-wide lockfile for bundles that declare nested workspaces;
(c) leave Rust nested workspaces out of scope for TASK-129 and file a follow-up. This must be
settled inside AC#4 rather than discovered mid-implementation.

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
