---
type: Playbook
title: "Reconstructing on an Airlocked Machine"
description: What a human types on the sealed box: fetch the dist branch, verify digests, install the drivers, reconstruct, run.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, airlock]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T10:30:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["2"] }
sources:
  - { id: githubcom-acme-myprojgit, resource: https://github.com/acme/myproj.git, title: acme/myproj.git }
---

# Reconstructing on an Airlocked Machine

## 2. Workflow B — the airlocked machine (cloning the orphan branch)

The target has: `git`, `tar`, a shell — **and not necessarily `zstd`** ⚠️ (measured absent on the reference
sandbox ✅, and on plain `alpine` until `apk add zstd` ⇒ hence `kit.tar.gz` / `docs pack` default to gzip; a
`.tar.zst` artifact is only allowed when the pack itself ships `zstd` in `bin/`). It does **not** have pixi, cargo, python, or network except
to the git host. Everything below is the *documented* offline path — the interesting part is that pixi
itself supports a **local channel**, so the kit is not a dead prefix, it is a package source.

> [!NOTE]
> The ladder below is not a ritual a human performs: **the assembler binary shipped inside every kit (D21) *is*
> §2.1–§2.6 as code** — it tries rung 3, then 2, lands on 5 only with an explicit warning, and prints which rung
> it took (`--print-rung`). Its contract is the measured `assemble.sh` oracle ✅
> ([nine outcomes](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox)); read the steps that
> follow as that contract's specification, and keep them as the manual fallback for when no kit binary exists.

**The default path is none of the ladder below** *(amended 2026-09-19 — "one command": the binary ships
inside the kit, and the airlocked host fetches nothing else)*:

```bash
git clone --depth 1 --single-branch --branch pixi-sandbox-dist https://github.com/acme/myproj.git kit
sh kit/assemble                          # verify → pick the host's binary → reconstruct; add --print-rung
```

`kit/assemble` — the entry shim [The Assembler Binary §2](/spec/assembler-binary.md#2-the-one-command)
specifies — is what a kit published by the action carries at its root. The manual steps that follow stay for
two reasons: they are the *specification* the shim and binary implement, and the fallback for a kit too old
to carry one.

### 2.1 Fetch, verify, install the drivers

```bash
git clone --depth 1 --branch pixi-sandbox-dist https://github.com/acme/myproj.git kit
cd kit
sha256sum -c SHA256SUMS                      # integrity BEFORE executing anything (exit 4 on mismatch)

export PIXISB_HOME="$HOME/.local"             # no root needed
install -m755 bin/pixi-$(uname -m)-unknown-linux-musl         "$PIXISB_HOME/bin/pixi"
install -m755 bin/pixi-sandbox-$(uname -m)-unknown-linux-musl "$PIXISB_HOME/bin/pixi-sandbox"
install -m755 bin/pixi-unpack-$(uname -m)-unknown-linux-musl  "$PIXISB_HOME/bin/pixi-unpack" 2>/dev/null || true
export PATH="$PIXISB_HOME/bin:$PATH"
pixi --version        # 0.81.0 — from OUR branch, not pixi.sh
```

### 2.2 Reconstruct: the ladder, now with the rungs the docs actually support

```bash
pixi sandbox reconstruct --mode auto        # tries these in order, prints which one it used
```

| # | Rung | What it does | You can then… | Basis |
|---|---|---|---|---|
| 1 | **cache** | copy `cache/pkgs/*` → `PIXI_CACHE_CONDA_PACKAGES_DIR` (`[cache] conda-packages` is the config form ✅), then `pixi install --frozen` | run **any** task, install **any** declared env, `pixi shell` | ✅ each piece; ⚠️ the composition is what M4.5 measures |
| 2 | **local channel** | point the workspace at the pack's `channel/` dir: `pixi project channel add file://$PWD/channel --no-install` ✅ then `pixi install --frozen` | run tasks **and** `pixi add` anything present in that channel — *offline solving works* | ✅ `file://` channels are documented ✅ (`channels = ["file:///abs/path"]`); ✅ offline mode counts a local channel as "available, no download needed" |
| 3 | **unpack** | `pixi-unpack -o .pixi -e envs/<env>` (real flags ✅) → prefix + `activate.sh` | `.pixi/envs/<env>` exists and is *installer-produced* (`Prefix::install`, `conda-meta/history` written ✅) → looks like a pixi env | ✅ read from `src/unpack.rs` |
| 4 | **foreign installer** | `micromamba create -p env --file environment.yml` / `conda env create` from the pack's own `environment.yml` ✅ | prefix without pixi at all; note pip side-effect caveat (§2.5) | ✅ documented by pixi-pack verbatim |
| 5 | **floor** | `tar -xf pack.tar` only | a directory tree. No `conda-meta`, **no prefix rewriting** → fine for self-contained binaries, not for `python`/`pip` entry points | ⚠️ inference from what `tar` does *not* do; treat as "rescue", never as install |

The point of rung 2 is that it turns the kit from *an environment* into *a registry*: on the target you can
`pixi add <package-that-happens-to-be-in-the-pack>` and pixi will solve offline, because
*"a conda package counts as available locally when it is already in the package cache, or when it is served
from a local (`file://`) channel, which needs no download either way"* ✅ — and if a solve is impossible,
*"the error names the packages that were ruled out"* ✅. That is exactly the behaviour `spec/docs-site.md §9.4` was
hoping for; it is why the guarantee moved from R3-tar-only to this rung.

### 2.3 The `.pixi/config.toml` a kit ships (why config-in-the-workspace beats touching `$HOME`)

pixi's config discovery puts `your_project/.pixi/config.toml` at **priority 10**, above every user and
system location ✅ — so a kit can configure pixi for that workspace without writing to `$HOME`, and
without the user's machine-wide settings leaking in:

```toml
# kit/workspace/.pixi/config.toml — generated by `reconstruct`
offline = true                      # never reach for conda.anaconda.org ✅
[cache]
conda-packages = "/var/tmp/myproj/pkgs"   # MUST be absolute; ~ expands; $VAR does NOT ✅
detached-environments = "/var/tmp/myproj/envs"
# netfs-redirect = "never"           # set this when /var/tmp is local and you want determinism ✅
pinning-strategy = "exact-version"  # `pixi add` on the target pins what the channel HAS ✅
[repodata-config]
disable-sharded = true              # a small local channel has no sharded repodata 🚧 confirm needed
# run-post-link-scripts = "insecure"  # ONLY if a package needs it — see §2.4
```

`detached-environments` as a *path* keeps the actual environments off the git working tree
(`NAME_OF_PROJECT-HASH/envs` + `solve-group-envs` ✅) — which matters in a sandbox where the repo dir is
snapshotted and `.pixi/envs/**` is 3 GB of churn. Its documented downside: the workspace no longer owns
its envs, so deleting a workspace needs manual cleanup ✅.

### 2.4 The three gotchas that bite in an airlock, and what to type

1. **post-link scripts are off by default** ✅ — pixi deems them insecure and skips them
   (`run-post-link-scripts = "insecure"` opts in, with a planned sandbox mode). Consequence: a package that
   *needs* post-link work (rare, mostly Windows/`python`-adjacent) installs half-complete and the failure
   looks like a missing file, not an error. `reconstruct` therefore runs a **smoke task** (`pixi run
   __selftest__`) after any install, before reporting success.
2. **`--offline` is not a total barrier.** ✅ It does not apply to build backends for source deps
   (`pixi-build-*` run as separate processes with their own clients), and PyPI resolution is delegated to
   `uv`, which gets *offline connectivity* but "cannot [be] restricte[d] to cached distributions, so a
   solve that succeeds may still require network access for PyPI" ✅. So: on an airlocked target, **avoid
   `[pypi-dependencies]` in any env you intend to re-solve** — pre-pack them (wheels only ✅) or vendor
   them as `.conda`.
3. **Cache paths are absolute, and `$HOME/.cache` may be NFS.** `[cache]` rejects relative paths at
   config-load time ✅ and auto-redirects some kinds when the root looks like a network filesystem ✅
   (`$SLURM_TMPDIR`/`$PBS_JOBFS`/`$SCRATCH`/`$TMPDIR` order, `PIXI_CACHE_NETFS_REDIRECT` to force). On a
   shared home dir that heuristic can move your cache mid-run; a kit that wants reproducibility pins
   `conda-packages` and sets `netfs-redirect = "never"` instead of hoping.

### 2.5 Two footnotes that save an afternoon

* `pixi-unpack` writes `activate.sh` **on the target**, with absolute paths, using `rattler_shell`
  (`--shell bash|zsh|xonsh|cmd|powershell|fish|nushell` ✅). So *never ship* an activation script in a kit —
  it is target-specific by construction, and shipping one is how "works on my machine" gets baked into a
  tarball.
* If you must use `conda`/`micromamba` on a pack (`--fallback conda`), note ✅: *"both `conda` and `mamba`
  are always installing pip as a side effect when they install python"*, unlike pixi → solver errors on
  packs built by pixi. Documented fixes: `pixi add pip` **into the source env** (so the pack has it), or
  `conda config --set add_pip_as_python_dependency false`.

### 2.6 What "done" means on the target

```bash
pixi run build && pixi run test      # tasks, offline, from a real pixi workspace
pixi list                            # reads the lockfile ✅ — proves env↔lock agreement
pixi sandbox kit verify .            # digests + the rung's file set (§9.3 of /spec/artifacts.md)
pixi sandbox doctor --json | head -20  # re-measures: hosts, tools, lockfile freshness
```

`pixi run` working, `pixi list` agreeing with the lockfile, and `kit verify` green is the definition of a
successful reconstruction. Anything less is a prefix, and the report says so (`reconstruct-report.json`
names the rung and what that rung cannot do).

---
