# Decisions

Fourteen load-bearing decisions. Each is referenced by ID from code comments and from
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
environments in the work dir and renames them in. A staged environment points at the staging
path until the moment before the rename, so every valid-UTF-8, NUL-free text file has that path
replaced by the final prefix first; a NUL byte means a fixed-width binary payload, which is
left exactly as it is. Once everything is in place the scratch this restore created is removed,
and a failed restore keeps it.

**Why.** A half-written environment is worse than a failed restore: pixi and cargo will happily
use a prefix that is 99 % correct and fail hours later in a way nobody connects to the
transport. Failure must be loud, complete and non-destructive. The same reasoning runs backwards
for scratch: an environment that still names the staging path works only until the work dir goes
away, so a leftover `.restore-work` is a silent time bomb (#18), while leftover scratch from a
*failed* restore is the evidence an operator needs.

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

**Why.** This repository is the tool's own development environment — `default` contains the
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

---

## D12 — The conda pack/unpack subprocesses stay (rattler is not v2 work yet)

**Decision.** `pixi-pack` and `pixi-unpack` remain external, sha256-pinned static release assets
driven as subprocesses (D2, D3). `rattler` is **not** linked into `pixi-sandbox`. This is a
recorded **no-go** on backlog task-9's v2 spike, not a rejection forever — see the four triggers
below.

**Why.** The measurable prize is small and the measurable costs are not. Removing the
`pixi-unpack` subprocess drops one blob and ~1.8 % of transport bytes, but the code that replaces
it is itself roughly the size of the binary it removes, drags a networking/async crate tree into
*this* repository's own vendored payload, and makes prefix installation — placeholder rewriting,
`conda-meta`, activation scripts — our correctness problem on a machine with no network and no
second chance. `pixi` itself (the majority of the tools bytes) has to stay regardless, so no
version of this changes the payload's category.

**Evidence.** Measured on this repository's own `sandbox/developer-linux-64` transport, restored
and verified offline on 2026-09-29 (see `rattler-spike.md` for the full workup):
`pixi-unpack` is **15.0 MiB / 1 blob of the 826.0 MiB, 10 313-blob payload (1.82 % / 0.01 %)**;
`pixi` is 76.6 MiB (81 % of the tools bytes) and cannot be removed; this repo's vendored tree
costs **1.79 MiB of transport per crate**, so adding rattler's ~28-crate tree grows the payload
by the opposite sign and a larger magnitude than the 15 MiB saved; `pixi-unpack` is already a
rattler front-end, making its static musl binary the best empirical proxy for what linking rattler
in would cost. Of the four things a v2 would genuinely buy, only peak restore disk (2 876.8 MiB of
scratch for a 1 867.5 MiB environment) is a categorical win, and only option C (replace the
*install* step alone) targets it.

**What would change it.** The four triggers in `rattler-spike.md` §5: (1) `pixi-unpack` stops
shipping static assets for a tier-1 platform or a pin becomes unmaintainable; (2) peak restore
disk blocks a real airlock — then prototype **option C only**, measured against today's restore;
(3) rattler publishes a supported "install this local channel into this prefix" entry point that
already owns placeholder/`conda-meta` handling; (4) the pack format gains a versioned
specification. Any prototype is measured, not argued: binary size for all five targets,
`cargo deny` delta, cold `cargo check` time, restore wall clock, peak disk, and a byte-for-byte
comparison of the installed prefix against a `pixi-unpack` restore of the same transport.

## D13 — A restored project is verified against a per-file oracle, not against its shape

**Decision.** Schema 2 manifests carry, per environment, a `files.json` oracle: the file list
of the *unpacked* prefix (relative path, sha256 of canonicalised content, exec bit, symlink
target; conda-meta records presence-only), recorded by `pack` from its own verification
unpack. `doctor --verify-restored <PROJECT>` compares a restored project against that list —
every entry, every extra file, the exec bits, the fingerprint marker — collects **all**
mismatches, and writes nothing. The airlock workflow's archived `ci`-feature e2e gate calls it
against the restored tree, and `restore.sh` calls it before cleaning up the worktree. A schema-1
env is reported `unverifiable`, never failed: published branches outlive the tool that packed
them.

**Why.** Everything the pipeline verified before task-10 was about the *branch*: blobs matched
the manifest, and the tree restore produced was checked only by shape — conda-meta present,
`pixi install` a no-op. A stub prefix with one hand-forged `conda-meta/*.json` record passed
all of it (measured against a real transport while designing this). Self-sufficiency and
integrity are different claims with different owners: the gate proves the sandbox needs no
network; only a per-file comparison of the restored tree against what the packer actually
unpacked can prove the tree is the one the manifest describes. The oracle closes exactly that
gap, and it travels in the branch — verified like every other blob — so the checker and the
digests it checks cannot disagree about what "correct" is.

**The canonicalisation rule.** Relocation rewrites the staging prefix into text files, but
NUL-padded binaries keep the staging path forever, and conda-meta records embed digests of
files relocation changed. So a digest is taken over a canonical form: collapse NUL runs, then
replace *this side's* path spellings with a sentinel. Pack's candidates are its scratch paths;
verify's candidates are the final prefix and the restore work dir — derived from the project
path, env name and work dir alone, never from files restore wrote (a candidate list taken from
restore output would let a malicious restore name its own scratch as "correct"). With that,
the same digest is stable across relocation without weakening the check: a prefix still
pointing into restore scratch fails it.

**Honest limits.** conda-meta records are presence-only (their bodies embed
`sha256_in_prefix` of relocated files — recording their content would fail every honest
restore); `conda-meta/pixi`, `conda-meta/history` and the two restore markers are exempt as
pixi/restore bookkeeping; and a malicious writer who rewrites payload *and* oracle and
manifest consistently is still outside the model (no signatures yet, §7). The oracle costs one
verification unpack per env at pack time (≈8.5 s and ~2 GiB of scratch, measured on the real
transport) and a 1895-byte fixture blob.

---

## D14 — The standalone bootstrap is single and the helpers stay (task-24's measured decision)

**Decision.** `pixi-sandbox` is the standalone transport/restore orchestrator and the *single*
transport bootstrap: the verified binary travels exactly once, under
`.pixi-sandbox/tools/<platform>/`, and every launcher path (generated `restore.sh`/`restore.ps1`,
`scripts/restore.sh`, the bare-binary bootstrap, and the user-PATH registration) executes that
manifest-verified copy — never a `PATH`-discovered or environment-embedded one. `pixi-pack` and
`pixi-unpack` are not absorbed (D2/D3/D4 stand). The transport schema stays at 2 under the
compatibility policy recorded in backlog decision-2 (readers accept 1..=N and refuse newer;
additive `#[serde(default)]` fields may land without a bump; removals/semantic changes are a
bump that must keep reading every older schema while published branches exist). An environment
that depends on the `pixi-sandbox` package is payload, restored verbatim; `pack` warns about the
duplication instead of refusing; promoting the environment's copy to the bootstrap is rejected.

**Why.** One canonical executable per transport is what makes the launchers and the user-PATH
registration honest: the bytes they run are the bytes the manifest verified. Absorbing the
unpacker is a bad trade measured end to end, and the schema rule is what lets a published branch
outlive the binary that packed it.

**Evidence ✅.** Measured 2026-10-01 on the published `sandbox/developer-linux-64` branch
(backlog doc-7): `pixi-unpack` costs 6.5 MiB **in-pack** of a 503.9 MiB branch (1.3 %) — the
"15 MiB prize" is the file size, not the branch cost; `pixi` (32.9 MiB in-pack, 6.5 %) cannot be
removed; absorbing the unpacker grows the binary by roughly what it saves (proxy: `pixi-unpack`
is a 15.0 MiB static rattler front-end) and adds ~28 rattler crates to this repo's own vendored
payload at 1.79 MiB raw / 0.17 MiB in-pack per crate; measured peak restore disk 2 746 MiB
project / 4 098 MiB combined for a 1 830 MiB environment in 16 s — not a reported blocker. The
bootstrap copy itself costs 1.7 MiB in-pack (0.34 %), so double-shipping it is cheap but
pointless; git content-dedups the vendor tree (10 254 declared files → 9 663 objects,
278.1 MiB declared → 142.2 MiB raw → 25.8 MiB in-pack) while restore still materialises every
declared file.

**What would change it.** The four triggers of doc-5 §5 (an unmaintained `pixi-unpack` pin, a
real airlock blocked by peak restore disk — then prototype option C only, a supported rattler
"install this local channel into this prefix" entry point, a versioned pack format), plus one
new trigger: a real airlock blocked by *combined* disk (git store + checkout + restore), where
doc-7's option C′ (restore from the git object store, no checkout; −845 MiB measured) is the
first lever because it changes no binary and no format.

## D15 — Turbo is deferred until the JS workspace graph earns it (task-26's threshold)

**Decision.** No Turbo today. The bun workspace has one JS package (`docs`), no cross-package
JS edge, and a ~27 s docs CI build job; Pixi already expresses the only task graph that exists
(`docs-install → docs-build`, `docs-install → lint-docs`, reused by the pre-commit hook as
`lint-docs-write`). Recorded as backlog decision-3 (deferred), evaluation in backlog doc-8.

**Adoption threshold (all three, doc-8 §2).** (1) ≥ 3 JS packages in the bun workspace —
two packages still fit two pixi tasks; (2) a real cross-package edge — some package imports
another workspace package; (3) ≥ ~60 s of repeated JS work on the CI path a cache would
prune, or a local feedback loop past ~10 s for unchanged packages. Any PR adding a second or
third JS package re-checks the triggers; a hit reopens the decision rather than silently
accumulating pixi task duplication.

**If adopted.** Turbo is a root `package.json` devDependency pinned through `bun.lock` (like
`@biomejs/biome`), never a conda/pixi dependency — the sandbox branch's payload budget is not
spent on a CI-only tool. Invoked as `bun x turbo …` inside the `web` environment; pixi
per-package tasks become thin `bun x turbo <task>` facades with `turbo.json` mirroring the
pixi task names. Local `.turbo` cache for developers; published builds run `--force`; CI
caches `.turbo` keyed on `bun.lock` + OS; **remote caching stays off** (third-party cache
round-trips contradict the airlock posture — flipping it on is a new decision, not a config
change).

**Why.** With one package there is nothing to parallelise, deduplicate, or prune; Turbo would
add a dependency and a second place where "what runs when" is expressed, in exchange for
caching a build that already finishes faster than the runner takes to boot.

**Evidence ✅.** Measured 2026-10-01 (doc-8 §1): `package.json` workspaces = `["docs"]`;
docs workflow build job 27 s wall on `ubuntu-latest` including runner bootstrap (run
36879920379); recent full runs ~30–80 s including the Pages deploy.

**What would change it.** Meeting the §2 threshold, or committing to the docs site as the
only JS package forever — then close as rejected instead of deferred.
