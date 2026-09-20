#!/usr/bin/env bash
# One-liner offline reconstruction from orphan branch, with PATH aliases.
# Usage: bash scripts/restore.sh [branch] [output-path]
#   branch defaults to sandbox/linux-64
#   output-path defaults to .
# After restore, sources .pixi/sandbox-env.sh and adds dev env to PATH.

set -euo pipefail

BRANCH="${1:-sandbox/linux-64}"
OUTPUT="${2:-.}"
TMPDIR="${TMPDIR:-/tmp}"
WORKTREE="$TMPDIR/sb-$$"

echo "→ fetching $BRANCH"
git fetch origin "$BRANCH:refs/remotes/origin/$BRANCH" --depth 1 || git fetch origin "$BRANCH"

echo "→ worktree $WORKTREE"
rm -rf "$WORKTREE"
git worktree add "$WORKTREE" "origin/$BRANCH" --force

# Prefer the self-contained root bootstrap produced by `pack --self-bin`; retain the
# nested path for branches generated before the root convenience copy existed.
BIN=""
for candidate in \
  "$WORKTREE/pixi-sandbox" \
  "$WORKTREE/pixi-sandbox.exe" \
  "$WORKTREE/.pixi-sandbox/tools/linux-64/pixi-sandbox" \
  "$WORKTREE/.pixi-sandbox/tools/linux-64/pixi-sandbox.exe" \
  "$WORKTREE/.pixi-sandbox/tools/win-64/pixi-sandbox.exe"; do
  if [ -x "$candidate" ] || [ -f "$candidate" ]; then
    BIN="$candidate"
    break
  fi
done

if [ -z "$BIN" ]; then
  echo "::error::No pixi-sandbox binary found in $WORKTREE"
  exit 1
fi

echo "→ doctor $BIN"
"$BIN" doctor --branch-location "$WORKTREE" --verify || true

echo "→ restore to $OUTPUT"
if [ -f "$WORKTREE/restore.sh" ]; then
  # New branches keep the restore policy next to the root binary.
  bash "$WORKTREE/restore.sh" "$OUTPUT"
else
  # Legacy branches may only have the nested self-binary and no launcher.
  if "$BIN" restore --branch-location "$WORKTREE" --output-path "$OUTPUT" --force 2>&1; then
    :
  else
    "$BIN" restore --branch-location "$WORKTREE" --path-to-main-repo-code "$OUTPUT" --force
  fi
fi

echo "→ cleanup worktree"
git worktree remove "$WORKTREE" --force || rm -rf "$WORKTREE"

# Wire PATH aliases like setup-pixi
if [ -f "$OUTPUT/.pixi/sandbox-env.sh" ]; then
  echo "→ sourcing $OUTPUT/.pixi/sandbox-env.sh"
  # shellcheck disable=SC1090
  source "$OUTPUT/.pixi/sandbox-env.sh"
  export PATH="$PWD/.pixi/envs/dev/bin:$PATH"
  echo "PATH now includes:"
  echo "  $PWD/.pixi/tools/linux-64"
  echo "  $PWD/.pixi/envs/dev/bin"
  echo "  pixi() function → bundled pixi"
  echo "Try: pixi --version; cargo --version; cargo check --offline"
else
  echo "restore complete but no sandbox-env.sh found"
fi
