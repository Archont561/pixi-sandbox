# Evidence

Every number the design leans on, as measured. Nothing here is estimated unless marked `⚠️` or
`open`.

**Lab (unless a section says otherwise):** Debian 13 container, 2 vCPU, ~20 GB free disk,
pixi 0.81.0, pixi-pack/pixi-unpack 0.7.11 (static musl), cargo/rustc 1.98.1, linux-64,
git 2.4x, Python 3.13, date 2026-09-20.

---

## §1 Toolchain and baseline

* pixi 0.81.0 installs with the official installer; `pixi global install rust` provides
  cargo 1.98.1 / rustc 1.98.1.
* A pixi environment for a real project (`dev`) is ~507 MiB unpacked / ~5 567 files; those
  files are **hardlinks** into the package cache, not copies.
* `cp -a` of such an environment works at runtime (it is a valid prefix) but leaves **213
  files** carrying build-path references — which is exactly why the transport does not carry
  prefixes (D2).
* `unshare -rn` works in this sandbox, which is how every "no network" claim below was
  produced (network namespace with no interfaces plus a dead proxy).

## §2 What `pixi-pack` / `pixi-unpack` produce

* `pixi-pack --directory-only` output for one environment: **65 MB** with a local channel of
  raw `.conda` files + `environment.yml` + `pixi-pack.json`. Same environment as a prefix
  tar: ≈**3.7× larger** ✅.
* `pixi-unpack <pack>` restores it offline with an empty package cache; pixi then accepts the
  environment if the markers are right (§4).
* Neither `pixi-pack` nor `pixi-unpack` exists on crates.io ✅ (both are git-only projects),
  which is why D4 ships static release assets.
* Clip of the environment fingerprint: pixi stores 16 bytes of xxh3 over the lock's
  `(name, sha256)` records → 16 hex characters, in
  `conda-meta/.pixi-environment-fingerprint`; it identifies the *lockfile*, so it cannot
  detect a tampered prefix (the per-blob sha256 catalogue does that).

## §3 Git economics of the transport

| measurement | result ✅ |
| --- | --- |
| full pipeline, two envs: packed payload → branch size | 218.8 MiB → **107.34 MiB** (~2× dedup) |
| cost of adding one new 5 MB package | ≈ +4.8 MiB in the branch |
| `git push --force` then `git gc --aggressive` on the orphan repo | **no shrink** |
| recompression of `.conda` payloads (already compressed) | minimal win — additive storage |
| GitHub per-file limits | >100 MiB rejected, >50 MiB warned |
| GitHub push/repo budgets | push ≤2 GB, repo soft ~1 GB |

Consequences: 95 MiB shard limit, one branch per (platform × env-set), rotation by rebuilding
a branch rather than by force-pushing, and CI that reports the branch size per publish (D1).

## §4 Integrity and tamper behaviour

* A transported environment whose single missing file was deleted ⇒ `doctor --verify` reports
  `missing` for that path and *all* other failures, not just the first ✅.
* A blob overwritten with different bytes ⇒ `integrity … expected sha256 … got …`, exit 1 ✅;
  `restore` writes nothing.
* A split blob with one corrupted part ⇒ refuses, and **no half-written file is left** ✅
  (this is the case that produced the atomic join in D7; the first implementation left a
  partial destination behind and a property test caught it).
* Hand-placed but *correct* environment, markers present ⇒ `pixi install --frozen --offline`
  accepted it in **~0.03 s** ✅.
* The same environment with the `conda-meta` record missing ⇒ `pixi install
  --frozen --offline` **fails** (it tries to do real work with no network) ✅ ⇒ D5.
* Restoring an environment whose fingerprint does not match the branch's manifest value:
  pixi still uses the prefix, so the manifest value is what an operator should compare ⚠️.

## §5 Sharding

* Default split limit **95 MiB**; a split blob is stored as siblings `<file>.part000`,
  `<file>.part001`, … and the manifest records each part's `size` + `sha256` and the
  reassembled blob's own `sha256`/`size` ✅.
* The core crate and the prototype were pointed at the same synthetic transport (two env
  blobs, one of them split into parts, one tool, one vendored file): both report
  **4 blobs / 14.0 MiB, 0 failures**, and both catch a tampered `.partNNN` as a size mismatch ✅.
  That cross-check paid for itself immediately — the first `verify` implementation reported a
  split blob as *missing*, because the whole file is (correctly) not on the branch; the
  regression is now covered by `a_good_transport_verifies`.
* Whole-file sharding is what preserves dedup: two environments share most `.conda` files
  byte-for-byte; fixed-size chunking of the same trees produced **+3.96 MiB per crate bump**
  in the vendor experiment compared with **+0.07 MiB** for the loose tree ✅ — chunk boundaries
  do not survive an edit any more than tarballs do.

## §6 Tool acquisition (canonical tool-pin catalogue, `--fetch-tools`)

* Pins verified for 5 platforms: `pixi-pack` and `pixi-unpack` for linux-64, linux-aarch64,
  osx-64, osx-arm64, win-64 (10 assets), plus `pixi` 0.81.0 linux-64.
* With `PATH` stripped to a bare directory, `pack --fetch-tools` downloaded, verified sha256,
  cached in `$PIXI_SANDBOX_TOOLS_CACHE`, executed `--version` to confirm the pinned version,
  and used the binary ✅ — i.e. the "no manual installs" scenario holds.
* The wrong binary is worse than a missing one: `~/.pixi/bin/pixi-unpack` is a 766 KiB
  trampoline that execs a dynamically linked binary inside its own prefix. Restore with it
  "succeeded" on the build machine and failed on the airlock with a missing
  `trampoline_configuration` ✅ ⇒ the ELF `PT_INTERP` check in `verify.rs` (D4).
* conda-forge availability checked for the dev tooling: rust 1.98.1, cargo-nextest,
  cargo-llvm-cov, cargo-deny, actionlint, lefthook, bun, biome, nodejs, rattler-index and
  taplo all exist ✅; `convoco` returns 404 ✅ (see `.convoco/README.md`).

## §7 Cargo vendoring

| measurement | result ✅ |
| --- | --- |
| `cargo vendor --locked --versioned-dirs` for the project | 33 crates → 35 MB / 1441 files |
| git cost per crate bump: loose vs one archive | +0.07 MiB vs +3.96 MiB |
| clone of the vendored tree: loose vs tarball | 390 ms vs 59 ms |
| airlock debug build from the vendor tree, network severed | 13–18 s |
| registry-cache fallback (instead of vendoring) | 9.7 MB, but only covers crates.io and cannot be verified per file |
| tarball-mode rebuild | reconstructed tree is **byte-identical** to the loose tree |

Config resolution matrix (`[source.crates-io] replace-with = "vendored-sources"`):

| where the config lives | relative `directory` resolves against |
| --- | --- |
| `<project>/.cargo/config.toml` | the project root ✅ |
| `--config <file>` on the command line | the cwd ✅ |
| `$CARGO_HOME/config.toml` | the parent of `CARGO_HOME` ✅ |

With the replacement in place, a build on a dead network works **even without `--offline`** ✅;
we still export `CARGO_NET_OFFLINE=true` for fail-fast. `cargo vendor` hard-fails ("found
duplicate version of package X vendored from two sources") when a crate+version is reachable
from two sources (crates.io and a git dependency) ⚠️ upstream limitation — the packer must
fail at pack time, never let the airlock discover it.

## §8 Restore work dir and the `/tmp` trap

* This container's `/tmp` is a **993 MB tmpfs**; `pixi-unpack` stages the whole pack into
  `$TMPDIR` before extracting ⇒ a restore of a ~500 MB environment failed mid-unpack ✅.
* Fix, now the default: work dir `<project>/.pixi/.restore-work`, `TMPDIR`/`TMP`/`TEMP`
  redirected for the child process, a free-space preflight (`packed + unpacked + vendor`), and
  a same-filesystem rename of the finished environment into `.pixi/envs/<env>` ✅.
* `restore --work-dir` and `unpack --work-dir` override the default; none of these paths ever
  write into the fetched branch ✅.

## §9 Vendor end-to-end (airlock4)

An airlock directory was restored entirely from a branch and then proven by building:

| item | value ✅ |
| --- | --- |
| payload | 263.9 MB total — envs 51 %, tools 36 %, vendor 13 % |
| branch size (git) | 110 MB |
| restore: verify + install | all blobs verified; no network |
| `cargo build` (debug) with network severed | **17.99 s**, empty `CARGO_HOME` |
| tampering with one vendored file | restore aborts on integrity mismatch |

## §10 The scaffold: executed gates and the packaged path

Every gate below was executed locally on the scaffold (pixi 0.81.0, rust 1.98.1):

| gate | result ✅ |
| --- | --- |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo test --workspace` | 28 pass (cli 9, manifest 6, shard 7, verify 6) |
| `cargo deny check` | advisories/bans/licenses/sources ok |
| `actionlint .github/workflows/*.yml` | clean |
| `taplo fmt --check` (pixi.toml, crates/pixi-sandbox/pixi.toml, deny.toml) | clean |
| `biome ci docs` | clean |
| `lefthook validate` (throwaway git copy) | "All good" |
| `pixi lock` re-run | byte-identical |
| `pixi run -e docs docs-build` | 5 pages in 4.31 s |
| `bash .knowledge/research/reproduce.sh <out>` | **52 s** cold, all green |

The end-to-end proof re-runs from a bare checkout in **52 s** with identical numbers
(cold 62 s): fingerprints `eaca22bcfed5ef20` / `2c79f41f9afdf07c`, packed
67 346 020 B / 65 995 586 B, vendor 32 514 233 B — deterministic enough to gate CI.

Payload of that run: **262.0 MB** — envs 133.3 MB (51 %; dev 64.2 MiB/40 files, docs
62.9 MiB/28 files, unpacked 507.5 / 501.9 MiB), vendor 32.5 MB (12 %), tools 96.1 MB (37 %).

**The packaged path** (D8) was executed too:

```
pixi publish --path crates/pixi-sandbox --build-dir … --target-dir …
  → pixi-sandbox-0.1.0-ha35fb5c_0.conda   607,957 B, 34 s
  → --target-channel file://<dir>          indexed channel, no rattler-index needed
pixi global install -c <channel> -c conda-forge pixi-sandbox      → 0.1.0 installed
pixi sandbox --version / doctor --verify / tools list / unpack    → extension verbs work
pixi sandbox doctor --verify --branch-location <transport>
  → 1512 blob(s), 249.8 MiB checked, 0 failures, 1.28 s
```

Gotchas that cost time and are now part of the recipe: `[workspace] preview = ["pixi-build"]`
is mandatory; `[package.build.config] extra-args` must stay **empty** because the backend
already passes `--locked` (a second `--locked` aborts cargo); `pixi build` is deprecated in
favour of `pixi publish --path …`; and a `file://` channel needs `-c conda-forge` beside it or
the solve fails on `libgcc`.

## §11 Git-as-a-trait, fixtures, and CI one-liners

**The git crate** (`crates/pixi-sandbox-git`, decision D9):

| fact ✅ | value |
| --- | --- |
| tests | 11 (5 against the in-memory mock, 6 against real `git` + a local bare remote) |
| runtime | 0.33 s for all 11 — the mock tests do not touch a disk or a network |
| orphan proof | `RecordingRunner` shows `commit-tree <tree> -m …` with **no `-p`**, and `push --force <remote> refs/heads/<branch>:refs/heads/<branch>` |
| history proof | after two publishes, `git rev-list --count <branch>` = **1** on the bare remote |
| rejected push | `FakeGit::fail_next_push` → the remote keeps the previous tip, the error names remote + branch + reason |
| transport safety | after a publish the transport is byte-identical, has no `.git`, and no scratch directory survives |
| dry run | `publish --dry-run` prints the exact command list (`$ git … push --force …`) and leaves the transport unchanged |
| `remote_size` | 0 for a URL (no such query in the git wire protocol), a real object-store size for a local path |

The payload is **not copied** to publish: `GIT_DIR` points at a scratch repo inside the
transport, `GIT_INDEX_FILE` at a scratch index, `GIT_WORK_TREE` at the transport, committed
with plumbing — a 250 MiB transport is published by moving objects, not by duplicating it.

**The fixtures** (decision D10):

| fact ✅ | value |
| --- | --- |
| fixture transport | 15 files / 15 394 bytes on disk, 10 declared blobs, one split into 3 parts |
| fixture project | pixi.toml + pixi.lock (ripgrep 14.1.1, 8 conda records), Cargo.toml + Cargo.lock (11 crates), src; helper-tool pins now come from the CLI's embedded catalogue |
| vendoring the fixture crate | 6.8 MiB / 10 crates, `cargo vendor --locked --versioned-dirs` |
| **packing the fixture project without installing it** | `pixi-pack` resolved from `pixi.lock`: **485 blobs, 100.6 MiB, 2.2 s** — so pack-integration tests need network but not a multi-GiB local install |
| test suite | 49 tests, 0 failed (cli 14, fixtures 5, git 11, core: shard 7 / manifest 6 / verify 6) |
| suite runtime | ~7 s cold, ~1 s warm, no network |

Two tests *are* the policy: the fixture must not depend on `pixi-pack`/`pixi-unpack`, and no
test may walk out of its crate from `CARGO_MANIFEST_DIR`. Both failed on their first run —
the first because the fixture's `pixi.toml` *explains* why the packers are absent (fixed by
checking dependencies, not comments), the second because the checker flagged its own source
(fixed by forbidding only `".."`/`.parent()` escapes).

**The CI one-liner rule** (design.md §6) — verified with pixi 0.81.0:

| fact ✅ | value |
| --- | --- |
| pixi task variables | plain `$VAR` expands; `${VAR:-default}` does **not** (it is split on the colon) — hence `SANDBOX_*` read from the environment, and literal defaults in the local tasks |
| a comma inside a variable's value | survives (`--envs dev,docs` via `$SANDBOX_ENVS`) |
| the workflow's three core commands, run by hand | `ci-doctor` on the fixture (10 blobs OK), `ci-publish` to a throwaway bare remote (15 files, `rev-list --count` = 1), `ci-pack` on the fixture project (485 blobs / 100.6 MiB / 2.2 s) |
| `actionlint` on all three workflows | clean |
| the reusable workflow | pack → verify → publish on one runner; the payload never becomes a build artifact |

## §12 Current Rust-port validation (2026-09-20)

Sections above retain the original prototype measurements. This later run validates the Rust
port against the current lockfile and records the values that should be used for its CI smoke.
The historical airlock assertions below ran under `unshare -rn` on linux-64, so the restore and
build processes had no network interfaces. The later source-only gates named explicitly in the
table validate code/configuration but do not constitute a new multi-platform airlock proof.

| gate | result ✅ |
| --- | --- |
| current source gates (2026-09-20) | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo deny check` (only transitive duplicate-version warnings), `taplo fmt --check`, Python compilation, and `biome check docs` pass |
| `cargo test --workspace --all-targets` | **64/64** Rust tests pass, including embedded pins, config planning, and Windows `.exe` transport naming |
| `pixi run -e dev test-doc` | pass |
| action manifests/workflows | YAML parses; the source-archive-safe `scripts/lint_actions.py` temporary-worktree wrapper makes `actionlint` pass despite the intentional absence of `.git`; 13 hermetic stdlib tests cover local release API/checksum verification, bad bytes, ambiguous checksum rejection, release tag/target/repository/asset-name and output-file input confinement, exact verified-binary invocation despite a PATH shadow, deterministic checksum generation, and immutable external `uses:` policy |
| `pixi run -e dev docs-build` | 6 static pages built successfully |
| `pixi lock --check` | lockfile already up to date; no lockfile regeneration performed |
| `pixi run -e dev sandbox-proof` | **177 s**, Rust pack/doctor/publish plus portable bootstrap restore, offline Pixi install, and offline Cargo build all pass |
| direct Rust `restore` against that branch | full environments + 168-crate loose vendor tree restore, then offline Pixi install and Cargo build, all pass |

The proof transport contained **10,621 verified blobs / 931.7 MiB**: dev (61 packed files,
471.8 MiB packed → 1,948.3 MiB unpacked), docs (40 files, 85.4 MiB → 333.6 MiB), pinned static
tools (91.7 MiB), and 168 vendored crates (10,517 files / 282.8 MiB). Publishing produced a
single orphan commit with 10,625 files; the local bare remote stored 533.9 MiB after Git
deduplication. Both restore implementations first verified every declared byte before writing,
then passed `pixi install --frozen --offline` and `cargo build --offline` from a fresh project.
