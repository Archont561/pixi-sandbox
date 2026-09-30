#!/usr/bin/env bash
# Print the project version, from the one place it is written down.
#
# `[workspace.package] version` in the root Cargo.toml is the single source of truth:
# every crate inherits it, `prepare-release.sh` stamps it, and everything else DERIVES
# it rather than restating it. The docs tasks in pixi.toml export the result as
# SANDBOX_VERSION so the site substitutes it at build time (docs/astro.config.mjs)
# instead of carrying ~60 literal tags that a release must rewrite.
#
# The sed is the same anchored expression prepare-release.sh and release-refs.sh use:
# only the top-level `version = "…"` starts at column 0, so a dependency pin
# (indented, or written `pkg = { version = … }`) can never match. Keeping the three
# scripts on one spelling is deliberate — the reader and the writer must agree on
# which line is the source of truth.
#
# Usage: scripts/version.sh          → prints e.g. 0.3.2 (no leading v, no newline chatter)

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

VERSION="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$REPO_ROOT/Cargo.toml" | sed 1q)"
if [ -z "$VERSION" ]; then
  echo "::error::no top-level 'version = \"…\"' in $REPO_ROOT/Cargo.toml" >&2
  exit 1
fi
printf '%s' "$VERSION"
