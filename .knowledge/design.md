# Design

**Status:** settled and measured. The Rust CLI in `crates/` implements the connected-side
`pack` flow and the airlock-side `unpack`/`restore` flow; static Rust release artifacts are
now the primary bootstrap, with `action.yml` providing verified download (like `setup-pixi`).
**Scope:** how pixi environments (and, optionally, vendored cargo crates and the toolchain
itself) get from a connected machine to one that has no network.

Conventions: `✅` = measured in the lab (numbers in `research/EVIDENCE.md`), `⚠️` = reasoned
but not yet verified. Decision IDs (`D4`) point at `decisions.md`; the lab was Debian 13,
2 vCPU, pixi 0.81.0, pixi-pack/pixi-unpack 0.7.11, cargo/rustc 1.98.1, linux-64.

---

## §1 The problem and the constraints

The target machine is *airlocked*: no network, no package manager it is allowed to reach, and
an operator (often an agent, often with no context) who must get a working development
environment out of whatever arrived on a USB stick or in a git clone.

Hard constraints, all of which shaped the design:

| constraint | source | consequence |
| --- | --- | --- |
| git blobs > 100 MiB are rejected by GitHub (warn > 50 MiB) | GitHub docs ✅ | shard at 95 MiB (§3) |
| a repo is soft-limited to ~1 GB, a push to ~2 GB | GitHub docs ✅ | one branch per (platform × env-set), not per env; rotation (§2) |
| nothing may be fetched at restore time | the whole point | tools are embedded and static (D4) |
| `pixi install` on the airlock must not need the network | reproducibility claim | markers must be written correctly (D5) |
| `/tmp` can be a small tmpfs | measured here: 993 MB | work dir on the target filesystem (§4) |
| no new infrastructure (no registry, no cache server) | operational reality | the project's own git remote is the transport (D1) |
| the operator may be an agent | user scenario | branch carries a generated `AGENTS.md` (§3) |

### Questions this study answered

1. **Is `pixi-pack` needed?** Yes (D2). It produces exactly the transportable unit: the raw
   `.conda` files of an environment as a local channel, plus `environment.yml` and
   `pixi-pack.json`. Re-implementing that on top of `rattler` is possible but is not v1 work.
2. **Is `pixi-unpack` needed on the airlock?** Yes (D3). It is the only supported way to turn a
   pack back into a prefix, it stages through `$TMPDIR`, and it must therefore be *in* the
   branch.
3. **Should `pixi-pack`/`pixi-unpack` be compiled into `pixi-sandbox`?** Not in v1: neither is
   published on crates.io ✅ (they are git-only), and linking them would drag rattler/uv
   into every build. v1 embeds sha256-pinned release assets — static musl on Linux, native
   system executables on macOS/Windows that need separate proof (D4/D11); a `rattler`-based
   library implementation is the ⚠️ v2 path that would remove the subprocess entirely.
4. **What else is uncovered?** The airlock still needs: the *Rust toolchain* (vendoring does
   not carry `rustc`, §11), the pixi **markers** after a raw unpack (D5), and a
   **conda channel** for `pixi global install` to come from (§6). Everything else the
   payload carries.

### Branch layout

One orphan branch per **(platform × set of environments)**, e.g. `sandbox/dev-linux-64`,
`sandbox/dev+docs-osx-arm64`. Rationale: an airlock machine cares about one platform and
usually one project; putting several envs on one branch lets git dedup shared packages ✅
(measured ~2× across two envs), while mixing platforms would ship ~⅔ bytes nobody can run.

---

## §2 The transport is an orphan branch (D1)

An orphan branch has no history to merge, holds exactly one snapshot tree, and is force-pushed
on every publish. The airlock does:

```bash
git fetch origin sandbox/dev-linux-64:sandbox/dev-linux-64
git archive sandbox/dev-linux-64 | tar -x -C .pixi/branch   # or: git worktree/checkout
```

Two measured properties make this cheap:

* **Dedup is automatic.** Git content-addresses every file, so unchanged `.conda` files, the
  identical `environment.yml`, and — most importantly — the *vendored crate tree* are stored
  once no matter how many publishes a branch has ✅. A new 5 MB package costs ≈ +4.8 MiB.
* **Force-push does not reclaim space.** `git push --force` followed by `git gc --aggressive`
  on the server does **not** shrink an orphan repo ✅ — the objects are only unreferenced, and
  a server-side gc may keep them for a long time. Practical consequences:
  * publishing a fresh branch per platform-evset (rather than reslicing one branch) keeps
    history small;
  * rotation (`publish --keep N`) must *rebuild* history — i.e. push a new tree and let the
    operator/admin prune — instead of assuming a force-push is a shrink;
  * a branch that has grown past the repo budget is best replaced (new name, old one deleted
    and the repo gc'd with the administrator's help).

The branch is **read-only** for the airlock: nothing in the tooling ever writes into the
fetched checkout (AGENTS invariant 2). A restore stages everything in the *project*.

---

## §3 What a transport contains (pack)

```
<branch>/
├── .pixi-sandbox/
│   ├── manifest.json               schema 1 — the only source of truth
│   ├── envs/<env>/pack/            output of `pixi-pack --directory-only` (D2)
│   │   ├── channel/{noarch,linux-64}/*.conda
│   │   ├── environment.yml
│   │   └── pixi-pack.json
│   ├── tools/<platform>/           pixi · pixi-unpack · pixi-sandbox (sha256-verified; static on Linux)
│   └── vendor/                     cargo vendor output (D6)
├── README.md                       generated from the manifest
└── AGENTS.md                       generated from the manifest
```

**Packing** is `pixi-pack <project> -e <env> -p <platform> -o <pack-dir> --directory-only` ✅.
The directory form is what makes the transport addressable at file granularity; the tar form
(and `--create-executable`) is for single-file hand-offs, and a prefix tar is ≈3.7× larger
than the same environment as raw `.conda` files ✅ (`--directory-only` is therefore the
default, D2).

**Shards are whole files.** The only split that ever happens is a single file larger than
`shard_limit_bytes` (95 MiB), which becomes `<name>.partNNN` pieces; `doctor --verify`
reassembles them logically, `materialise()` reassembles them physically. Chunking *everything*
into fixed-size pieces would destroy git dedup (measured: +0.07 MiB vs +3.96 MiB per crate
bump ✅) and is explicitly rejected.

**`manifest.json` (schema 1)** — every field is load-bearing:

| field | meaning |
| --- | --- |
| `schema` | bumped only for incompatible changes; readers refuse anything newer |
| `tool`, `created_at` | provenance, informational only |
| `platform` | the conda subdir the payload targets (`linux-64`, …) |
| `shard_limit_bytes` | what the packer used; `doctor` reports files that were split |
| `source.commit`, `source.lock_sha256` | which commit and which `pixi.lock` produced this |
| `tools` | embedded tools: `version`, `url`, `pinned_sha256`, `linkage`, `size_bytes`, `path` |
| `envs.<name>` | `pack_path`, packed/unpacked sizes, `pixi_environment_fingerprint`, `blobs` |
| `envs.<name>.blobs[]` | every packed file: `path`, `size`, `sha256`, optional `parts[]` |
| `vendor` | `mode`, `crates`, `size_bytes`, `cargo_lock_sha256`, `directory`, `blobs[]` (D6) |

Rules the validator enforces: schema match, at least one env or tool, no empty envs, every
`path` relative and non-escaping, every digest a 64-hex sha256, and split parts summing to the
blob size. Every `path` in the manifest — env blobs *and* `tools.<name>.path` — is relative to
`.pixi-sandbox/`; `envs.<name>.pack_path` is relative to the branch root and informational
(it is where *pack* put the directory).

**`README.md` and `AGENTS.md` are generated, never hand-edited.** They carry the env table,
the payload split, the exact restore commands for this platform, the vendor summary and the
"`pixi install --frozen --offline` must be a no-op" acceptance rule. `README.md` is for the
human who opens the branch on GitHub; `AGENTS.md` is for the agent doing the restore.

**Pack also prints the payload split** (envs / tools / vendor, bytes and %) so CI logs show
where the bytes went; the same numbers appear in the generated `README.md`.

---

## §4 The airlock side: `restore` (and `unpack`)

`restore` is the verb the airlock runs. Its order is the contract (implementation checklist in
`crates/pixi-sandbox/src/commands/restore.rs`):

1. **Load + validate the manifest.** Unknown schema ⇒ refuse, do not guess.
2. **Verify everything, write nothing.** Every blob of every selected env, every vendor blob,
   every tool (size + pinned sha256), and the tool linkage (a `dynamic` tool aborts here —
   D4). `doctor --verify` is the same code path, and `restore --verify-only` stops after it.
3. **Materialise the tools** into `<project>/.pixi/tools/<platform>/`. Idempotent: an existing
   file whose sha256 matches the pin is skipped ✅ (this is what makes a re-restore cheap).
4. **Preflight disk space** on the *work dir's* filesystem (packed + unpacked + vendor,
   measured with headroom). Fail before writing rather than mid-unpack.
5. **Install each environment**: materialise its pack dir under the work dir, then
   `pixi-unpack <pack> -o <stage> -e <env>`, then rename `<stage>/<env>` into
   `<project>/.pixi/envs/<env>`. Same filesystem ⇒ rename, not copy.
   * `pixi-unpack` stages into `$TMPDIR` ✅ — the work dir defaults to
     `<project>/.pixi/.restore-work` and `TMPDIR`/`TMP`/`TEMP` are redirected for the child.
     A small `/tmp` (993 MB here) fails mid-unpack; this is not theoretical ✅.
6. **Write the pixi markers** (D5): `<prefix>/conda-meta/pixi_env_prefix` and, when the
   manifest recorded one, `<prefix>/conda-meta/.pixi-environment-fingerprint`. A raw prefix
   without them is rejected by pixi's fast path and `pixi install --frozen --offline` tries to
   do real work (which fails offline ✅).
7. **Materialise the vendored crates** into `<project>/.pixi-sandbox/vendor/` (same verified
   blob path as everything else) and wire `.cargo/config.toml` with a **relative** directory
   (D6, §11). `--cargo-config auto|write|print|none`.
8. **Finish with the two assertions the flow exists for:**
   `pixi install --frozen --offline` must be a no-op, and `cargo build --offline` must
   succeed on a severed network ✅.

`restore` may also run with `--envs dev` (partial restore). Note that ✳ the **vendor tree is
always materialised**: the crates are not per-environment, and a partial restore that leaves
`cargo build` broken would be worse than useless.

`unpack` is the single-environment primitive `restore` drives once per env, exposed because a
payload can legitimately arrive *outside* the branch flow (a single `pack` directory on a USB
stick, an artifact from another CI). It accepts a transport dir, a
`.pixi-sandbox/envs/<env>/pack` dir, or a bare pack dir; with a manifest it verifies and
materialises first, without one it treats the input as a bare pack and says so. It leaves the
pixi markers to `restore` on purpose (D5) — `unpack` produces a *prefix*, not a pixi
environment.

**What restore does *not* do:** it never writes into the fetched branch, never installs
anything system-wide, never touches the network, and never deletes a user's environment
without `--force`.

---

## §5 CLI surface

`pixi-sandbox <verb>` and — because pixi execs `pixi-<verb>` found on `PATH` ✅ — the same
verbs as `pixi sandbox <verb>`.

| verb | flags |
| --- | --- |
| `pack` | `--repo-root . --envs dev,docs --output-dir DIR [--platform linux-64] [--shard-limit-mib 95] [--cargo-vendor] [--cargo-vendor-mode loose\|tarballs] [--fetch-tools] [--tools-lock PATH] [--tools-cache DIR] [--self-bin PATH]` — no override means embedded pins |
| `publish` | `--input-dir DIR --branch-name NAME [--remote origin] [--keep N] [--dry-run]` |
| `restore` | `--branch-location DIR --output-path DIR [--envs a,b] [--verify-only] [--force] [--no-vendor] [--work-dir DIR] [--cargo-config auto\|write\|print\|none]` |
| `unpack` | `--input-dir X --output-dir PREFIX [--env NAME] [--unpacker PATH] [--force] [--work-dir DIR] [--verify-only]` |
| `doctor` | `--branch-location DIR [--verify] [--envs a,b] [--json]` |
| `plan` | `--config .pixi-sandbox.toml [--json]` — validate explicit bundle/platform publication and emit native jobs |
| `tools` | `list [--tools-lock F]` (embedded pins by default) · `update [--tools-lock F]` (⚠️ not implemented yet) |

This matches the original sketch, with two additions the study forced: `unpack` (a payload can
arrive outside the branch flow) and `doctor` (an airlock operator must be able to check a
transport without writing). `--verify-only` exists on `restore`/`unpack` for the same reason.

Conventions: every verb is idempotent where it writes; nothing needs `sudo`; exit code 1 on
any failure, with the failing path and the reason on stderr; `doctor --json` is the
machine-readable form for scripts and CI.

---

## §6 CI, docs, and the conda channel

* **`ci.yml`** — on PR/main: rust lint (`fmt`, `clippy -D warnings`, `cargo deny`), config lint
  (`actionlint`, `taplo`), docs lint (`biome`), tests (matrix), coverage (`cargo llvm-cov` →
  codecov), docs build, and an **airlock bootstrap proof**. Its script invokes the Rust
  connected-side pack/doctor/publish verbs, then restores through the Rust self-binary
  embedded in the branch.
* **`docs.yml`** — Astro + Starlight → GitHub Pages (limits that matter: 1 GB site, 10-minute
  build, bandwidth budget). The **payload never lives on Pages**: it is git history, which the
  airlock already knows how to fetch.
* **`publish-sandbox.yml`** — unified publisher (replaces `publish-sandbox` + `publish-sandboxes`):
  validates `.pixi-sandbox.toml` via `plan --json`, then for each native (bundle×platform)
  runs on native runner, uses composite actions `setup-pixi-sandbox` (verified download) and
  `publish-pixi-sandbox` (install, pack with `--self-bin` Rust binary, doctor, publish).
  Pack, verify and publish run on the *same* runner (payload never travels as artifact).
* **`release.yml`** — builds 5 tier-1 static binaries (musl Linux x86_64/aarch64, macOS x86_64/aarch64, Windows x86_64), strips, generates `SHA256SUMS`, creates GitHub Release. Contract matches `setup-pixi-sandbox` README.
### The one-liner rule (how the workflows are written)

**Source-driven GitHub Actions commands are one line: `pixi run -e <env> <task>`.** If a
source-driven step needs more than one line, it is a task in `pixi.toml` (or a script in
`.knowledge/research/`) — not inline bash in YAML. The release-driven publisher uses dependency-free composite-action shell (bash + pwsh) with checksum verification, native-runner, and token handling, not caller-inline shell. This is not
style for its own sake:

* the exact command CI runs is the command a developer runs, so gate failures reproduce
  locally without reading the workflow;
* the logic is diffable, reviewable and testable, and it does not depend on which shell the
  runner defaults to (`bash` on Linux, `pwsh` on Windows);
* a workflow file becomes a *list of gates*, which is what a reviewer should be reading.

Paths come from the environment, because pixi tasks expand `$VAR` but have no
`${VAR:-default}`: local convenience tasks carry literal defaults, and the `ci-*` variants
read `SANDBOX_PROJECT`, `SANDBOX_ENVS`, `SANDBOX_TRANSPORT`, `SANDBOX_PLATFORM`,
`SANDBOX_BRANCH`, `SANDBOX_REMOTE`, `SANDBOX_SELF_BIN`, `SANDBOX_CARGO_VENDOR` and
`SANDBOX_PROOF_OUT` — all set in the workflow's `env:` block. Example, in full:

```yaml
      - uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
      - uses: prefix-dev/setup-pixi@ba3bb36eb2066252b2363392b7739741bb777659 # v0.8.1
        with: { environments: ci, cache: true }
      - run: pixi run -e ci ci-pack      # pack + vendor + embed the pinned tools
      - run: pixi run -e ci ci-doctor    # verify every blob, write nothing
      - run: pixi run -e ci ci-publish   # one orphan commit, force-pushed
```
* **The tool's own distribution** is a conda package (D8): `pixi publish --path
  crates/pixi-sandbox --target-channel file://…` produces a `.conda` and an indexed channel ✅
  (no `rattler-index` needed for a `file://` target); `pixi global install -c <channel>
  -c conda-forge pixi-sandbox` installs it, after which the verbs appear as
  `pixi sandbox …` ✅. Shipping to conda-forge itself (and whether the channel should be a
  GitHub-Pages-hosted prefix.dev-style channel) is **open** — see `research-results.md`.
  Note the channel needs conda-forge as a second channel: a bare `-c <local-channel>` fails to
  solve for `libgcc` ✅.

---

## §7 Integrity and failure model

The promise: **nothing reaches the user's working tree before its sha256 matches the
manifest.** That is implemented as: verify-before-write everywhere (`shard::materialise`,
`join_parts` stages in a temp file and renames only after the whole blob verifies), one code
path for verification (`verify::verify`) shared by `doctor`, `restore` and `unpack`, and an
error type that distinguishes *missing*, *size*, *integrity*, *missing-part* and *dynamic-tool*
so the operator's report is actionable. `doctor` reports **all** failures, not the first ✅.

What this model does **not** protect against: a malicious writer who rewrites payload *and*
manifest consistently (there is no signature/attestation yet — open), and a compromised
*upstream* (the lockfiles are only as trustworthy as the commit that produced them). The
fingerprint marker records identity, not authenticity.

Failure catalogue (each verified in the lab ✅ unless marked):

| symptom | cause | response |
| --- | --- | --- |
| `integrity: … expected sha256 …, got …` | truncated/corrupted file | re-fetch the branch; re-run with `--force` |
| `missing split part …` | a `.partNNN` not pushed (or filtered by a proxy) | re-fetch; check `doctor --verify` output |
| `tool … is dynamically linked` | a `~/.pixi/bin` trampoline or a distro binary got embedded | ship the static asset (D4); the packer refuses it too |
| `pixi install` wants the network | markers missing/wrong, or a package absent from `envs/<env>/pack` | D5; re-pack with the same `pixi.lock` |
| unpack fails at ~all of the disk's free space | `$TMPDIR` on a small tmpfs | use `--work-dir`/default work dir (§4) |
| restore fails | misconfigured work dir | use --work-dir on same filesystem |

Backward compatibility: `doctor` must keep reading schema 1 across tool versions (§9), and a
schema bump requires a fixture update in `tests/manifest.rs`.

---

## §8 Repository layout and conventions

| path | notes |
| --- | --- |
| `crates/pixi-sandbox-core` | wire format, sharding, tool pins, verification — no CLI, no network |
| `crates/pixi-sandbox` | the CLI: pack, publish, restore, unpack, doctor, plan and tools list; its integration suite uses synthetic transports and fake local helper tools |
| `.knowledge/` | this document, `decisions.md`, `research/` (evidence + prototype) |
| `docs/` | Astro + Starlight site; `/restore` is written for someone with no network |
| `.github/workflows/` | `ci.yml`, `docs.yml`, unified `publish-sandbox.yml`, `release.yml` (multi-platform binaries) |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | canonical helper-tool pins compiled into the CLI (§10) |
| `.pixi-sandbox.toml` | explicit publish bundles/platform runners consumed by `plan` and the reusable workflow (D11) |
| `.github/actions/` | `setup-pixi-sandbox` (like `setup-pixi`) + `publish-pixi-sandbox` composites (D11) |
| `action.yml` | root composite alias so `uses: Archont561/pixi-sandbox@vX` works like `setup-pixi` |
| `pixi.toml` | tasks: `lint`, `test`, `coverage`, `docs-build`, `sandbox-*` |
| `scripts/restore.sh` | one-liner offline reconstruction with PATH aliases |

Conventions that keep it maintainable: the CLI stays a thin shell over `pixi-sandbox-core`;
comments point at a decision ID or a measurement rather than restating code; generated weight
(`.pixi/`, `target/`, `vendor/`, `dist/`, `.pixi-sandbox/`) is never committed; docs pages are
user-facing, `.knowledge/` is for people changing the design.

---

## §9 Rust implementation status

The Rust CLI now implements the flow without a second wire format (original Python prototype archived):

1. **`pack`** resolves the requested environments through `pixi-pack`, optionally vendors
   cargo crates in loose or per-crate-tar mode, fetches pinned tools only after sha256
   verification, rejects dynamic embedded tools, records/splits payload files, and writes the
   manifest plus branch guides.
2. **`unpack`** accepts a transport or bare pack, verifies the transport before writes,
   materialises a pack in project-local staging, runs `pixi-unpack`, and leaves a raw prefix.
3. **`restore`** verifies every selected byte before writing, materialises tools/environments/
   vendor sources into project-local staging, atomically installs complete prefixes, writes Pixi
   markers, and writes a relative cargo source replacement when requested.
4. **`publish`**, `doctor`, and `tools list` remain implemented as before; `publish --keep` and
   `tools update` intentionally report that they are deferred rather than silently pretending to
   work.
5. **`plan`** parses the strict schema-1 `.pixi-sandbox.toml`, rejects unsafe/implicit runner
   selection, and emits a stable GitHub Actions `include` matrix for exactly the reviewed bundles.

The test suite includes a hermetic synthetic Rust CLI flow — pack → doctor --verify → unpack →
restore — with an intentionally sharded blob, plus restore tests for the checked-in vendor tree.
Core tests additionally reject manifest tool paths that escape the transport.

Release engineering now provides **validated static Rust bootstrap binaries** for tier-1 platforms
via `release.yml`. CI and the airlock proof both use the Rust self-binary embedded in the branch.

---

## §10 Helper tools: embedded pinning and explicit overrides

`crates/pixi-sandbox-core/assets/tools.lock.json` (schema 1) is compiled into every release and
pins, per platform, the exact tool builds the flow uses. `--tools-lock PATH` is the explicit
override path for a reviewed organisation mirror or replacement pin:

```json
{ "schema": 1, "generated_at": "…",
  "tools": { "pixi-unpack": { "version": "0.7.11",
      "url_template": "https://github.com/Quantco/pixi-pack/releases/download/v{version}/pixi-unpack-{target}",
      "platforms": { "linux-64": { "target": "x86_64-unknown-linux-musl",
                                   "sha256": "8191f586…", "linkage": "static" } } } } }
```

Rules (D4): `pack --fetch-tools` and CI download → verify sha256 → cache
(`PIXI_SANDBOX_TOOLS_CACHE`, default `~/.cache/pixi-sandbox/tools`) → execute; the version is
verified against the running binary (`--version`) before it is used; nothing is installed
system-wide; nothing is taken from `PATH` unless you ask for it by omitting `--fetch-tools`.
The embedded catalogue covers linux-64, linux-aarch64, osx-64, osx-arm64, and win-64 for pixi,
pixi-pack, and pixi-unpack. Coverage of a pin is not a platform proof: native pack/restore
validation remains required under D11. The embedded copy must be **static** on Linux: a
`~/.pixi/bin` shim is a 766 KiB trampoline that execs a dynamic binary inside its own prefix —
it works on the build machine and dies on the airlock ✅. Mach-O and PE require native proof.

## §11 Cargo vendoring

### §11.1 Layout: a sibling of `envs/` and `tools/`

`cargo vendor` output travels in the branch under `.pixi-sandbox/vendor/` — a **sibling** of
`envs/` and `tools/` (D6), not a subdirectory of an env, because:

1. crates are project-scoped, not env-scoped (one `Cargo.lock`, one tree);
2. `envs/<env>/` must stay exactly what `pixi-pack` produced, so it can be re-packed and
   compared by digest;
3. `doctor`/`restore` iterate environments; mixing a crate tree into `envs/` would make blob
   accounting lie;
4. a partial restore (`--envs dev`) still needs the crates (§4);
5. it mirrors where restore puts it (`<project>/.pixi-sandbox/vendor`), so the branch layout
   and the project layout agree.

### §11.2 Storage modes

Modes: **`loose`** (default, `cargo vendor --versioned-dirs`) — 33 crates ≈ 35 MB / 1441 files
✅ — and `tarballs` (one archive per crate) for transports where file count matters more than
dedup. Loose wins on git: a single crate bump costs +0.07 MiB in git (only the changed files)
vs +3.96 MiB if the whole tree is one artifact ✅, and a fresh clone of the vendored tree is
390 ms vs 59 ms ✅.

### §11.3 Wiring the vendored tree into cargo

Wiring after restore is a `[source]` replacement, written with a **relative** path:

```toml
[source.crates-io]
replace-with = "vendored-sources"
[source.vendored-sources]
directory = ".pixi-sandbox/vendor"     # resolved against the project root
```

Measured subtleties: the base directory of a relative `directory` differs by *where the config
lives* — a project `.cargo/config.toml` resolves against the project root, `--config` against
the cwd, `$CARGO_HOME/config.toml` against the CARGO_HOME's parent ✅; with the replacement in
place, a dead-network `cargo build` works even without `--offline` ✅ (we still export
`CARGO_NET_OFFLINE=true`, for fail-fast). Cargo gives **no useful error** if a crate is
missing, so the packer must fail at pack time rather than let the airlock discover it —
notably `cargo vendor` hard-fails when the same crate+version is reachable from two sources
(crates.io and a git dependency) ⚠️ known upstream.

Vendoring carries **no toolchain and no build artifacts**: `rustc`/`cargo` must come from the
restored environment (which is why `dev`/`ci` environments include rust) and the airlock pays
the compile time (measured: a small project's debug build, severed network, 13–18 s ✅).

---

## §12 Git access: one trait, two implementations

Everything this project does to a repository — publish a snapshot as an orphan branch, fetch
one back — goes through `pixi-sandbox-git`, and through the `GitProtocol` trait inside it:

```rust
pub trait GitProtocol {
    fn publish(&self, snapshot: &Snapshot<'_>) -> Result<Published>;
    fn fetch_into(&self, remote: &str, branch: &str, dest: &Path) -> Result<Published>;
    fn branch_exists(&self, remote: &str, branch: &str) -> Result<bool>;
    fn remote_size(&self, remote: &str, branch: &str) -> Result<Option<u64>>;
}
```

**Why a trait rather than calling `git` where it is needed.** `publish` and the airlock's
fetch are the two operations that touch a *remote* — the one part of the system a unit test
cannot have, and the one part that is dangerous to run for real (it force-pushes). With the
trait:

* `FakeGit` is an in-memory remote with branch tips, an operation log, and a switch that makes
  the next push fail. Tests of the publish/fetch flow are then fast, offline, and cannot reach
  anybody's repository — including this one.
* `ShellGit` is the real thing, and it takes a `Runner`: `ProcessRunner` executes,
  `RecordingRunner` executes *and* records every argv, `PreviewRunner` records and executes
  nothing. That is how `publish --dry-run` is implemented — the same code path with a runner
  that does not run — and how a test can assert "orphan" (a `commit-tree` with no `-p` seen)
  and "force" (`push --force`) as facts instead of intentions.

The trait is deliberately **semantic** (`publish`, not `push`/`commit_tree`/`update_ref`): a
mock of a command surface proves nothing, while a mock of the two operations states the
contract — one parentless commit per publish, history replaced rather than appended, the
snapshot directory never written to, `fetch_into` producing exactly the branch tree. Those
four properties are asserted against **both** implementations (`crates/pixi-sandbox-git/tests/publish.rs`).

Implementation notes that are decisions in disguise:

* **The payload is not copied.** `publish` points `GIT_DIR` at a scratch repository *next to*
  the transport directory (same filesystem, never /tmp), `GIT_INDEX_FILE` at a scratch index, and
  `GIT_WORK_TREE` at the transport itself, then commits with plumbing (`add` → `write-tree` →
  `commit-tree` → `update-ref` → `push --force`). No `.git` appears in a directory we do not own, no second
  copy of a 250 MiB payload is made, and the scratch is removed even if a later step fails. Previously
  the scratch was inside the transport and leaked `.pixi-sandbox-publish-<pid>/` because git only ignores `.git`, not custom GIT_DIR names.
* **`fetch_into` materialises with plumbing** (`ls-tree -r -z` + `cat-file blob`) instead of
  `archive | tar -x`: no tar crate, no external `tar`, byte-exact control, and the `.git`
  directory is removed afterwards so the airlock gets branch *content*.
* **A rejected push is its own error** naming the remote and the branch, because the fix is on
  the remote (a protected branch, a token scope) and not in the payload.
* **`remote_size` may answer `None`**: the git wire protocol has no "how big is that branch"
  query, so it only reports a size for a remote reachable as a local path.

`restore` is now ported (§9); future airlock fetch integration can use `fetch_into` so the same
mock covers the "branch is not there" and "push was rejected" paths without an external remote.

---

## §13 Tests, and why they never point at this repository

The test suite has two fixture roots and one rule.

```
crates/pixi-sandbox/tests/fixtures/
├── demo-project/     a complete pixi project definition (pixi.toml + pixi.lock, Cargo.toml +
│                     Cargo.lock, src/) — what a *user's* project looks like
└── transport/        a synthetic, already-packed payload (manifest + fake envs, tools, vendor;
                      one blob split into .partNNN) — what `pack` produces
```

**The rule: tests must not sandbox this repository.** This repository *is* the tool's
development environment: its `dev` environment contains `pixi-pack`, and its lockfile resolves
hundreds of MiB. A test pointed at the repository root therefore tests the developer's
machine: it silently passes where a fresh clone would fail, it is slow, it mutates state, and
a bug that only appears on a *plain* project — one whose environment has no packer in it —
would never be seen. The fixture is the opposite of all four:

* `demo-project/pixi.toml` has **no** `pixi-pack`/`pixi-unpack` dependency, and
  `tests/fixtures.rs::the_demo_project_does_not_depend_on_the_packers` asserts that stays true
  (it reads dependencies, not comments);
* `tests/fixtures.rs::no_test_targets_the_repository_root` scans the test sources and fails if
  a test walks out of its crate from `CARGO_MANIFEST_DIR` (`".."`, `.parent()`);
* `transport/` is committed with real digests, so `doctor`, `publish`, `restore` and the
  verification path are covered with **no pixi, no packer and no network** — including the
  split-blob (`.partNNN`) case that a 95 MiB payload would otherwise be needed for;
* fixtures are read-only in tests: anything that writes copies them into a `tempfile::TempDir`
  first.

Fixture transport is static and checked in. It was originally generated via Python, now maintained
as a synthetic payload with real digests; `demo-project/` is a real project (pixi.lock from `pixi lock`,
Cargo.lock from `cargo generate-lockfile`).

Two findings that shaped this section, both measured:

* the fixture project is **packable without installing an environment**: `pixi-pack` resolves
  from `pixi.lock` and fetches the packages itself, so pack-integration tests need network but
  not a multi-GiB local install (measured: 485 blobs / 100.6 MiB packed in 2.2 s from the
  fixture);
* the split-blob fixture is what caught a real bug in `verify` (a split blob was reported as
  *missing* because the whole file correctly is not on the branch) — evidence for keeping the
  awkward cases in the fixture rather than the happy path.

A test that genuinely needs the network (a real `pixi install`, a real tool download) is
`#[ignore]`d with the reason in the attribute, so `cargo test` stays offline and fast.
