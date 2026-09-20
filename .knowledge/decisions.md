# Decisions

Eleven load-bearing decisions. Each is referenced by ID from code comments and from
`design.md`. If you disagree with one, bring a measurement — the numbers behind each are in
`research/EVIDENCE.md`.

Format: **decision** · why · evidence · what would change it.

---

## D1 — The transport is a git orphan branch on the project's own remote

**Decision.** `pack` writes a directory; `publish` puts it on an orphan branch
(`sandbox/<envs>-<platform>`) as a single force-pushed commit; the airlock `git fetch`es that
branch and never merges it.

**Why.** No new infrastructure (no registry, no artifact server, no credentials beyond the
remote the project already uses), the airlock already knows how to clone, and git gives us
content-addressed dedup for free.

**Evidence ✅.** Two environments packed separately share ~2× of their bytes in git; a new 5 MB
package adds ≈ +4.8 MiB to the branch; `git gc --aggressive` after a force-push does *not*
shrink an orphan repo, so history growth must be managed by rotating branches, not by
force-pushing.

**What would change it.** A project whose remote forbids force-push, or a payload > 1 GB per
platform set (GitHub's practical repo budget) would push us toward one branch per env, or to a
release-asset/artifact channel for the payload with the branch kept as the manifest.

---

## D2 — `pixi-pack --directory-only` is the packer

**Decision.** Environments are packed with `pixi-pack` (a static release asset, see D4) using
`--directory-only`, one directory per env under `.pixi-sandbox/envs/<env>/pack/`.

**Why.** It produces exactly the transportable unit: the environment's raw `.conda` files as a
local channel plus `environment.yml` + `pixi-pack.json`. Directory mode keeps file
granularity, which is what git dedup and our sharding need. Re-implementing this on top of
`rattler` is possible but is not v1 work.

**Evidence ✅.** `--directory-only` output for a real environment is ≈3.7× smaller than the
same environment as a prefix tar (which is mostly hardlink-flattened duplicates); 5567 prefix
files vs a few dozen `.conda` blobs; the tar form also loses the per-file digests we verify.
`pixi-pack`/`pixi-unpack` are **not on crates.io** ✅, so using them means shipping binaries
(D4), not linking them.

**What would change it.** A `rattler`-based packer that is byte-compatible with what
`pixi-unpack` expects would remove the subprocess and let us drop the external tools
(the ⚠️ v2 path in `design.md` §1).

---

## D3 — `pixi-unpack` is shipped, not assumed

**Decision.** The branch carries `pixi-unpack` (same static asset as D4) and `restore` drives
it; `unpack` accepts `--unpacker` to override it.

**Why.** It is the only supported way to turn a pack back into a prefix. Without it the airlock
cannot install anything, and "install pixi-unpack first" would violate the no-manual-steps
rule.

**Evidence ✅.** `pixi-unpack <pack> -o <stage> -e <env>` is the documented pairing for
`pixi-pack` output; it stages into `$TMPDIR` (the trap in `design.md` §4), and the static musl
build runs on the airlock with no loader present.

**What would change it.** Shipping our own unpacker built into `pixi-sandbox` (v2), or an
upstream format guarantee that would let us treat a pack as a plain conda channel install.

---

## D4 — Helper tools are pinned release assets, compiled into the CLI by default

**Decision.** The canonical `crates/pixi-sandbox-core/assets/tools.lock.json` pins version + URL
template + sha256 + linkage per platform and is compiled into every `pixi-sandbox` release.
`pack --fetch-tools` (and release-driven CI) download → verify → cache → execute without a
sidecar file. `--tools-lock PATH` is an explicit, reviewable override for an organisation mirror
or emergency pin change. Nothing is installed system-wide and nothing is taken from `PATH`
unless the caller opts out. Linux embedded tools must be `static`; the packer and `doctor` refuse
ELF `dynamic` ones. Native macOS/Windows binaries are classified as `system` and require their
own platform proof.

**Why.** An installed CLI must remain usable without a loose JSON file beside it, while the
airlock cannot install anything. A tool discovered on `PATH` makes the build machine and the
airlock differ in ways nobody can see. Data embedded at compile time keeps defaults reproducible;
an override keeps private mirrors and urgent security response possible.

**Evidence ✅.** The `~/.pixi/bin` shims that `pixi global install` writes are ~766 KiB
trampolines that exec a dynamically linked binary inside their own prefix: they restore
perfectly and then fail on the airlock with a missing `trampoline_configuration`. The static
musl assets (14.7 MiB `pixi-pack`, 15.7 MiB `pixi-unpack` v0.7.11) run anywhere; a
PATH-stripped run of `--fetch-tools` (download → sha256 → cache → execute) was verified with
the cache empty.

**What would change it.** A project that requires a tool version per repository can use the
explicit override; upstream publishing a static asset for a new platform requires a reviewed
embedded catalogue update and a new CLI release. `tools update` remains the future helper for
preparing that review diff.

---

## D5 — A restored prefix is not a pixi environment until the markers are written

**Decision.** After unpacking, `restore` writes `<prefix>/conda-meta/pixi_env_prefix` and
`<prefix>/conda-meta/.pixi-environment-fingerprint` (when the manifest recorded one).
`unpack` deliberately does *not* — it produces a prefix, not an environment.

**Why.** pixi validates the prefix through those markers before it will trust the environment
on disk and skip solving; without them `pixi install --frozen --offline` tries to do real
work.

**Evidence ✅.** A hand-placed, otherwise-correct environment is accepted by
`pixi install --frozen --offline` in ~0.03 s *with* the markers, and fails (offline) *without*
the `conda-meta` record. The fingerprint is pixi's own value: 16 bytes of xxh3 over the
`(name, sha256)` records of the lock ❓ it identifies the lockfile, and therefore cannot detect
a tampered prefix — that is `sha256` per blob's job.

**What would change it.** pixi changing either file name or its fingerprint algorithm (the
manifest stores the value, so a change is a re-pack, not a schema break).

---

## D6 — Vendored cargo crates live in `.pixi-sandbox/vendor/`

**Decision.** `pack --cargo-vendor` writes `cargo vendor` output to
`.pixi-sandbox/vendor/` — a sibling of `envs/` and `tools/`, not a child of an env — records
every vendored file as a manifest blob, and `restore` materialises it to
`<project>/.pixi-sandbox/vendor/` and wires `.cargo/config.toml` (relative `directory`).
Default mode `loose`; `tarballs` available.

**Why.** Crates are project-scoped, not env-scoped (one `Cargo.lock`); `envs/<env>/pack` must
stay byte-for-byte what `pixi-pack` produced so it can be re-packed and compared; blob
accounting and `doctor` iterate environments and would lie if a crate tree hid inside one; a
partial `restore --envs dev` still needs the crates; and the branch layout then mirrors where
the tree lands in the project. Full reasoning: `design.md` §11.

**Evidence ✅.** 33 crates ≈ 35 MB / 1441 files loose; per-crate-bump git cost +0.07 MiB vs
+3.96 MiB for a single archive; clone of the vendored tree 390 ms (loose) vs 59 ms (tarball);
a severed-network debug build with an empty `CARGO_HOME` succeeds in 13–18 s; a registry-cache
fallback (9.7 MB) is possible but cannot be verified the same way.

**What would change it.** A project whose crates change on every commit (tarball mode), or
cargo gaining a first-class "vendor into a prefix" mode that resolves paths itself.

---

## D7 — Verify before write; a join is atomic; never a half-populated environment

**Decision.** No file reaches the user's tree before its sha256 matches the manifest.
`join_parts` writes a sibling temp file, verifies the parts *and* the reassembled blob, and
renames into place only then; on any failure the temp file is removed. `restore` stages whole
environments in the work dir and renames them in.

**Why.** A half-written environment is worse than a failed restore: pixi and cargo will happily
use a prefix that is 99 % correct and fail hours later in a way nobody connects to the
transport. Failure must be loud, complete and non-destructive.

**Evidence ✅.** The property test plus two concrete bugs caught by it (parts resolved relative
to the destination; a half-written destination left behind after a corrupted part) — both are
now covered by tests in `crates/pixi-sandbox-core/tests/shard.rs` and by the tamper case in
`research/EVIDENCE.md` §4.

**What would change it.** Nothing we can think of; this is the invariant the whole tool exists
to provide.

---

## D8 — The tool itself ships as a conda package

**Decision.** `crates/pixi-sandbox/pixi.toml` builds `pixi-sandbox` with the
`pixi-build-rust` backend; releases are cut with `pixi publish --path crates/pixi-sandbox
--target-channel <dir>` and installed with
`pixi global install -c <channel> -c conda-forge pixi-sandbox`, after which the verbs are
available as `pixi sandbox <verb>`.

**Why.** It is the same distribution channel the users already have (pixi), it works on an
airlock (the `.conda` is a file you can carry), and it needs no Rust toolchain on the machine
that installs it.

**Evidence ✅.** Verified end-to-end with pixi 0.81.0: `pixi publish` produced
`pixi-sandbox-0.1.0-ha35fb5c_0.conda` (607,957 B, 34 s) ✅; pointing
`--target-channel file://…` at a directory produced an indexed channel without
`rattler-index`; `pixi global install -c file://… -c conda-forge pixi-sandbox` installed 0.1.0
and `pixi sandbox doctor --verify` then checked 1512 blobs / 249.8 MiB in 1.28 s ✅. Gotchas
recorded in `research/EVIDENCE.md` §10: `[workspace] preview = ["pixi-build"]` is required,
`build.config.extra-args` must stay empty (the backend already passes `--locked`), and a
`file://` channel needs `-c conda-forge` alongside it or the solve fails on `libgcc` ✅.

**What would change it.** Publishing to conda-forge itself (needs a feedstock and a review),
or GitHub-Pages-hosted channels becoming the preferred distribution (that is an **open**
question in `research-results.md`, not a decision yet).

---

## D9 — Git access goes through a trait (`pixi-sandbox-git`)

**Decision.** `publish` and the airlock's fetch are expressed as `GitProtocol` operations
(`publish`, `fetch_into`, `branch_exists`, `remote_size`). `ShellGit` implements them with a
real `git` through a `Runner` (`ProcessRunner`, `RecordingRunner`, `PreviewRunner`); `FakeGit`
implements them in memory. Nothing else in the codebase runs `git`.

**Why.** The two operations that touch a remote are exactly the ones a test cannot have and
the ones that are dangerous to run for real (a force-push to a repository that might be
somebody's). A trait makes both testable: the flow's contract — one parentless commit, history
replaced, snapshot read-only, byte-exact fetch — is asserted against a mock *and* against real
git, and `--dry-run` becomes "the same code path with a runner that records instead of
executing" rather than a second implementation that drifts.

**Evidence ✅.** 11 tests in `crates/pixi-sandbox-git/tests/publish.rs`: the mock round-trips
publish → fetch byte for byte, replaces history instead of appending, and keeps the remote
unchanged when a push is rejected; the shell implementation does the same against a local bare
remote, and `RecordingRunner` proves `commit-tree` is called without `-p` (orphan) and `push
--force` is called (a replacement). `publish --dry-run` leaves the transport byte-identical
(asserted in the CLI tests).

**What would change it.** A git library (gix) as a second implementation behind the same
trait, if shelling out ever becomes the bottleneck or the portability problem.

---

## D10 — Tests use fixtures, never this repository

**Decision.** Project-level tests operate on `crates/pixi-sandbox/tests/fixtures/demo-project`
(a complete pixi project definition with no packer in its environment) and payload-level tests
on `tests/fixtures/transport` (a committed synthetic transport with real digests, including a
split blob). Two tests enforce the rule: the fixture must not depend on `pixi-pack`/`pixi-unpack`,
and no test may walk out of its crate from `CARGO_MANIFEST_DIR`.

**Why.** This repository is the tool's own development environment — `dev` contains the
packer, the lockfile resolves hundreds of MiB — so a test pointed at the repository root tests
the machine, not the code: it passes where a fresh clone would fail, it is slow, it mutates
state, and a bug that only appears on a plain project would never surface. The payload fixture
also makes the verification path (including `.partNNN` reassembly) hermetic and instant
instead of needing a 95 MiB payload.

**Evidence ✅.** 5 policy tests plus 14 CLI tests run against the fixtures in milliseconds;
one of them caught a real `verify` bug (a split blob reported as *missing*). Measured bonus:
the fixture project packs **without installing an environment** — `pixi-pack` resolves from
`pixi.lock` — so pack-integration tests need network but not a multi-GiB local install
(485 blobs / 100.6 MiB / 2.2 s).

**What would change it.** Nothing we can think of; a test that cannot use a fixture should say
so in a comment and be `#[ignore]`d.

---

## D11 — Published sandbox branches come from an explicit native matrix

**Decision.** A project declares publishable environment bundles in `.pixi-sandbox.toml`.
`pixi-sandbox plan --json` validates that declaration and emits one job per
(bundle × platform). The reusable `publish-sandboxes.yml` workflow downloads a
checksum-verified standalone `pixi-sandbox` release and runs each job on its matching native
runner; it never silently cross-packs a platform.

**Why.** A Pixi manifest commonly contains environments that are experimental, CI-only, or not
supported on every platform. Packing "all environments" is an unsafe implicit publish policy.
One GitHub Actions job has one OS/architecture, so a custom action cannot honestly create and
airlock-test a Mac or Windows payload from an unrelated runner. Explicit bundles, branch names,
and runners make that reviewable.

**Evidence.** The Linux real transport path is proven under a network namespace. macOS and
Windows are not yet proven, so the planner requires a named native runner and complete compiled
helper pins; the publish action compares its detected platform with the requested platform before
any bytes are packed.

**What would change it.** A measured, supported cross-packing implementation for
`pixi-pack`/`pixi-unpack` plus equivalent target execution tests could relax the native-runner
constraint. Until then, an action that loops platform labels on one runner is only pretending to
provide multi-platform sandboxes.
