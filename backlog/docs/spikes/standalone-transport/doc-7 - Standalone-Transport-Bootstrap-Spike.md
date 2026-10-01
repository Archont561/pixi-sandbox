---
id: doc-7
title: Standalone Transport Bootstrap Spike
type: other
created_date: '2026-10-01 15:18'
updated_date: '2026-10-01 15:18'
tags:
  - spike
  - transport
  - restore
  - measurement
  - bootstrap
---
# Spike — standalone transport bootstrap and tool deduplication

**Status:** measured spike, no production code changed. Produced for backlog task-24, which
requires binary-size and restore-disk measurements, a schema compatibility policy, a
duplicate-packaging rule, and an explicit decision recorded through the Backlog decision API
(**decision-2**, accepted). It completes the no-go already reasoned in `doc-5` (rattler) with
numbers taken from the *current* v0.3.7 design: five native packages published, the channel
install canonical, and the verified standalone `pixi-sandbox` binary embedded under
`.pixi-sandbox/tools/<platform>/` as the transport bootstrap.

**Question.** Should the standalone `pixi-sandbox` binary absorb `pixi-pack`/`pixi-unpack`
(instead of embedding them as pinned helper tools), what would that buy in binary size and
restore disk, how is the transport schema kept compatible, and how is `pixi-sandbox` kept from
being shipped twice — as the bootstrap tool *and* as an environment dependency?

**Environment.** All numbers were produced on 2026-10-01 in the restored developer sandbox of
this repository (Debian 12 bookworm, Linux 6.1, x86_64, 2 vCPU, git 2.39.5, cargo 1.98.1,
pixi 0.81.0, pixi-unpack 0.7.11) from the published `sandbox/developer-linux-64` branch
snapshot `7c69402` (manifest schema 2, packed by pixi-sandbox 0.3.6 from commit `910df8f`,
created 2026-10-01T14:20:49Z). Tags follow the `.knowledge` convention: ✅ measured here,
⚠️ reasoned or projected from measured parts, 📄 upstream-documented (not fetchable from this
airlock: crates.io and prefix.dev are unreachable; github.com and npm are not).

Every command is reproducible from a restored checkout; the exact invocations are in §5.

---

## 1. Binary size — what each design weighs ✅

Built `cargo build --release --offline -p pixi-sandbox` from this checkout (v0.3.7, vendored
crates, 1 m 19 s): **3 437 520 bytes (3.28 MiB)**, dynamically linked against the restored
environment (dev build; release assets are static musl). The restored bootstrap of the same
line, pixi-sandbox 0.3.6 static, measures **3 553 504 bytes (3.39 MiB)** — the dev build is
within 4 % of the shipped static binary, so it is an honest stand-in.

| Binary | Raw bytes ✅ | sha256 (first 12) | gzip -9 ✅ |
|:---|---:|:---|---:|
| `pixi-sandbox` v0.3.7 dev build | 3 437 520 (3.28 MiB) | `69546e08f523` | 1 696 101 (1.62 MiB) |
| `pixi-sandbox` v0.3.6 static (shipped bootstrap) | 3 553 504 (3.39 MiB) | `661c15b35592` | 1 760 938 (1.68 MiB) |
| `pixi-unpack` v0.7.11 static (embedded helper) | 15 729 744 (15.00 MiB) | `8191f586b734` | 6 794 003 (6.48 MiB) |
| `pixi` v0.81.0 (embedded helper) | 80 335 584 (76.60 MiB) | `248510f5f754` | 34 307 524 (32.72 MiB) |

**The proxy for "absorb the unpacker".** `pixi-unpack` *is* a rattler front-end, and its
static musl binary is the best empirical proxy for what linking the rattler install path into
`pixi-sandbox` costs: **15.00 MiB raw / 6.48 MiB gzip** ✅ (doc-5 §2.1 reached the same
conclusion from the same proxy). A `pixi-sandbox` that absorbs it plausibly lands at
12–18 MiB raw ⚠️. Net transport change of absorbing: **somewhere between −6.5 and +1.5 MiB
in-pack** (see §2) — the binary that replaces the helper weighs roughly what the helper
weighed.

**What absorption cannot remove.** `pixi` (76.6 MiB raw) stays regardless — the airlock must
run `pixi install --frozen --offline` and `pixi run` after the prefix is installed. Even a
perfect v2 that deleted both `pixi-unpack` and the bootstrap copy leaves the tools section at
~32.9 MiB in-pack.

## 2. Restore disk and transport bytes — the whole pipeline, measured ✅

Measured end-to-end on the real published branch, the way an airlock actually gets it
(fetch → worktree → restore), with `du` sampling every 0.5 s during the restore:

| Layer | Measured ✅ | Notes |
|:---|---:|:---|
| git object store for one snapshot | **505 MiB** | shallow fetch, 14 s wall |
| extracted transport (git worktree) | **845 MiB** | what `restore --branch-location` reads |
| restore wall clock | **16 s** | includes hashing all 819.9 MiB of declared blobs |
| restore preflight request | **2 831.9 MiB** | packed 445.9 + unpacked 1 829.8 + 2 × vendor 278.1 |
| **peak project footprint** (sampled) | **2 746 MiB** | during vendor staging; final tree is 2 238 MiB |
| **peak combined** (project + worktree + git store) | **4 098 MiB** | the true disk cost of one airlock restore |
| final project tree | **2 238 MiB** | envs 1 669 + 174, tools 96, vendor 301 (`du -sm`) |

The preflight (2 831.9 MiB) is honest and slightly conservative: the measured peak is 2 746 MiB,
~86–185 MiB below it (the preflight assumes all packs and all unpacked sizes live at once and
doubles the vendor bytes, while the vendor stage is renamed — not copied — into place).

**Per-part transport cost, from the packfile itself** ✅ (`git verify-pack -v` joined with
`git rev-list --objects`; "in-pack" is the bytes the branch actually stores after zlib):

| Part | Objects | Raw | In-pack | Share of branch |
|:---|---:|---:|---:|---:|
| `pixi` 0.81.0 | 1 | 76.6 MiB | **32.9 MiB** | 6.5 % |
| `pixi-sandbox` 0.3.6 (bootstrap) | 1 | 3.4 MiB | **1.7 MiB** | 0.34 % |
| `pixi-unpack` 0.7.11 | 1 | 15.0 MiB | **6.5 MiB** | 1.3 % |
| env `default` packs (.conda, already zipped) | 35 | 398.9 MiB | 393.9 MiB | 78.2 % |
| env `web` packs | 27 | 42.4 MiB | 42.3 MiB | 8.4 % |
| vendor tree (10 254 declared files) | 9 663 | 142.2 MiB | **25.8 MiB** | 5.1 % |
| manifest.json + oracles | 3 | 2.9 MiB | 0.7 MiB | 0.14 % |
| **total** | **9 733** | **681.3 MiB** | **503.9 MiB** | 100 % |

Two findings that change the framing of the "15 MiB prize":

1. **The prize is 6.5 MiB, not 15.0.** Removing `pixi-unpack` from the tools map removes
   6.5 MiB of in-pack bytes (1.3 % of the branch) and one object out of 9 733 — git stores
   the deflate, not the file. `pixi`, which cannot be removed, costs 32.9 MiB in-pack, 5×
   that.
2. **Git dedups the vendor tree, the manifest does not.** The manifest declares 10 254 vendor
   blobs (278.1 MiB, and the restore preflight doubles that number), but the branch stores
   9 663 unique objects — 591 vendor files are byte-identical to other vendor files (repeated
   LICENSE/boilerplate across crate versions) and share one object — and they deflate to
   25.8 MiB in-pack. Dedup helps the branch, not the airlock disk: restore still materialises
   all 10 254 files.

**Restore-disk alternatives**, compared on the measured baseline:

| Option | Peak project | Peak combined | Transport delta | Correctness surface |
|:---|---:|---:|:---|:---|
| **A. Status quo** (pack copy → `pixi-unpack` stage → rename) | 2 746 MiB ✅ | 4 098 MiB ✅ | — | delegated to the unpacker's maintainers |
| **B. Full absorb** (rattler pack *and* install in-process) | ≈ 2 300 MiB ⚠️ | ≈ 3 650 MiB ⚠️ | **+~10 MiB in-pack on this repo's own branch** ⚠️: binary +~1.5 in-pack, plus ~28 rattler crates 📄 × 1.79 MiB raw / 0.17 MiB in-pack per crate ✅ ≈ +4.7 in-pack vendor, to *save* 6.5 | prefix installation (placeholder rewriting, `conda-meta`, activation) becomes ours, on a machine with no second chance |
| **C. Narrow absorb** (keep `pixi-pack` output; streaming install only, no pack copy) | ≈ 2 300 MiB ⚠️ (peak − 445.9 MiB of measured pack copies) | ≈ 3 650 MiB ⚠️ | as B, without saving the `pixi-pack` CI download | as B |
| **C′. No checkout** (read blobs from the git object store; keep both helpers) | 2 746 MiB ✅ | **≈ 3 253 MiB** ⚠️ (− 845 MiB measured worktree) | none | none new; changes the restore contract from a directory to a repo+ref |

Option C′ is the only alternative that improves peak disk (−11 % combined) without growing the
binary or moving the correctness surface — it is noted for the backlog, not adopted here: it
changes the `--branch-location` contract that `doctor`, `restore`, `unpack`, the generated
launchers, and `scripts/airlock-gate.sh` all share, and today's measured peak (2 746 MiB for a
1 830 MiB environment, 16 s) has not been reported as a blocker by any real airlock.

## 3. Schema migration and compatibility policy

The accepted decision (decision-2) changes **no** wire format: schema stays 2. The policy that
governs any future change, recorded so it is not re-derived per task:

1. **Readers accept every schema they understand and refuse only newer ones.** `manifest.rs`
   accepts 1..=`SCHEMA_VERSION` and refuses newer — a published branch outlives the binary that
   packed it. This rule is load-bearing and tested (`tests/manifest.rs` freezes the wire
   format; a bump requires a fixture update there).
2. **Additive-compatible fields may land without a bump** only when a reader of the previous
   schema ignores them safely: the field must be `#[serde(default)]`-shaped and must not
   change what an existing field *means*. Schema 2's `envs.<name>.files` landed this way as an
   addition; schema-1 envs are reported `unverifiable`, never failed (D13).
3. **Removals and semantic changes are a schema-3 event.** If a future release drops
   `pixi-unpack` from the tools map (option B/C) or adds tool roles (bootstrap vs helper),
   `SCHEMA_VERSION` bumps, the new field/removal is specified in doc-2 §3, and `doctor`,
   `restore`, and `unpack` must keep reading schema 1 and 2 for as long as published branches
   exist — the airlock may hold a transport packed by an older release. A schema-1 transport
   without an oracle is the precedent: report, never guess.
4. **The tools map is the dedup oracle for tool identity** (see §4): one entry per tool name,
   keyed by extension-free name, with an explicit in-transport path. Any dedup mechanism that
   relies on git hardlinks is invalid — `git archive`/worktrees do not preserve them (doc-1
   non-goal; measured: the worktree materialises every blob as a separate file).

## 4. Duplicate `pixi-sandbox` packaging — the rule

Today there is no double shipment in the flagship transport: `sandbox/developer-linux-64`
carries `pixi-sandbox` exactly once, as the `tools/linux-64/pixi-sandbox` bootstrap
(1.7 MiB in-pack, 0.34 % of the branch) ✅, and neither `default` nor `web` depends on the
`pixi-sandbox` conda package ✅. The risk is a *user* project that (a) depends on the
published `pixi-sandbox` package in a packed environment and (b) packs with `--self-bin`.
The rule, specified rather than left implicit:

1. **The `tools` entry is the canonical executable.** Launchers (the generated `restore.sh` /
   `restore.ps1`, `scripts/restore.sh`), the bare-binary bootstrap path, and the TASK-33
   user-PATH registration execute the manifest-verified copy under `.pixi/tools/<platform>/`,
   never a copy discovered on `PATH` or inside a restored environment. One execution source,
   byte-verified by the manifest.
2. **An environment copy is payload, not a tool.** A project that depends on the
   `pixi-sandbox` package gets it restored verbatim inside `.pixi/envs/<env>/` — excluding it
   would corrupt the prefix against `pixi.lock` and break the D13 per-file oracle. The
   duplication is bounded and cheap: the bootstrap costs 1.7 MiB in-pack ✅; the env package
   is the user's own lockfile choice.
3. **`pack` warns instead of refusing.** When a packed environment contains a
   `pixi-sandbox-*` package, the packer should notice (the pack channel's file names are
   visible to it) and print the duplication cost with the guidance: keep `pixi-sandbox` out of
   packed environments; it is a connected-host/publisher dependency. Refusing would block the
   legitimate case of a project whose *airlock workflow* runs `pixi-sandbox` commands from
   inside the restored environment.
4. **Promotion (use the environment's copy as the bootstrap) is rejected.** It saves 1.7 MiB
   in-pack (0.34 %) but couples the bootstrap's version to the project's lockfile: an
   environment pinned to an older `pixi-sandbox` could not read a newer manifest schema, and
   the binary the launcher executes would silently differ from the one that packed the
   transport. `--self-bin` stays the explicit, reviewed source of the bootstrap.

## 5. Reproduction

```bash
# from a restored checkout of this repository at 5a0f0c1 (v0.3.7):
source .pixi/sandbox-env.sh                       # restored tools on PATH, CARGO_NET_OFFLINE
cargo build --release --offline -p pixi-sandbox   # 1 m 19 s here
stat -c '%n %s' target/release/pixi-sandbox .pixi/tools/linux-64/{pixi,pixi-sandbox,pixi-unpack}
gzip -9 -c <file> | wc -c                         # deflate proxy for the branch's storage

# end-to-end restore-disk measurement (airlock ingress shape):
git clone --quiet --no-checkout --depth 1 \
  --branch sandbox/developer-linux-64 https://github.com/Archont561/pixi-sandbox.git repo
du -sm repo/.git                                  # 505
git -C repo worktree add ../transport-checkout sandbox/developer-linux-64 --force
du -sm transport-checkout                         # 845
mkdir project && target/release/pixi-sandbox restore \
  --branch-location transport-checkout --output-path project   # 16 s; preflight 2831.9 MiB
# peak sampling: loop `du -sm project transport-checkout repo/.git` every 0.5 s during the
# restore — measured peak project 2746, combined 4098, final project 2238

# per-part transport cost from the packfile:
git -C repo rev-list --objects sandbox/developer-linux-64 > objects.txt
git -C repo verify-pack -v .git/objects/pack/*.pack | awk '$2=="blob"{print $1,$3,$4}' > packsizes.txt
# join on object sha; group by path prefix (see §2 table)
```

## 6. Recommendation

**Accept the standalone design as it stands; retain the helpers** — recorded as
**decision-2 (accepted)**:

- `pixi-sandbox` is the standalone transport/restore orchestrator and the single transport
  bootstrap; the verified binary travels once, under `tools/<platform>/`, executed by every
  launcher path (rule §4.1).
- `pixi-pack`/`pixi-unpack` remain pinned, sha256-verified static helper tools (D2/D3/D4),
  reaffirming D12 with current numbers: absorbing the unpacker trades 6.5 MiB of in-pack
  branch bytes for a binary that grows by about the same, ~28 vendored crates on this repo's
  own branch, and ownership of prefix installation on a machine that cannot be debugged.
- Schema stays 2 under the compatibility policy in §3; no migration is proposed.
- The dedup rules in §4 are the specification for "never shipped twice"; the pack-time notice
  (§4.3) and the TASK-33 registration pointing at `tools/<platform>/` are the implementation
  tasks that follow from it.

**What would change it** (triggers, in the `.knowledge` sense): the four triggers of doc-5 §5
stand unchanged — an unmaintained `pixi-unpack` pin, a real airlock blocked by peak restore
disk (then prototype option C only, measured against §2's baseline), a supported rattler
"install this local channel into this prefix" entry point, or a versioned pack format
specification. To them this spike adds: **a real airlock blocked by *combined* disk** (git
store + worktree + restore), where option C′ (no checkout) is the first lever to pull because
it changes no binary and no format.
