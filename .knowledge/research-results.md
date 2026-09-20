# Research results

The study's conclusions without the workflow detail. Raw numbers: `research/EVIDENCE.md`.
Decisions: `decisions.md`. Full design: `design.md`.

## The verdict, in one paragraph

Shipping a pixi environment to an airlocked machine through a git orphan branch is cheap and
safe: a two-environment payload of 218.8 MiB packs into a **107.34 MiB** branch (git dedup does
the heavy lifting), a restore into a project with an empty package cache and no network
completes in seconds, and the subsequent `pixi install --frozen --offline` /
`cargo build --offline` checks pass on a severed network. The cost that decides the design is
not bandwidth but *file granularity*: whole `.conda` files as shards (split only above 95 MiB)
keep dedup working, while fixed-size chunking or prefix tarballs destroy it.

## Answers to the original questions

| question | answer |
| --- | --- |
| Is `pixi-pack` needed? | **Yes** — it produces the transportable unit (raw `.conda` files as a local channel + `environment.yml` + `pixi-pack.json`) [D2] |
| Is `pixi-unpack` needed? | **Yes** — it is the only supported way back to a prefix, and it must be *in* the branch [D3] |
| Should both be compiled into our binary? | **Not in v1** — neither is on crates.io; embed the pinned static release assets and keep a `rattler` library implementation as the v2 path [D4] |
| What else is uncovered? | The pixi **markers** after a raw unpack [D5], the Rust **toolchain** (vendoring carries no `rustc`), a **conda channel** to install the tool itself from [D8], and **signing/provenance** (open) |
| One branch per env, or per platform+env-set? | **Per platform × env-set** — git dedups shared packages (~2× measured across two envs) while mixing platforms ships bytes nobody can run |

## What was measured (and what it changed)

| finding | number | consequence |
| --- | --- | --- |
| Raw `.conda` transport vs prefix tar | ≈3.7× smaller | `--directory-only` is the default [D2] |
| Git dedup across two environments | ~2× of the bytes | one branch per env-set is worth it |
| Cost of a new 5 MB package | ≈+4.8 MiB | incremental publishes are cheap |
| `push --force` + `git gc --aggressive` | no shrink | rotate branches instead [D1] |
| GitHub per-file limit | >100 MiB rejected, >50 MiB warned | 95 MiB shard limit [§3] |
| Vendored crates, per crate bump | +0.07 MiB (loose) vs +3.96 MiB (archive) | loose tree is the default [D6] |
| `pixi-pack`/`pixi-unpack` on crates.io | absent | ship static assets, don't link [D4] |
| `~/.pixi/bin` shims | 766 KiB trampolines, dynamic payload | never embed one; static only [D4] |
| Restore with empty caches, no network | succeeds; build 13–18 s | the flow is genuinely offline |
| `/tmp` staging | 993 MB tmpfs → mid-unpack failure | work dir on the project filesystem [§4] |
| `pixi install --frozen --offline` without markers | fails (tries the network) | write the markers [D5] |
| Full cold-start reproduction (prototype baseline) | 62 s cold / 52 s second | established the original CI design [§9] |
| Current Rust-port proof (dev + docs) | 177 s, 931.7 MiB / 10,621 verified blobs | Rust pack/doctor/publish and both bootstrap/Rust restores pass [EVIDENCE §12] |

## The end-to-end proofs

1. **Transport proof** (prototype): pack 218.8 MiB → branch 107.34 MiB → restore on an empty
   cache with no network → `pixi install --frozen --offline` a no-op; a tampered blob aborts
   the restore.
2. **Cargo-vendor proof**: branch 110 MB (envs 51 % / tools 36 % / vendor 13 %); a
   severed-network debug build from the vendored tree in 17.99 s; tamper ⇒ integrity mismatch.
3. **Current Rust-port proof**: `pixi run -e dev sandbox-proof` → 177 s, all green,
   931.7 MiB / 10,621 verified blobs. Rust performs pack, verify, and publish; the embedded
   portable bootstrap performs the airlock restore. A separate network-severed Rust `restore`
   run against that same branch also passed its Pixi and Cargo offline assertions [EVIDENCE §12].
4. **Distribution proof** [D8]: `pixi publish` → 607,957 B `.conda` → `file://` channel →
   `pixi global install` → `pixi sandbox doctor --verify` = 1512 blobs / 249.8 MiB / 1.28 s.

## Added after the first round of review

* **Git behind a trait** (`pixi-sandbox-git`, D9): `publish` is implemented and tested against
  both a real `git` and an in-memory mock — 11 tests, including "the branch is an orphan"
  (asserted from recorded argv), "history is replaced" (`rev-list --count` = 1 after two
  publishes) and "a rejected push leaves the remote alone". `--dry-run` is the same code path
  with a runner that does not run.
* **Fixtures instead of the repository** (D10): tests operate on a complete fixture pixi
  project and a committed synthetic transport, never on this repository — whose `dev`
  environment contains `pixi-pack` and would hide exactly the bugs worth catching. Measured
  bonus: the fixture packs without installing an environment (485 blobs / 100.6 MiB / 2.2 s).
* **GitHub Actions as one-liners** (design.md §6): every step is `pixi run -e <env> <task>`;
  logic lives in `pixi.toml`. Paths come from the environment because pixi tasks have no
  `${VAR:-default}`.

## Open questions (deliberately not decided)

* **Provenance/signing**: nothing detects a branch writer who rewrites payload *and* manifest
  consistently. Sigstore/in-toto or a signed manifest is the likely answer.
* **Platform matrix**: linux-64 is proven; osx-arm64/win-64 need a packer that can
  cross-pack or a machine of that platform (the payload is per-platform by nature).
* **Rotation**: `--keep N` needs a policy (rebuild history vs new branch names) and a
  repository-gc story with the remote's administrator.
* **Channels for the tool itself**: conda-forge vs a Pages-hosted prefix.dev channel vs
  release assets — including licensing for repacked upstream binaries.
* **Partial restores**: `--envs` works, but the *vendor* tree is always materialised; a
  per-env vendor split is possible only if crates ever become env-scoped.
* **Windows/macOS airlocks**: the static-asset story is proven for linux-64 only; the
  `linkage` check treats Mach-O/PE as "system" rather than verifying them.
