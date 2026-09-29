# Spike — replacing the `pixi-pack` / `pixi-unpack` subprocesses with `rattler`

**Status:** spike note, no code. Produced for backlog task-9, which requires a measured
trade-off *before* any implementation and a go/no-go recorded as a decision ID.

**Question.** `design.md` §1.3 and D2/D3 record that re-implementing pack and unpack on the
[`rattler`](https://crates.io/crates/rattler) library is "possible but not v1 work". Is it worth
doing now? What would it actually save, and what would it cost?

**Reference environment.** The numbers below come from this repository's own published transport,
`sandbox/developer-linux-64`, restored and verified offline on 2026-09-29 (Debian 13, 2 vCPU,
pixi 0.81.0, pixi-unpack 0.7.11, cargo 1.98.1, linux-64). Tags follow the `.knowledge` convention:
✅ measured here, ⚠️ reasoned.

---

## 1. What is actually in the payload today

| Part | Bytes | Blobs | Share of payload |
|:---|---:|---:|---:|
| environment `default` (packed `.conda` channel) | 453.0 MiB | 56 | 54.8 % |
| vendored crates (this repo's own `Cargo.lock`) | 278.1 MiB | 10 254 | 33.7 % |
| **tools total** | **94.9 MiB** | **3** | **11.5 %** |
| ├─ `pixi` 0.81.0 | 76.6 MiB | 1 | 9.3 % |
| ├─ `pixi-unpack` 0.7.11 | **15.0 MiB** | **1** | **1.82 %** |
| └─ `pixi-sandbox` 0.2.0 (self-bootstrap) | 3.3 MiB | 1 | 0.4 % |
| **payload** | **826.0 MiB** | **10 313** | 100 % |

✅ `doctor --verify`: 10 313 blobs, 826.0 MiB, 0 failures. Unpacked, the environment is
1 867.5 MiB; the restore preflight asked for **2 876.8 MiB** of working space (packed + unpacked
+ vendor staged side by side).

**The prize, stated exactly.** Removing the `pixi-unpack` subprocess removes **15.0 MiB (1.82 %)
of payload and one blob out of 10 313 (0.01 %)**, once per branch — git stores that blob once no
matter how many snapshots the branch carries (D1, measured dedup). `pixi-pack` is *not* in the
payload at all: it runs on the connected build machine, so replacing it saves zero transport bytes
and only removes a CI-side download.

**What rattler cannot remove.** `pixi` itself — 76.6 MiB, **81 % of the tools bytes** — stays,
because the airlock must run `pixi install --frozen --offline` and `pixi run` after the prefix is
installed (D5 markers, §6). So even a perfect v2 leaves the tools section at ~80 MiB and the
payload at ~811 MiB: **a 1.8 % transport reduction, not a category change.**

---

## 2. What it would cost

### 2.1 The saving is spent on our own binary ✅→⚠️

`pixi-unpack` *is itself a rattler front-end* — its release notes track rattler bumps and it
gained an offline mode through one ([Quantco/pixi-pack v0.7.10](https://github.com/Quantco/pixi-pack/releases/tag/v0.7.10)).
Its static musl binary is therefore the best available empirical proxy for "what rattler's install
path costs when compiled in": **15.0 MiB** ✅.

`pixi-sandbox` is 3.3 MiB today ✅. Absorbing the same functionality plausibly lands it in the
12–18 MiB range ⚠️, and that binary is *also* shipped in the transport (it is the self-bootstrap).
Net transport change: **somewhere between −12 MiB and +3 MiB** ⚠️ — i.e. the measured 15 MiB prize
is largely or entirely spent on the binary that replaces it.

### 2.2 The dependency tree lands in *our* vendored payload ✅

rattler is ~28 crates ([rattler book](https://prefix-dev.github.io/rattler-book/deep-dive-crate-ecosystem/)),
and the top-level `rattler` crate pulls `tokio`, `reqwest`, `reqwest-middleware`, `rayon`,
`rattler_cache`, `rattler_networking`, `rattler_menuinst`, `rattler_shell`, `reflink-copy`
([crates.io dependencies](https://crates.io/crates/rattler/dependencies)) — a networking and async
stack this project currently does not have at all (today: 158 packages in `Cargo.lock`, `ureq` as
the only HTTP client).

This repository dogfoods itself with `cargo_vendor = true`, so **our** dependency tree is a third
of **our** payload: 155 crates → 278.1 MiB → **1.79 MiB of transport per crate** ✅. A few hundred
added crates is a few hundred MiB on this repo's own sandbox branch ⚠️ — the opposite sign, and
larger in magnitude, than the 15 MiB saved. (For a *user's* project the vendor tree is their
`Cargo.lock`, so this cost is ours alone; the binary-size cost in §2.1 is everyone's.)

### 2.3 Format coupling moves, it does not disappear

The pack format is `pixi-pack.json` + `environment.yml` + a `channel/` tree with `repodata.json`
([pixi docs](https://prefix-dev.github.io/pixi/latest/deployment/pixi_pack/)) — a tool's output
layout, not a versioned specification. Two exits, both with a bill:

- **Stay byte-compatible** with what `pixi-unpack` expects (D2's "what would change it"): the
  coupling remains, now as our code tracking someone else's undocumented layout, without their CI.
- **Own the format**: `manifest.json` `SCHEMA_VERSION` bumps, and §9 obliges `doctor` to keep
  reading schema 1 across tool versions — so we would carry both readers, plus a regenerated
  transport fixture (today a synthetic 95 KB payload with a real conda prefix) that would have to
  become a real indexed channel.

### 2.4 Correctness surface we would be taking on

Installing a prefix is not "extract the tarballs": `conda-meta` records, hardlink/reflink
strategy, activation scripts, and **prefix placeholder rewriting in binaries** (padded, in-place —
distinct from the text-file relocation we already do, CONTEXT invariant 7). A bug here is not a
failed CI run, it is a broken environment on a machine with no network and no second chance. The
current design deliberately delegates exactly this to the tool whose maintainers test it on five
platforms.

### 2.5 Build, supply chain, platform proofs

- `cargo deny check` surface grows from 158 packages to several hundred ⚠️ — licences, advisories
  and duplicate-version bans, all in the lint gate.
- rattler and its tree target edition 2024 and move fast (149 releases of
  `rattler_package_streaming` alone); our workspace pins `rust-version = 1.85` and `rust = 1.98.1`
  in `pixi.toml`. Every rattler bump becomes our MSRV and our `pixi.lock` churn ⚠️.
- Five static release targets (incl. `aarch64-unknown-linux-musl` and `x86_64-pc-windows-msvc`)
  must keep linking statically with TLS in the tree; **invariant 7 says a dynamically linked tool
  is a bug**, and `verify.rs` fails the build over it.
- Today `cargo check --offline --workspace` is 21 s and the whole lint gate ~50 s ✅. A rattler
  tree makes both minutes ⚠️, on every CI run of a repository whose CI is one job.
- It does **not** unlock a platform: D11's macOS/Windows gate exists because the *helper assets*
  are system-linked there — and `pixi`, which stays either way, is system-linked on exactly those
  platforms too (`tools.lock.json`).

---

## 3. What it would genuinely buy

Honest list, because it is not nothing:

1. **One binary, one pin.** `pixi-unpack` leaves `tools.lock.json`, `pack --fetch-tools` has one
   less asset to download and verify, and task-4 (`tools update`) shrinks by a tool.
2. **The `$TMPDIR` trap becomes ours.** Invariant 3 exists because `pixi-unpack` stages into
   `$TMPDIR` and a small tmpfs kills a restore mid-flight; we currently work around it by
   redirecting the child's environment. An in-process installer just… writes where we say.
3. **Peak disk.** ✅ Restoring 1 867.5 MiB of environment asked for 2 876.8 MiB of scratch. A
   streaming install (extract straight into the staging prefix, no intermediate pack copy) could
   cut most of that overhead — the one number in this note where a v2 could be *categorically*
   better, and the one an airlock operator actually feels.
4. **Error quality and control**: failures become typed Rust errors on our side of the boundary
   instead of a child process's stderr, and `verify → write` can be interleaved per package.

---

## 4. Options

| Option | Transport | Risk | Verdict |
|:---|:---|:---|:---|
| **A. Status quo** (D2/D3: pinned static assets, subprocess) | 826.0 MiB | none new | recommended |
| **B. Full v2**: rattler pack *and* install, own format | ≈ −1.8 % ⚠️ | high: format, prefix correctness, supply chain, CI time | not now |
| **C. Narrow**: keep `pixi-pack` output, replace only the *install* step with `rattler_package_streaming` | ≈ −1.8 % ⚠️ | medium; still owns placeholder rewriting and `conda-meta` | the only variant worth a prototype, and only if §3.3 becomes a real complaint |

---

## 5. Recommendation

**No-go for now.** The measurable prize is 15.0 MiB (1.82 %) and one blob out of 10 313; the
measurable costs are a binary that grows by roughly what it removes ⚠️, a vendored dependency tree
that grows at 1.79 MiB per added crate ✅ on this repo's own branch, a minutes-longer CI gate, and
ownership of prefix installation on a machine that cannot be debugged. D2 and D3 stand as written.

**What would change it** (the triggers to watch, in the `.knowledge` "what would change it" sense):

1. `pixi-unpack` stops shipping static release assets for a tier-1 platform, or a pin becomes
   unmaintainable — then the subprocess is no longer cheaper than owning the code.
2. Peak restore disk (§3.3, measured 2 876.8 MiB for a 1 867.5 MiB environment) blocks a real
   airlock. Then prototype **option C only**, and measure peak RSS + peak disk + wall clock against
   today's restore before writing anything else.
3. rattler publishes a supported "install this local channel into this prefix" entry point whose
   placeholder/`conda-meta` handling we do **not** have to reimplement.
4. The pack format gains a versioned specification, which would collapse §2.3 to a single reader.

Any prototype must be measured, not argued: binary size for all five targets, `cargo deny` delta,
cold `cargo check` time, restore wall clock, peak disk, and a byte-for-byte comparison of the
installed prefix against a `pixi-unpack` restore of the same transport.

### Proposed decision text (not yet recorded)

> **D12 — The conda pack/unpack subprocesses stay (rattler is not v2 work yet).**
> *Decision.* `pixi-pack` and `pixi-unpack` remain external, sha256-pinned static assets driven as
> subprocesses (D2, D3). `rattler` is not linked into `pixi-sandbox`.
> *Why.* Measured on this repository's transport: dropping `pixi-unpack` removes 15.0 MiB of
> 826.0 MiB (1.82 %) and one blob of 10 313, while the replacement code is itself ~15 MiB of
> binary, the added crates cost ~1.79 MiB each in the vendored payload, and prefix installation
> (placeholder rewriting, `conda-meta`, activation) becomes ours to get right on a machine with no
> network. `pixi` (76.6 MiB, 81 % of the tools bytes) stays either way, so the payload cannot
> change category.
> *What would change it.* The four triggers in `rattler-spike.md` §5.

Recording this in `decisions.md` as D12 is a maintainer call, which is why task-9's AC#2 is a
separate step from this note.
