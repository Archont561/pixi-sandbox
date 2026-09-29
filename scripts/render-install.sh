#!/usr/bin/env bash
# Render templates/install.sh into the `install.sh` release asset.
#
# Usage: bash scripts/render-install.sh <TAG> <OUTPUT>
#   TAG      the release tag, e.g. v0.4.0 — becomes the script's default VERSION
#   OUTPUT   path to write, e.g. dist/install.sh
#
# The install one-liner is *rendered* here rather than copied from a committed
# `install.sh`, because a committed copy carries a VERSION default that only the
# release knows the right value for, so it silently lags: the published v0.3.0
# one-liner installed v0.2.0 binaries (task-2). Rendering means the default IS the
# tag being released, and "downloads a vX.Y.Z script, installs vX.Y-1.Z binaries"
# cannot be constructed.
#
# `sed`, not a template engine: this repo is pure shell/PowerShell/native Rust with
# zero Python runtime dependency (CONTEXT.md invariant 3), and this runs on all five
# release runners including windows-latest. One substitution does not earn a dependency.
#
# The rendered asset is deliberately NOT added to SHA256SUMS — that file stays the binary
# contract setup-pixi-sandbox reads, and install.sh verifies the binary it downloads against
# it. install.sh itself travels over `curl | sh` (trust-on-first-use via TLS), so it carries
# no checksum of its own.
set -euo pipefail

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || dirname "$SCRIPT_DIR")"
TEMPLATE="$REPO_ROOT/templates/install.sh"

TAG="${1:-}"
OUTPUT="${2:-}"
[ -n "$TAG" ] || die "usage: render-install.sh <TAG> <OUTPUT>"
[ -n "$OUTPUT" ] || die "usage: render-install.sh <TAG> <OUTPUT>"

# A version flows into a downloaded URL; keep it a strict semver core.
case "$TAG" in
  v[0-9]*.[0-9]*.[0-9]*) ;;
  *) die "tag '$TAG' is not vX.Y.Z" ;;
esac

[ -f "$TEMPLATE" ] || die "templates/install.sh not found at $TEMPLATE"

mkdir -p "$(dirname "$OUTPUT")"
sed "s|^VERSION=\${PIXI_SANDBOX_VERSION:-__VERSION__}$|VERSION=\${PIXI_SANDBOX_VERSION:-$TAG}|" \
  "$TEMPLATE" >"$OUTPUT"

# Refuse to ship anything but a script that will actually do the right thing.
grep -q "^VERSION=\${PIXI_SANDBOX_VERSION:-$TAG}$" "$OUTPUT" ||
  die "could not stamp $TAG into $OUTPUT — check the VERSION line in templates/install.sh"
if grep -q '__VERSION__' "$OUTPUT"; then
  die "__VERSION__ placeholder survived into $OUTPUT"
fi
head -1 "$OUTPUT" | grep -q '^#!/bin/sh$' || die "$OUTPUT lost its shebang"
sh -n "$OUTPUT" || die "$OUTPUT is not valid POSIX sh"
chmod +x "$OUTPUT"

log "→ $OUTPUT (default VERSION=$TAG)"
