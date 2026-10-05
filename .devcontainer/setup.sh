#!/usr/bin/env bash
# Devcontainer post-create setup. Kept out of devcontainer.json so each step
# can be commented and the JSON stays a one-liner (same approach as
# Archont561/qgis-rs).
set -euo pipefail

pixi --version

# Baseline CLI tooling that the pixi image does not ship with.
pixi global install git gh

# A C toolchain is needed to build the Rust crates. The conda compiler
# binaries are prefixed with the target triple, so expose the unprefixed
# names (cc/gcc/ar) that build scripts expect.
pixi global install \
  --expose cc \
  --expose gcc \
  --expose ar=x86_64-conda-linux-gnu-ar \
  c-compiler

# Project environments, pinned to the lockfile for reproducibility.
pixi install --locked --all

# Docs site dependencies (bun install --frozen-lockfile at the workspace root).
pixi run docs-install

# ---------------------------------------------------------------------------
# opencode: the agent CLI. A developer tool, so it is installed here rather than
# as a pixi task — `pixi.toml` describes what the repository is built, tested and
# released with, and a 185 MB agent binary is none of those. `bun` comes from the
# `web` *feature*, which task-70 folded into the single `default` environment that
# `pixi install --locked --all` above already materialised — hence `pixi run bun`
# and not `pixi run -e web …`, which pixi rejects as an unknown environment.
#
# bun's global bin dir (`bun pm bin -g`, which follows $BUN_INSTALL rather than a
# hardcoded $HOME/.bun/bin) is on nobody's PATH, so link the binary into a directory
# that already is: /usr/local/bin first (the container is root), ~/.local/bin second.
# The install itself still succeeded if neither is writable, so report where it
# landed instead of failing the container over a $PATH detail.
# ---------------------------------------------------------------------------
pixi run bun add -g opencode-ai@latest

BUN_BIN="$(pixi run bun pm bin -g | tail -n1)"
OPENCODE="$BUN_BIN/opencode"
if ln -sfn "$OPENCODE" /usr/local/bin/opencode 2> /dev/null; then
  OPENCODE=opencode
elif mkdir -p "$HOME/.local/bin" 2> /dev/null && ln -sfn "$OPENCODE" "$HOME/.local/bin/opencode" 2> /dev/null; then
  OPENCODE="$HOME/.local/bin/opencode"
else
  echo "opencode installed to $BUN_BIN, which is not on PATH: neither /usr/local/bin nor ~/.local/bin is writable"
fi

# The model catalogue is a separate cache (~/.cache/opencode/models.json, rebuilt from
# models.dev) and a binary upgrade does not invalidate it, so refreshing is its own step —
# otherwise a new release lists stale models.
"$BUN_BIN/opencode" models --refresh

cat <<EOF

────────────────────────────────────────────────────────────
opencode is ready. Start a session in this Codespace with:

  $OPENCODE -m opencode/big-pickle

────────────────────────────────────────────────────────────
EOF
