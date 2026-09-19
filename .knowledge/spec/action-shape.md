---
type: Design Spec
title: "The Action Shape (v1)"
description: A public GitHub Action named pixi-sandbox whose CI steps run the pixi-sandbox Rust binary (pack/publish), packing pixi environments, the vendored crate graph and the pixi executable onto a named branch — and shipping that same binary as the kit's assembler.
resource: https://github.com/Archont561/pixi-sandbox
tags: [spec, action, ci, distribution]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-20T00:15:00Z }
verified:
  - { by: process:sandbox-measurement, at: 2026-09-19T00:38:00Z }
sources:
  - { id: quantco-pixi-pack, resource: https://github.com/Quantco/pixi-pack, title: Quantco/pixi-pack — releases carry pixi-pack + pixi-unpack for 8 platforms with digests ✅ }
  - { id: withastro-action, resource: https://github.com/withastro/action, title: withastro/action — the composite-action shape being copied }
  - { id: prefix-dev-setup-pixi, resource: https://github.com/prefix-dev/setup-pixi, title: prefix-dev/setup-pixi — the pixi installer the action wraps }
legacy: { files: [`DESIGN.md`], sections: ["9", "10"] }
---

# The Action Shape (v1)

**One sentence:** the product is **a public GitHub Action (`pixi-sandbox`) whose steps run one Rust binary** —
CI calls its packing verbs (`pixi-sandbox pack` / `publish`), the branch carries the payloads, and the *same
binary* ships inside the kit as the assembler that does the unboxing (`pixi-sandbox reconstruct`). The user
installs nothing; the binary is built in CI and mirrored, digest-pinned, like the `pixi` drivers (D19's
distribution model, delivered through D21's Rust artifact).

> **The deliverable is a binary; the spec you are reading was validated by a script.** Under D21 the artifact is
> compiled Rust — but this sandbox cannot compile, so the behaviour was first proven as a 183-line POSIX
> `assemble.sh`, executed here against fixture kits (nine outcomes ✅). That script is now frozen as the
> **behavioural oracle**: its rung ladder, exit codes and measured outcomes are the acceptance vectors the
> compiled assembler must reproduce vector-for-vector. See [The Assembler Binary](/spec/assembler-binary.md) and
> [Testing Strategy](/spec/testing-strategy.md). Everything binary-agnostic on this page — the branch layout, the
> four CI-push rules, the relocation trap, the runner-matrix rule — is part of that contract.

Why the action shape (D19) plus the binary artifact (D21) is better than either alone:

| Problem in a pure CLI shape | In the Action shape, binary artifact |
|---|---|
| The tool must be built by *someone*, and the airlock cannot build it ⇒ the whole bootstrap/seed ceremony ([§11](/spec/bootstrap.md), D11) | **the binary is built in CI (networked) and mirrored into the kit** with a sha256 — the target executes a verified blob, never compiles; no seed ceremony ⚠️ (the risk register holds this open until M1.5) |
| `pixi sandbox reconstruct` re-implements the ladder in Rust, then must be *tested by compiling it* | it is tested **twice**: the ladder already ran as `assemble.sh` here ✅ (nine outcomes, table in [Running the Action](/workflows/action-run.md)), and the Rust port must reproduce those exact vectors in CI ([Testing Strategy](/spec/testing-strategy.md)) |
| Distribution story: "install our binary from a branch" (a mirror of pixi's own release assets, which are blocked) | `uses: you/pixi-sandbox@<sha>` — Actions *is* the distribution channel, and it is a host the sandbox allows ✅; the binary rides the kit branch, not a release asset |
| Local ergonomics are the reason a binary exists | Kept and *promoted*: the CLI's `pack`/`publish`/`reconstruct` verbs **are** the action and the kit; `plan`/`explain`/`doctor` ergonomics stay v2; the action's `--dry-run` twin is `push: false` staging + a printed manifest ✅ |

# Contract

Inputs (validated by `js-yaml` in the author's box ✅ — see [Testing Without a Compiler](/spec/testing.md)):
`branch` (required), `environments`, `platforms`, `bundle-crates`, `ship-pixi`, `pixi-version`,
`pixi-pack-version`, `push`, `extra-files`. Outputs: `manifest`, `revision`, `bytes`.

The action absorbs exactly three verbs of the old surface — **`kit build`** (pack envs via `pixi exec --spec
pixi-pack@0.7.11 -- pixi-pack …`, and `cargo vendor` when `bundle-crates=true`), **`dist push`** (plumbing-only
push to `inputs.branch`), and **`reconstruct`**'s *target-side half*, which it does not implement but instead
**emits as `assemble.sh`** next to the payloads so the branch is self-opening. Everything else in
[Command Surface](/spec/cli.md) is either a CI shell step or a v2 nicety.

# `action.yml`, validated as a document

The whole contract is 73 lines of YAML — small enough to review in one screen, and `npx js-yaml action.yml`
passes on it here ✅ (the first draft did not: `inputs.environments:{ description: … }` is invalid YAML, a missing
space before a flow map — which is precisely the class of bug that a compiled CLI would have hidden behind a
`#[derive(Parser)]` and a CI cycle):

```yaml
name: pixi-sandbox
description: >-
  Pack pixi environments (and optionally the vendored crate graph and the pixi executable)
  onto an orphan branch of the calling repository, and ship the pixi-sandbox binary as the
  kit's assembler for the far side of the airlock.
author: Archont561
branding:
  icon: package
  color: purple
inputs:
  branch:
    description: Orphan branch to publish onto
    required: true
    default: pixi-sandbox-dist
  environments:
    description: Space-separated pixi environments to pack
    required: false
    default: default
  platforms:
    description: Space-separated conda subdirs; the matrix keys on these
    required: false
    default: linux-64
  bundle-crates:
    description: Ship the vendored crate graph so the target can recompile
    required: false
    default: "false"
  ship-pixi:
    description: Put the pixi executable in bin/ on the branch
    required: false
    default: "true"
  pixi-version:
    description: setup-pixi pin, passed through
    required: false
    default: v0.81.0
  pixi-pack-version:
    description: pixi-pack pin used via pixi exec
    required: false
    default: 0.7.11
  push:
    description: Set false to stage only (fork PRs have no write token)
    required: false
    default: "true"
  extra-files:
    description: Newline-separated paths to include verbatim, e.g. docs/dist
    required: false
outputs:
  manifest:
    description: dist-manifest.json path in the pushed tree
    value: ${{ steps.publish.outputs.manifest }}
  revision:
    description: SHA of the pushed branch tip
    value: ${{ steps.publish.outputs.revision }}
  bytes:
    description: Total payload bytes pushed
    value: ${{ steps.publish.outputs.bytes }}
runs:
  using: composite
  steps:
    - uses: actions/checkout@v4
    - uses: prefix-dev/setup-pixi@v0.9.0
      with:
        pixi-version: ${{ inputs.pixi-version }}
        environments: ${{ inputs.environments }}
        cache: "true"
    - name: pack
      shell: bash
      run: ${{ github.action_path }}/scripts/pack.sh
    - name: assemble
      shell: bash
      run: ${{ github.action_path }}/scripts/make-assemble.sh
    - name: publish
      id: publish
      shell: bash
      run: ${{ github.action_path }}/scripts/publish.sh
```

Steps `pack` / `assemble` / `make-assemble.sh` / `publish.sh` are the four jobs of the old verbs; `make-assemble.sh`
exists so the script shipped *inside* a kit is byte-identical to the one reviewed here — the kit's copy is
`sha256`-pinned in `dist-manifest.json`, so a consumer can prove which `assemble.sh` they ran ✅.

> **D21 delta to this document (accepted 2026-09-19):** the block above is preserved byte-for-byte because it is
> the YAML that `js-yaml` validated here ✅ — but under D21 its three `run:` steps become invocations of the Rust
> binary (`pixi-sandbox pack`, `pixi-sandbox publish`, with `reconstruct` shipped inside the kit rather than
> generated as a script), one `fetch-tool` step prepends them (digest-pinned prebuilt binary, falling back to a
> cached `cargo build`), and `make-assemble.sh` disappears — `publish` writes the same binary it is running as
> into the kit's `bin/`, so kit copy == builder copy by construction, recorded in `dist-manifest.json`. Inputs,
> outputs, permissions and the four CI-push rules below are unchanged. See
> [The Assembler Binary §3](/spec/assembler-binary.md) for the binary's own distribution.

# Branch layout the action pushes
```
pixi-sandbox-dist (orphan, force-updatable, one commit per run)      ← KIT branch: small index
├── README.md  AGENTS.md         ← rendered by publish; travel with the branch
├── SHA256SUMS                   ← `sha256sum -c` is step 1 of the reconstructor, not a suggestion
├── dist-manifest.json           ← per-artifact sha256 + git-oid + env/platform keys (§9.2)
├── manifest.tsv                 ← tab-separated payload index the reconstructor reads (§10.6)
├── workspace/{pixi.toml,pixi.lock}   # copied verbatim, never regenerated
└── bin/                              # each mirrored + digest-pinned
    ├── pixi-sandbox-<triple>         #   ← the assembler binary (D21): `reconstruct` for the target
    ├── pixi-<triple>                 #   → R2 (`pixi install --frozen`) and every `pixi run` afterwards
    └── pixi-unpack-<triple>          #   → R3, the rung that makes a prefix *installed* (D14)

pixi-sandbox-dist-envs[-N]       ← PAYLOAD branches (sharded by byte budget, §10.6)
└── packs/<ws>-<env>-<plat>.tar[.part-*]   # pixi-pack output, split if over the blob budget

pixi-sandbox-dist-vendor[-N]
└── vendor/vendor.tar.gz[.part-*]          # only when bundle-crates=true
```

The kit branch is deliberately small; the heavy payloads live on the `-envs`/`-vendor` branches and are fetched
lazily by the reconstructor ([§10.6](/spec/git-registry.md)). The reconstructor is the **assembler binary**
`bin/pixi-sandbox-<triple>` (D21) — it looks up `bin/<name>-$(uname -m)-unknown-linux-musl` for `pixi`,
`pixi-unpack` and itself, falling back to the bare name ✅ (measured against fixtures where only some drivers
existed; a missing `pixi-unpack` is what produced T2's descent to rung 5). Without `pixi-unpack` in the kit the
target can only ever reach rung 2 or 5 — which is exactly the failure the mirror in
[Research: pixi-pack](/research/pixi-pack.md) exists to prevent.

No `node/` row: the JS component was retired (D20) — a JS lockfile stays a CI assertion (`bun ci`), never a payload.

# Re-location: the trap that decides the artifact format

The tempting shortcut — *"install the env in CI, `tar` up `.pixi/envs`, push that"* — produces a prefix whose
absolute paths, shebangs and `conda-meta` records point at `/home/runner/.pixi/envs/<env>` 🚧. That is why the
action ships **`pixi-pack` output** instead: its tarball is a channel-shaped artifact that R2/R3 consume, and
`Prefix::install` (via `pixi-unpack`) is what rewrites the prefix ✅. The reconstructor — the assembler binary,
whose contract was proven as `assemble.sh` — therefore:

* prefers `pixi-unpack` when the kit carries it (**rung 3**), else `pixi install --frozen` against the kit's
  `file://` channel (**rung 2**) when a `pixi` exists, and only then falls back to `tar -xf` (**rung 5**) —
  *with an explicit warning that paths are not rewritten* ✅ (measured: `rung=5` printed alongside the warning);
* **never** relocates a raw env tar silently, and
* writes `.pixi/config.toml` (`offline`, absolute `[cache]`, `pinning-strategy = exact-version`) at priority 10 ✅.

# Runner matrix = platform coverage

"Installed in CI" is a *platform claim*: a pack made on `ubuntu-latest` unpacks only on `linux-64` ✅
(same-platform-only is pixi-pack's documented constraint). So `platforms` is not cosmetic — each entry needs a
matching runner (`macos-14` for `osx-arm64`, `windows-latest` for `win-64`), and the action must either matrix
itself per platform or record `omitted[]` for what it could not build ⚠️. For Linux-only targets one runner is
enough, which is why `platforms` defaults to `linux-64`.

# Pushing from Actions — four rules that bite

1. `permissions: { contents: write }` and nothing else; `pages: write`+`id-token: write` only in the docs job.
2. **Pushes made with `GITHUB_TOKEN` do not trigger workflows** ✅ — that is what stops
   "action pushes to dist → dist push triggers action" from looping; if a PAT is used instead, add
   `[skip ci]` to the commit message ⚠️.
3. Fork PRs have **no write token** ⇒ `push: false` must stage and print the manifest, never fail the run.
4. **Git blobs are forever.** A 227 MB payload (measured size of `node_modules` for one Astro site ✅) on a
   branch is a permanent repo cost even after `keep-last = 3` pruning of *references*; the escapes are LFS opt-in
   or `git bundle` for the air-gap hop ([§10.4](/spec/git-registry.md)).

# `assemble.sh` — the measured oracle

Not pseudocode: this is the script that produced the nine measured outcomes in
[Running the Action](/workflows/action-run.md#the-nine-outcomes-measured-in-this-sandbox). Under D21 it is the
**frozen behavioural oracle** of the Rust assembler: the binary ships in kits, this script defines what
"reconstruct correctly" means, and CI's L2 integration tests replay these exact vectors against the compiled
artifact ([Testing Strategy](/spec/testing-strategy.md)).

```sh
#!/bin/sh
# assemble.sh — target-side half of the `pixi-sandbox` GitHub Action (POSIX sh, no build step).
# Usage: sh assemble.sh --from <kit-dir> [--env NAME]… [--with-vendor] [--workspace DIR]
#                        [--mode auto|unpack|tar] [--pack-format auto|pack|raw] [--promote-path] [--print-rung]
set -eu

FROM=""; WORKSPACE="."; WANT_VENDOR=0; MODE="auto"; PACK_FORMAT="auto"; RUNG=""; PRINT=0; SELFTEST=0; PROMOTE=0
ENVS=""
while [ $# -gt 0 ]; do
  case "$1" in
    --from)       FROM="$2"; shift 2 ;;
    --workspace)  WORKSPACE="$2"; shift 2 ;;
    --env)        ENVS="$ENVS $2"; shift 2 ;;
    --with-vendor) WANT_VENDOR=1; shift ;;
    --mode)       MODE="$2"; shift 2 ;;
    --pack-format) PACK_FORMAT="$2"; shift 2 ;;
    --promote-path) PROMOTE=1; shift ;;
    --print-rung) PRINT=1; shift ;;
    --self-test)  SELFTEST=1; shift ;;
    *) printf 'error[E-USAGE]: unknown flag: %s\n = try: assemble.sh --help\n' "$1" >&2; exit 1 ;;
  esac
done
[ -n "$FROM" ] || { echo "error[E-USAGE]: --from <kit-dir> is required" >&2; exit 1; }
[ -d "$FROM" ] || { echo "error[E-UNAVAILABLE]: no kit at $FROM" >&2; exit 5; }
# Drivers land INSIDE the workspace by default: a shared $HOME/bin is where a real, possibly newer
# pixi already lives, and an unboxing script has no business overwriting that ⚠️
BIN="${PIXISB_BIN_DIR:-$WORKSPACE/.pixi/bin}"
export PATH="$BIN:$PATH"

say() { printf '  %s\n' "$1"; }
fail() { printf 'error[E-%s]: %s\n' "$1" "$2" >&2; exit "$3"; }

# 1 · integrity BEFORE executing anything that came from the branch
if [ -f "$FROM/SHA256SUMS" ]; then
  ( cd "$FROM" && sha256sum -c SHA256SUMS >/dev/null ) \
    || fail INTEGRITY "SHA256SUMS mismatch — re-fetch the branch, do not retry" 4
  say "integrity: sha256 verified ✅"
else
  say "integrity: no SHA256SUMS in kit (refusing to run binaries without it)"
  [ "$SELFTEST" = 1 ] || fail INTEGRITY "missing SHA256SUMS" 4
fi

# 2 · install the drivers (pixi is part of the kit: no pixi.sh, no curl, no sudo)
mkdir -p "$BIN"
# 2a · never inherit another kit's driver. Without this, a kit that ships no pixi-unpack
#      "succeeds" with whatever the *previous* assemble left behind — a stale, differently-pinned
#      binary, which defeats the point of mirroring drivers into the kit. We only delete inside the
#      directory we own; an explicit PIXISB_BIN_DIR is the user's, so we say what we left alone.
case "$BIN" in
  "$WORKSPACE/.pixi/bin") for b in pixi pixi-unpack pixi-sandbox; do rm -f "$BIN/$b"; done ;;
  *) for b in pixi pixi-unpack pixi-sandbox; do
       [ -e "$BIN/$b" ] && say "note: left existing $BIN/$b in place (PIXISB_BIN_DIR is yours)"
     done ;;
esac
for b in pixi pixi-unpack pixi-sandbox; do
  src="$FROM/bin/$b-$(uname -m)-unknown-linux-musl"
  [ -f "$src" ] || src="$FROM/bin/$b"
  if [ -f "$src" ]; then install -m 755 "$src" "$BIN/$b"; say "driver: $b installed"; fi
done
printf '%s\n' "$FROM" > "$BIN/.pixi-sandbox-kit"

# 2c · resolve drivers by provenance, not by PATH luck: the kit's own verified copies win,
#      and a foreign one is reported rather than silently used (cargo is the exception — it is
#      the user's toolchain, and the design says so in D17/§7.4)
driver() {
  [ -x "$BIN/$1" ] && { printf '%s' "$BIN/$1"; return 0; }
  command -v "$1" 2>/dev/null || return 1
}
PIXI="$(driver pixi || true)"
[ -n "$PIXI" ] || fail UNAVAILABLE "no pixi binary in the kit and none on PATH" 5
[ "$PIXI" = "$BIN/pixi" ] || say "warning[W-PROVENANCE]: pixi resolved to $PIXI, outside the verified kit"
UNPACK="$(driver pixi-unpack || true)"
[ -n "$UNPACK" ] || say "rung 3 unavailable (no kit pixi-unpack) — auto mode will land on the tar floor"

# 2b · expose the drivers on PATH in a way that outlives this process
#      (the export above only helps *this* script; a human or a later CI step needs persistence)
ENVRC="$WORKSPACE/.pixi/assemble.env"
mkdir -p "$(dirname "$ENVRC")"
#      deliberately only PATH: we never move pixi's global home (PIXI_HOME) or rewrite $HOME
#      unless --promote-path is given — the workspace's own .pixi/config.toml does the rest
printf 'export PATH="%s:$PATH"\n' "$BIN" > "$ENVRC"
if [ -n "${GITHUB_PATH:-}" ]; then
  printf '%s\n' "$BIN" >> "$GITHUB_PATH"          # inside Actions: later steps *and jobs* see pixi
  say "path: $BIN appended to \$GITHUB_PATH"
fi
if [ "$PROMOTE" = 1 ]; then
  for rc in "$HOME/.profile" "$HOME/.bashrc"; do
    [ -f "$rc" ] || continue
    grep -qs '# pixi-sandbox assemble.sh' "$rc" || printf '\n# pixi-sandbox assemble.sh\n. "%s"\n' "$ENVRC" >> "$rc"
  done
  say "path: rc files point at $ENVRC (idempotent)"
else
  say "path: source \"$ENVRC\"  —  or re-run with --promote-path to edit your rc files"
fi

# 3 · materialise a WORKSPACE, never a regenerated one
mkdir -p "$WORKSPACE/.pixi/envs"
for f in pixi.toml pixi.lock; do
  [ -f "$FROM/workspace/$f" ] && cp -f "$FROM/workspace/$f" "$WORKSPACE/$f"
done
[ -f "$WORKSPACE/pixi.lock" ] || say "warning[W-STALE]: no pixi.lock — pixi install --frozen will refuse, which is correct"
cat > "$WORKSPACE/.pixi/config.toml" <<'EOC'
offline = true
pinning-strategy = "exact-version"
[cache]
conda-packages = "/var/tmp/pixi-sandbox/pkgs"
EOC

# 4 · unpack each environment: an installer-made prefix beats copied files (§2.1–§2.6 of
#     /workflows/airlock-clone.md is this block's specification)
pack_shape() {            # a pixi-pack tarball is channel-shaped; a raw env tar is a prefix tree
  tar -tf "$1" 2>/dev/null | grep -q -e pixi-pack.json -e 'channel/' && printf pack || printf raw
}
try_foreign_installer() {  # rung 4: micromamba/conda create from the pack's environment.yml
  for mm in micromamba conda; do
    command -v "$mm" >/dev/null 2>&1 || continue
    case "$mm" in
      micromamba) "$mm" create -y -p "$1/.pixi/envs/$2" -f "$3/environment.yml" >/dev/null ;;
      *)          "$mm" env create -p "$1/.pixi/envs/$2" -f "$3/environment.yml" >/dev/null ;;
    esac || return 1
    printf '%s' "$mm"
    return 0
  done
  return 1
}
WORKSPACE_2_DONE=0
for e in $ENVS; do
  pack="$(ls "$FROM"/packs/*"$e"*.tar 2>/dev/null | head -1 || true)"
  [ -n "$pack" ] || fail NOT-DETECTED "no pack for env '$e' (kit has: $(ls "$FROM"/packs 2>/dev/null | tr '\n' ' '))" 7
  shape="$PACK_FORMAT"
  [ "$shape" = auto ] && shape="$(pack_shape "$pack")"
  case "$MODE" in
    unpack|auto)
      if [ -n "$UNPACK" ]; then
        "$UNPACK" -o "$WORKSPACE/.pixi" -e "envs/$e" "$pack" \
          || fail RECONSTRUCT "pixi-unpack failed for env $e" 8
        RUNG=3; say "rung 3: $e unpacked by $UNPACK (installer-made prefix)"; continue
      fi
      [ "$MODE" = unpack ] && fail UNAVAILABLE "--mode unpack, but the kit ships no pixi-unpack" 5
      ;;
  esac
  if [ "$shape" = pack ]; then
    # no unpacker + a channel-shaped pack: rung 2 then rung 4, never a blind tar
    if [ -d "$FROM/channel" ] && [ "$WORKSPACE_2_DONE" = 0 ]; then
      "$PIXI" project channel add "file://$FROM/channel" --no-install >/dev/null 2>&1 \
        || say "warning[W-CHANNEL]: could not register file://$FROM/channel with pixi"
      if ( cd "$WORKSPACE" && "$PIXI" install --frozen >/dev/null ); then
        RUNG=2; WORKSPACE_2_DONE=1
        say "rung 2: workspace installed offline against file://$FROM/channel (pixi.lock untouched)"
        continue
      fi
    fi
    mm="$(try_foreign_installer "$WORKSPACE" "$e" "$FROM/channel")" && {
      RUNG=4; say "rung 4: $e created by $mm from the pack's environment.yml"; continue; }
    fail UNAVAILABLE "no kit pixi-unpack, rung 2 unavailable, no micromamba/conda on PATH — refusing to tar a channel-shaped pack (see /workflows/airlock-clone.md §2.4)" 5
  fi
  # raw env tar: files only, said out loud, because relocating a prefix is what breaks shebangs
  mkdir -p "$WORKSPACE/.pixi/envs/$e"
  tar -xf "$pack" -C "$WORKSPACE/.pixi/envs/$e" \
    || fail RECONSTRUCT "tar -xf $pack failed" 8
  RUNG=5
  say "rung 5 (tar): prefix paths are NOT rewritten — fine for relocatable trees, wrong for python entry points"
done

# 5 · the crate graph, wired without editing the repo
if [ "$WANT_VENDOR" = 1 ]; then
  [ -f "$FROM/vendor/vendor.tar.gz" ] || fail NOT-DETECTED "kit has no vendor payload (CI ran with bundle-crates=false)" 7
  mkdir -p "$WORKSPACE/.pixi/vendor"
  tar -xzf "$FROM/vendor/vendor.tar.gz" -C "$WORKSPACE/.pixi/vendor"
  mkdir -p "$WORKSPACE/.cargo"
  printf '[source.crates-io]\nreplace-with = "kit-vendored"\n\n[source.kit-vendored]\ndirectory = "%s/.pixi/vendor/vendor"\n\n[net]\noffline = true\n' "$WORKSPACE" > "$WORKSPACE/.cargo/config.toml"
  say "vendor: directory source wired via .cargo/config.toml (kit-scoped, not your repo's)"
  if command -v cargo >/dev/null 2>&1; then
    ( cd "$WORKSPACE" && cargo metadata --locked --offline >/dev/null ) \
      || fail VENDOR-INCOMPLETE "cargo metadata --offline failed" 8
    say "vendor: cargo metadata --locked --offline ✅"
  else
    say "vendor: cargo not on PATH — compile-on-target needs `rust` in the env (see D17/§7.4)"
  fi
fi

[ "$PRINT" = 1 ] && printf 'rung=%s\n' "${RUNG:-none}"
say "done: pixi run <task> now resolves against $WORKSPACE/.pixi/envs (drivers in $BIN)"
```

# What v1 still does not do (and what stays v2)

* **No target-side `Inventory` in the reconstructor.** Rung selection is a small decision table, not the L0→L3
  `Inventory` ([§4.4](/spec/architecture.md)); a wrong-platform pack fails at the manifest/integrity step
  (`dist-manifest.json` carries env/platform keys ✅), not with a diagnostic about *your* machine.
* **No `--json`/`--dry-run` everywhere, no `explain`, no `doctor` on the target.** Refusals are typed and
  name the failed check ([Error Taxonomy](/spec/error-taxonomy.md)), but the interactive ergonomics stay v2.
* **Nothing outside GitHub Actions.** "Pack a stranger's repo locally in 30 seconds" is a v2 verb — although
  under D21 the *same binary* already carries the code for it; v2 is an unlock, not a rewrite ⚠️.
* **Cadence is CI's:** the digest-keyed `--only-changed` idea survives as a manifest comparison inside
  `pixi-sandbox pack` (`dist-manifest.json` already has this `pixi.lock` digest ⇒ skip packing) ✅.

The honest summary: **D19 deleted the bootstrap and distribution problems; D21 put the binary back without
resurrecting either**, because the binary is *mirrored*, not installed — built in CI where network exists,
digest-verified before execution on a target where it does not. The measured `assemble.sh` oracle is the
guarantee that the compiled artifact behaves exactly like the design that ran here.

