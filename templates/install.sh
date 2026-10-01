#!/bin/sh
# Download a checksum-verified pixi-sandbox release and scaffold this project.
#
# The VERSION default below is filled in at release time so a published one-liner always
# installs the binaries of its own release; it is never a hardcoded pin that can lag one
# release behind. Override it with PIXI_SANDBOX_VERSION to install a different release.
set -eu

REPOSITORY=${PIXI_SANDBOX_REPOSITORY:-Archont561/pixi-sandbox}
VERSION=${PIXI_SANDBOX_VERSION:-__VERSION__}
PROJECT_ROOT=${PIXI_SANDBOX_PROJECT_ROOT:-.}

case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) target=x86_64-unknown-linux-musl ;;
  Linux-aarch64|Linux-arm64) target=aarch64-unknown-linux-musl ;;
  Darwin-x86_64) target=x86_64-apple-darwin ;;
  Darwin-arm64) target=aarch64-apple-darwin ;;
  *) echo "unsupported connected-host platform: $(uname -s)-$(uname -m)" >&2; exit 2 ;;
esac

asset="pixi-sandbox-$target"
base="https://github.com/$REPOSITORY/releases/download/$VERSION"
work=$(mktemp -d "${TMPDIR:-/tmp}/pixi-sandbox-init.XXXXXX")
trap 'rm -rf "$work"' EXIT HUP INT TERM

curl -fsSL "$base/$asset" -o "$work/$asset"
curl -fsSL "$base/SHA256SUMS" -o "$work/SHA256SUMS"
expected=$(awk -v name="$asset" '$2 == name || $2 == "*" name { print $1 }' "$work/SHA256SUMS")
[ -n "$expected" ] || { echo "no checksum published for $asset" >&2; exit 1; }
case "$expected" in *[!0-9a-fA-F]*|'') echo "invalid checksum for $asset" >&2; exit 1 ;; esac
if command -v sha256sum >/dev/null 2>&1; then
  actual=$(sha256sum "$work/$asset" | awk '{print $1}')
else
  actual=$(shasum -a 256 "$work/$asset" | awk '{print $1}')
fi
[ "$actual" = "$expected" ] || { echo "checksum mismatch for $asset" >&2; exit 1; }

chmod +x "$work/$asset"
"$work/$asset" init --project-root "$PROJECT_ROOT" "$@"
printf '%s\n' "Scaffold complete. Commit the generated files and push to publish the sandbox branch."
