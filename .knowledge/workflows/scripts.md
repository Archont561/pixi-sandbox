---
type: Reference
title: "Copy-paste Scripts"
description: Runnable shell for both sides of the airlock, kept verbatim so a reviewer can diff intent against implementation.
resource: https://github.com/Archont561/pixi-sandbox
tags: [workflow, scripts]
status: stable
confidence: mixed
generated: { by: arena-agent/agent-mode, at: 2026-09-19T21:00:00Z }
legacy: { files: [`WORKFLOWS.md`], sections: ["5"] }
---

# Copy-paste Scripts

## 5. Copy-paste scripts

### 5.1 `scripts/bootstrap.sh` — the target's entry point (POSIX, no build step)

```sh
#!/bin/sh
# usage: bootstrap.sh <git-url> [dist-branch] [workspace-dir]
set -eu
URL="${1:?need the kit repo url}"; BRANCH="${2:-pixi-sandbox-dist}"; WS="${3:-.}"
BIN="${PIXISB_HOME:-$HOME/.local}/bin"; CACHE="${PIXISB_CACHE:-/var/tmp/$(id -un)/pixi-pkgs}"
T="$(mktemp -d)"; trap 'rm -rf "$T"' EXIT

git clone --depth 1 --single-branch --branch "$BRANCH" "$URL" "$T/kit"
cd "$T/kit"; sha256sum -c SHA256SUMS                      # fail loudly, never "retry"

mkdir -p "$BIN" "$CACHE"
for b in pixi pixi-sandbox pixi-unpack; do
  [ -x "bin/$b" ] && install -m755 "bin/$b" "$BIN/$b"    # already-tripled names from the kit build
done
PATH="$BIN:$PATH" pixi --version >/dev/null

cd "$WS"
PIXI_CACHE_CONDA_PACKAGES_DIR="$CACHE" PATH="$BIN:$PATH" pixi sandbox reconstruct \
  --from "$T/kit" --mode auto --print-rung
```

Deliberately: no `curl`, no `sudo`, no `python`, no `$HOME/.pixi` mutation (the kit's
`workspace/.pixi/config.toml` at priority 10 ✅ does the config work instead), and one output that matters
— `--print-rung` prints `1..5` so a calling agent can branch on the *strength* of what it got.

### 5.2 `scripts/use-pack.sh` — when you only have a pack, not a workspace

```sh
#!/bin/sh
# source me:  . scripts/use-pack.sh [pack.tar]
PACK="${1:-./environment.tar}"
[ -f "$PACK" ] || { echo "no pack at $PACK" >&2; return 1 2>/dev/null || exit 1; }
if command -v pixi-unpack >/dev/null 2>&1; then
  pixi-unpack -o "$PWD" -e ".local-env" "$PACK"            # installer: conda-meta + prefix fix ✅
  . ./.local-env/activate.sh                              # generated HERE, with local paths ✅
else
  tar -xf "$PACK"                                          # floor rung: channel dir + environment.yml ✅
  echo "no pixi-unpack: use 'micromamba create -p ./.local-env -f environment.yml'"
  echo "or add the channel offline:  pixi project channel add file://$PWD/channel --no-install"
fi
[ -d vendor ] && [ -f .cargo/config.toml ] && export CARGO_NET_OFFLINE=true   # only if truly vendored
```

### 5.3 `ci/republish.yml` skeleton (the pieces that are policy, not plumbing)

```yaml
jobs:
  lock-guard:                      # never auto-commit to a PR; print the command instead
    steps: [ { run: "pixi install --locked" }, { run: "cargo metadata --locked --offline" },
             { run: "bun ci" } ]
  artifacts:
    needs: lock-guard
    strategy: { matrix: { env: [default, rust, py], platform: [linux-64, osx-arm64] } }
    steps:
      - { run: "pixi sandbox kit build --env ${{ matrix.env }} --target ${{ matrix.platform }} --only-changed" }
      - { run: "pixi sandbox dist push --no-push" }        # staging only; a human/app pushes on main
  republish:
    needs: artifacts
    if: github.ref == 'refs/heads/main'
    steps: [ { run: "pixi sandbox dist push --with-self --with-pixi" },
             { run: "pixi sandbox dist tag --message \"kit @ ${{ github.sha }}\"" } ]
  reconstruct-e2e:
    needs: republish
    container: { image: alpine:latest }      # git + tar + zstd only — this is the airlock rehearsal
    steps: [ { run: "sh scripts/bootstrap.sh \"$GITHUB_SERVER_URL/$GITHUB_REPOSITORY\" pixi-sandbox-dist ." },
             { run: "pixi run test" } ]
```

`paths:`-filter the whole workflow to the seven files in [§0's](/workflows/lockfile-digest-map.md#0-one-table-because-everything-hangs-off-it)
table, or a `README.md` typo costs you a 300 MB pack (private-repo minutes: 2 000/month ✅).

---
