---
type: Source Notes
title: "Manifest discovery, platform validation, offline reconstruction"
description: The second-round research that answered the five requirements: inventory tiers, platform existence, and the reconstruction rungs.
resource: https://github.com/Archont561/pixi-sandbox
tags: [research, rev2]
status: stable
confidence: verified
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WEB_SEARCH_RESULTS.md`], sections: ["15"] }
stale_after: 2026-12-19T00:00:00Z
sources:
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_manifest/, title: pixi.prefix.dev/latest/reference/pixi_manifest/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/cli/pixi/lock/, title: pixi.prefix.dev/latest/reference/cli/pixi/lock/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/cli/pixi/add/, title: pixi.prefix.dev/latest/reference/cli/pixi/add/ }
  - { id: pixiprefixdev-latest-reference, resource: https://pixi.prefix.dev/latest/reference/pixi_configuration/, title: pixi.prefix.dev/latest/reference/pixi_configuration/ }
  - { id: condaanacondaorg-conda-forge, resource: https://conda.anaconda.org/conda-forge, title: conda.anaconda.org/conda-forge }
  - { id: condaanacondaorg-conda-forge, resource: https://conda.anaconda.org/conda-forge, title: conda.anaconda.org/conda-forge }
---

# Manifest discovery, platform validation, offline reconstruction

## 15. Rev. 2 research: manifest discovery, platform validation, offline reconstruction

Prompted by the five new requirements (arbitrary env packing, detection, lockfile-derived bundling,
target-platform validation, self-hosted pixi + reconstruction). All findings below are what `spec/architecture.md §4.4` / §7.4 / §9.4 / §10.5 now depend on.

### 15.1 What a pixi manifest can declare (the discovery surface)

Source: [pixi manifest reference](https://pixi.prefix.dev/latest/reference/pixi_manifest/) ✅

| Shape | Example | Why discovery must handle it |
|---|---|---|
| Environments in `pixi.toml` | `[environments] test = ["test"]` | the shorthand form ≡ `{ features = ["test"] }` — a *string list*, not always a table |
| Environments in `pyproject.toml` | `[tool.pixi.environments]` | everything pixi reads from `pixi.toml` lives under `[tool.pixi.*]` in `pyproject.toml`; a detector that only reads `pixi.toml` silently finds zero envs in Python-flavoured repos |
| **Inline per-environment tables** | `[environments.dev.dependencies]` | dependencies may be declared *on the environment*, not only on a feature → enumeration has to read `[feature.*]`, `[environments.*]` **and** `[environments.*.dependencies]` |
| Feature attributes | `[feature.cuda] platforms = ["linux-64"]`, `channels`, `activation`, `system-requirements`, `constraints` | **per-feature `platforms` is the documented escape hatch** and the correct remediation for a package missing on one platform (e.g. `bun`/`win-64`, C1) |
| `solve-group`, `no-default-feature` | `{ solve-group = "py311", no-default-feature = true }` | two envs sharing a solve-group share a resolution ⇒ packing them separately is wasted bytes; `no-default-feature` means the "obvious" env set is *not* `default + everything` |
| `solve-strategy` | left-most feature wins | affects reproducibility of a re-solve, not the pack |

### 15.2 Platform existence: what pixi itself will and won't tell you

* `platforms` at workspace level and per feature ⇒ **"is `win-64` a target?" is first a manifest question,
  second a lockfile question, third a channel question.** `spec/toolchain-resolution.md §7.4` encodes exactly that order
  (P0 declared / P1 locked / P2 published).
* `pixi lock` **writes the lockfile without installing** ✅ [pixi lock](https://pixi.prefix.dev/latest/reference/cli/pixi/lock/)
  → the remediation for "declared but not locked" is safe to suggest inside a sandbox.
* `pixi list` is documented as *"a nicer view on the lockfile"* ✅ → good for proving an env resolves
  offline; it does not need the prefix.
* Env vars `PIXI_ENVIRONMENT_NAME` and `PIXI_ENVIRONMENT_PLATFORMS` ✅ → tasks that branch on them behave
  differently per rung, so `reconstruct` records them instead of assuming.

### 15.3 The offline/cache facts that make reconstruction defensible

| Fact | Source | Consequence for the kit |
|---|---|---|
| pixi keeps environments at **`.pixi/envs/<name>`** | ✅ [docs](https://pixi.prefix.dev/latest/) | a kit that untars to `./env` is *not* pixi-managed; reproduce the `.pixi/envs/` layout (`keep-env-names = true`) |
| Cache is **`$PIXI_CACHE_DIR/{pkgs,repodata}`** | ✅ docs | R1 = copy that dir + `pixi install --frozen`: no channel injection, no root, no network. Content-addressed, so copying is safe |
| Channels are searched **in declared order** | ✅ docs | `channel/` ordering must be preserved when a pack is re-used as a local index |
| `pixi add` accepts **an absolute path to a local `.conda`/`.tar.bz2`** (name taken from the filename) | ✅ [pixi add](https://pixi.prefix.dev/latest/reference/cli/pixi/add/) | the *verified* offline install primitive; why `kit` emits `.conda` files next to raw binaries |
| `[mirrors]` accepts `oci://` and `s3://` | ✅ [pixi configuration](https://pixi.prefix.dev/latest/reference/pixi_configuration/) | mirrors are for **remote exact-copy channels**; a local dir is served as a *channel* instead (see §16), so no design dependency on a directory mirror |
| `pixi project export conda-environment --environment E <file>` | ✅ docs | produces `environment.yml` per env ⇒ the micromamba path (R4) needs no pixi-pack at all — relevant because pixi-pack's release assets are unreachable here |

> [!NOTE]
> **Status update after a second pass (same day).** (1) is **resolved ✅**: `file://` channels *are*
> documented for pixi — both in the manifest (`channels = ["conda-forge",
> "file:///home/user/staged-recipes/build_artifacts"]` ✅) and on the CLI (`pixi project channel add
> file:///home/user/local_channel` ✅), and offline mode explicitly counts a local `file://` channel as
> "available, no download needed" ✅ [§16.1](/research/pixi-pack.md#161-file-channels-and-offline-mode--the-fact-that-changed-the-design).
> (2) is **still unverified but no longer load-bearing**: mirrors are documented as *exact copies of the
> original channel* ✅ (remote http(s)/`oci://`/`s3://`), and local serving goes through (1) instead.
> The **one** probe that still decides a default is step **3** (copied cache) plus the composition question
> "does pixi `--frozen`-install from a *pixi-pack-produced* `channel/`" — see §16.5.

**The probe that settles both, on a machine that has pixi** (≤ 10 min, no code to write — record the
outcome in `spec/docs-site.md §9.4` and drop its ⚠️ markers once these pass). Shape over exactness here: the
fetch line depends on what the box already has, the *four assertions* are the point:

```bash
mkdir -p probe/channel/linux-64 && cd probe
# 0) need one .conda: download it, or copy it out of a machine that already solved (cache dirs hold the
#    extracted tree, so the archive may have to come from `pixi-pack --directory-only`)
cp "$SRC"/zlib-1.3.1-*.conda channel/linux-64/          # or curl -LO --output-dir channel/linux-64 <url>

# 1) does pixi accept a directory as a CHANNEL?  ← answered by docs ✅; still worth running, because the
#    remaining question is the *composition*: a pixi-pack-produced channel dir, with pixi-pack's repodata.
printf '[workspace]\nname="p"\nchannels=["file://%s"]\nplatforms=["linux-64"]\n[dependencies]\nzlib="1.3.1"\n' "$PWD/channel" > pixi.toml
pixi install -v      # then repeat with `channel/` extracted from a real `pixi-pack` tarball, not a hand-made dir

# 2) does [mirrors] accept a directory? (documented as remote exact-copy channels ✅; this probe only
#    decides whether we can *advertise* it — the design no longer needs it)
printf '[mirrors]\n"https://conda.anaconda.org/conda-forge" = "file://%s"\n' "$PWD/channel" >> pixi.toml
rm -rf .pixi && pixi lock -v && pixi install -v

# 3) is a COPIED CACHE enough for an offline install? (this is rung R1, the preferred one)
pixi install                                  # populate the real cache first
cp -a "$PIXI_CACHE_DIR" ./.cache && rm -rf .pixi
PIXI_CACHE_DIR="$PWD/.cache" pixi install --frozen --offline
# also worth recording: (a) whether pixi needs repodata.json in channel/ (pixi-pack ships one per subdir ✅,
# `pixi publish` generates one for local channels via [index-config] ✅), and (b) whether
# `repodata-config.disable-sharded` must be set for a small local channel
```

* ⚠️ Steps 0–3 must run on a networked box: `conda.anaconda.org` and `prefix.dev` are reset here ✅
  (`environment/network-model.md §4.3`), so this sandbox can specify the probe but not execute it. Step 3's
  `--offline` is the flag whose behaviour the whole R1 rung rests on.*

### 15.4 Self-hosting notes gathered on the way

* pixi **0.81.0, released 2026-09-15** ✅ (latest at time of writing) — the version `[selfhost].pixi-version`
  pins in the design.
* `pixi exec --spec <pkg> -- <cmd>` and `pixi global install pixi-pack pixi-unpack` ✅ are the two ways a
  *networked* host obtains the packager; neither works here, which is the entire reason §10.5 exists.
* `pixi-pack --create-executable --pixi-unpack-source <path|url>` ✅ gives a supported way to point the
  self-extracting env script at **our** git-branch copy of `pixi-unpack` instead of GitHub releases.

---
