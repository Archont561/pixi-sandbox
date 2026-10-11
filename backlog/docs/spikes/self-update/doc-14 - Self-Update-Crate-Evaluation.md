---
id: doc-14
title: Self-Update Crate Evaluation
type: other
created_date: '2026-10-11 15:05'
updated_date: '2026-10-11 15:05'
tags:
  - spike
  - self-update
  - dependencies
  - decision
---

# Self-Update Crate Evaluation

## Status

**Spike only — no dependency added, no source changed.** `self_update = "1.3"` is already
declared (`Cargo.toml:57`, `crates/pixi-sandbox/Cargo.toml:39`) ahead of this verdict so the
relocked vendor tree carries it; this document decides whether that line stays. Nothing in the
hand-rolled updater was touched.

Measured on a connected Codespace on 2026-10-11 against `main` (`2883243e`, v0.6.1), where
crates.io **is** reachable and the crate is compiled into the workspace. That is the asymmetry
TASK-78 already exposed: the airlock cannot fetch the crate at all, so every claim below that
needs the crate's bytes was read here, and the ones that only need its already-vendored sources
are cited by file and line. The workspace builds with the crate in the graph
(`cargo build -p pixi-sandbox` exit 0; `cargo tree -p pixi-sandbox -i self_update` →
`self_update v1.3.0 └── pixi-sandbox v0.6.1`).

## Verdict

**Adopt partially — the release-lookup and transport half, not the replace half.**

- **Adopt (delete ours):** `resolver.rs` and `checksums.rs`, plus the download academy inside
  `fetch`: use the crate's `backends::github` release lookup, its `Download` engine, and its
  `Checksum`/SHA256SUMS parser. This buys the one capability we genuinely lack — GitHub
  rate-limit handling — with a shipped, tested HTTP client instead of our untested one.
- **Keep ours (do not call `update()`):** `ownership.rs` (the refusal ladder, run before a byte
  moves), `replace.rs` (`ReplaceStrategy` as an explicit parameter, `windows_swap`,
  `sweep_displaced`), and `mod.rs`'s mode/header/reporting orchestration.

The crate's `update()` is **not** usable as-is: it owns both the replace dance (via
`self-replace`, whose strategy is a `cfg!` read, not a parameter) and the ordering of the
ownership decision relative to the download. Adopting the whole `update()` would surrender both
must-survive properties; adopting only the primitives keeps both and still removes the code we
most want gone. That is the "which halves" this document settles.

## What we have (the thing being judged)

`crates/pixi-sandbox/src/self_update/*`, 889 lines of production code:

| module | lines | role |
| --- | --- | --- |
| `resolver.rs` | 94 | latest-vs-exact version resolution and validation |
| `assets.rs` | 60 | canonical host→`pixi-sandbox-<triple>` asset map (mirrors `xtask/src/release_assets.rs`) |
| `checksums.rs` | 81 | `SHA256SUMS` parse + per-host entry selection |
| `ownership.rs` | 145 | the refusal ladder (`classify`) — managed launcher, `pixi global` trampoline, `conda-meta/` prefix, manifest-verified transport tool |
| `replace.rs` | 214 | `ReplaceStrategy` (Unix rename / Windows swap), `sweep_displaced` |
| `mod.rs` | 295 | orchestration, plan/check/run modes, header and report rendering |

Judged by 94 tests in `crates/pixi-sandbox/tests/self_update*.rs` (1,710 lines over seven files).

## AC#2 — the `windows_swap` corpse question

**Replacing a running image.** `self-replace` `windows.rs` `self_replace` (lines 282–292):
canonicalise `current_exe`; rename it to a sibling named `<name>.<random>.__relocated__.exe`
(`RELOCATED_SUFFIX`, `windows.rs:30`); `schedule_self_deletion_on_shutdown` registers
`self_delete_on_init` (`windows.rs:127`) as a CRT init function that waits on the parent's
handle and calls `DeleteFileW` (`windows.rs:165`); copy the new binary to
`<name>.<random>.__temp__.exe` (`TEMP_SUFFIX`, `windows.rs:31`) and rename it into place.
On Unix (`unix.rs`) `self_replace` writes a `tempfile` in the same directory and `fs::rename`s
it over the exe (atomic); `self_delete` is an immediate `fs::remove_file`.

**Reaping stale `.old-*` siblings.** `self-replace` does **not** sweep them, and says so.
`lib.rs:105–115`, heading *"Limitations"*: leftover files "resemble the original names of the
executable prefixed with a dot and a random suffix", and "**It's not recommended to run
automatic cleanup on startup** as the location of those temporary files placed is undefined"
(`lib.rs:111–112`). A power cut between the two renames leaves a `.__relocated__.*` corpse that
stays until the OS's own temp cleanup, or forever if it landed beside the exe.

**Our behaviour is the opposite, on purpose.** `replace.rs` `windows_swap` renames the running
image aside to a deterministic `<name>.pixi-sandbox-old-<version>` sibling, and `sweep_displaced`
reaps stale `.pixi-sandbox-old-*` siblings as **step 1 of every update** (`replace.rs:161`,
invoked from `replace.rs:80`). So a partial adoption that keeps `replace.rs` keeps the sweep;
a full `update()` adoption would replace our reaping with the crate's non-reaping behaviour.
That is a reason to keep `replace.rs`, not merely a neutral difference.

Nothing here is unverified: both crates' sources are in the workspace's cargo registry cache and
were read directly.

## AC#3 — the two must-survive properties

1. **Refusal before download.** `ownership::classify` (`ownership.rs`) is called in `mod.rs`
   before the fetch, and the crate's verify hooks run *after* the artifact is on disk — the
   archive hook at `self_update` `update.rs:1607`, the binary hook at `update.rs:1786` (both
   `run_verify_hook`/`run_archive_verify_hook`, defined `update.rs:1813`/`:1833`, called only
   from the install path). Under the partial adoption the ordering is preserved by
   construction: we keep our own orchestration and never hand control to `update()`; the
   crate only ever sees a destination that `classify` has already cleared. **Survives.**
2. **`ReplaceStrategy` as a parameter.** `install_binary` (`update.rs:1781`) branches on
   `dest == current_exe`: equal → `self_replace::self_replace(new_exe)` (`update.rs:1793`),
   else → `Move::from_source(new_exe).to_dest(...)` (`update.rs:1796`). Both are `cfg!`-selected
   inside `self-replace`, not parameterised, so there is no seam to inject the Unix dance on a
   Windows host or vice versa. The partial adoption keeps `replace.rs` and its 24 tests (which
   include the `::case_1`/`::case_2` Unix/Windows pairs run on every machine) plus the
   `replace-running-image` job on `windows-latest`. **Survives.** Nothing is lost, so the
   "say what is lost and why it is acceptable" branch does not apply.

The one thing we give up is `self-replace`'s automatic shutdown-time self-deletion on Windows.
We do not need it: `sweep_displaced` runs at the start of the *next* update and reaps any
deterministic `.pixi-sandbox-old-<version>` sibling left by an interrupted swap, which is a
strictly stronger guarantee than the crate's "the OS may clean it up, maybe not".

## AC#4 — test-surface mapping (94 tests, per file)

Counts obtained with `cargo test -p pixi-sandbox --test <name> -- --list` for each of the seven
roots.

| file | tests | verdict | detail |
| --- | --- | --- | --- |
| `self_update.rs` | 24 | **re-point** | orchestration and end-to-end flow. The ownership/refusal, mode (`--check`), header and report cases stay as-is (they drive our code). The resolve/download/verify cases (e.g. `latest_by_default_resolves_downloads_verifies_and_replaces`, `an_exact_version_installs_that_release_even_when_it_is_older_than_latest`, `a_missing_binary_asset_is_an_http_failure_that_writes_nothing`) move their fake from our resolver to the crate's `ReleaseSource` trait, injected through a test `HttpClient`. No drops. |
| `self_update_assets.rs` | 4 | **keep** | asserts our reviewed host→asset data (the invariant shared with `xtask/src/release_assets.rs`). That map is the asset identifier handed to the crate's `Release::{asset_for, download}`, so the data — and its test — stay ours. |
| `self_update_checksums.rs` | 5 | **drop → crate** | the SHA256SUMS contract (text/binary modes, leading paths, BSD tag form, `#` comments, missing entry ⇒ `ChecksumSourceInvalid`) is exactly what the crate's checksum path implements and tests (`checksum_from_asset`, and `Checksum::from_sums_file`). The contract does not move; only its home does. 5 dropped, reason per row: covered by the crate. |
| `self_update_fetch.rs` | 6 | **re-point** | drives a release fetch through the crate's `Download` + `Checksum`. `fetch_release_refuses_a_pixi_managed_destination_before_downloading_anything` stays ours (ownership, pre-download). No drops. |
| `self_update_ownership.rs` | 12 | **keep** | the refusal ladder, entirely ours (AC#3.1). No change. |
| `self_update_replace.rs` | 24 | **keep** | `ReplaceStrategy` + `windows_swap` + sweep, entirely ours (AC#3.2). No change. |
| `self_update_resolver.rs` | 19 | **drop/re-point** | latest-vs-exact resolution and tag validation become the crate's `ReleaseSource::get_latest_release` / `get_release_version` (`macros.rs:403`/`:415`) over `semver`. The bulk (the 9 `anything_that_is_not_an_exact_release_version_is_refused` cases, the 5 `exact_versions_are_accepted`, the v-prefix and never-ask-latest cases) is replaced by crate behaviour and its own tests. Any input-validation we want to keep (rejecting a non-release tag before lookup) survives as a thin wrapper on our side — a re-point, not a drop. |

**Count moving: 70 kept or re-pointed, 24 dropped-and-replaced-by-the-dependency** (5 checksums
+ 19 resolver). Dropping is not silent deletion — it is the deliberate movement of a contract
into a dependency whose own test suite then owns it, which is the point of the adoption.

## AC#5 — dependency-surface delta

**Feature set (declared, measured as enabled):** `default-features = false, features =
["checksums", "github", "rustls", "ureq"]` — confirmed by
`cargo tree -p pixi-sandbox -e features -i self_update`, which shows exactly those four
features enabled and nothing else. `reqwest`, `indicatif`, `archive-*`, `compression-*`,
`signatures` stay off.

**Crates gained by the enabled feature set:** `self-replace 1.5.0` (pulls `fastrand 2.5.0` and,
on Windows only, `windows-sys 0.52.0`) and `urlencoding 2.1.3`. `sha2 0.11.0` was already
present at the exact version the `checksums` feature wants; `ureq 3.4.2`, `http 1.5.0`, `log`,
`regex`, `semver`, `serde`, `serde_json`, `tempfile` were all already in the lock.

**Crates that stay out of the build:** `cargo tree -p pixi-sandbox -i reqwest` and
`-i hyper` both print "nothing to print" — feature-disabled, so not compiled, linked, or
shipped.

**Lockfile noise the declaration still carries:** `Cargo.lock` records `self_update`'s optional
dependencies anyway — its entry lists `reqwest` (with the hyper/tokio tree behind it) even
though no enabled feature activates it. This is cargo locking optional deps it will not build;
it is supply-chain/lock surface, not binary surface, but it is real and the `Cargo.toml:52`
comment already names it.

**Airlock limitation and the connected-host step.** This evaluation **could not** be built or
tested in the airlock — `crates.io`, `index.crates.io` and `static.crates.io` are refused and
the crate is absent from that cargo cache. It was built and reasoned about here, on the
connected host. Before any migration task starts, the connected host must (a) refresh
`Cargo.lock` for the enabled feature set and (b) perform one real `cargo build -p pixi-sandbox`
plus one real `self-update` round trip against a published release, to confirm the transport
behaves as its sources say. The airlock then receives the relocked vendor tree through the
normal repack, exactly as it received `rstest-bdd`'s runtime and macros.

**MSRV is already owed.** The crate declares `rustc 1.88` against the workspace's `1.85`
(`Cargo.toml:13`), but so does the already-in-use dev-dependency `rstest-bdd 0.6.0`
(`Cargo.toml:68`), whose comment says its MSRV is 1.88 too. The workspace therefore has to
lift `rust-version` to 1.88 regardless of this verdict; adoption adds no *incremental* MSRV
cost. Lifting it is TASK-92, which also switches on clippy's MSRV-gated `collapsible_if` lints
(six sites in `pixi-sandbox-core`) and is deliberately kept out of this spike.

## AC#6 — where the outcome lives

The outcome is an adoption, so it is recorded as a decision:
`backlog/decisions/decision-5 - Adopt-the-self_update-crate-transport-not-its-replace-path.md`.
doc-14 carries the analysis; decision-5 carries the call, the halves, and the consequences.

**No production code or dependency is migrated in this task.** The `self_update` declaration in
`Cargo.toml`/`crates/pixi-sandbox/Cargo.toml` and its `Cargo.lock` entry were already present
before this task; this document neither adds nor removes a dependency.

## Follow-ups this verdict creates

- **TASK-92** (raise `rust-version` to 1.88, clear the six MSRV-gated `collapsible_if` sites)
  becomes required by this verdict as well as by `rstest-bdd`.
- A **new migration task** (not filed here) would: delete `resolver.rs`/`checksums.rs`, replace
  the fetch internals with the crate's `backends::github` + `Download` + `Checksum`, keep
  `ownership.rs`/`replace.rs`/`mod.rs`, and move the 24 dropped tests' contracts onto the
  dependency. It must start on the connected host, after the lockfile refresh and the one real
  round trip above.
- **TASK-91** (unify the two GitHub adapters under one module) is unaffected in shape but should
  be revisited *after* the migration, since the crate brings its own GitHub backend.
