#!/usr/bin/env bash
# Resolve the next release version and stamp it into the repository, without touching git.
#
# Usage: bash scripts/prepare-release.sh [version]
#   version   the release to prepare. Accepts:
#               auto            derive from conventional commits (convco --bump) — default
#               major|minor|patch
#                               force that bump level regardless of the commits
#               vX.Y.Z | X.Y.Z  an explicit version
#             When omitted, PIXI_SANDBOX_RELEASE (env) is used, else `auto`.
#
# What it does (and deliberately does NOT do):
#   * resolves the target version with convco (respecting .versionrc / preMajor),
#   * rewrites the single top-level `version = "…"` in Cargo.toml and pixi.toml,
#   * regenerates CHANGELOG.md from the conventional-commit history (convco changelog).
# It never commits, tags, or pushes — that is the release workflow's job, so a human or
# a dry-run can review the diff first. The only value on stdout is `vX.Y.Z`; every log
# line goes to stderr so callers can capture the version with a plain command substitution:
#
#   VERSION="$(bash scripts/prepare-release.sh auto)"
#
# Requires convco on PATH (it is in the `default` pixi environment: `pixi run …`).

set -euo pipefail

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

command -v convco >/dev/null 2>&1 || die "convco not found on PATH — run through pixi (e.g. 'pixi run prepare-release')"

SELECTOR="${1:-${PIXI_SANDBOX_RELEASE:-auto}}"

# convco always prints the bare semver here; the leading `v` is added once, below.
resolve_version() {
  case "$1" in
    auto) convco version --bump ;;
    major) convco version --bump --major ;;
    minor) convco version --bump --minor ;;
    patch) convco version --bump --patch ;;
    v[0-9]*) printf '%s\n' "${1#v}" ;;
    [0-9]*) printf '%s\n' "$1" ;;
    *) die "unrecognised version selector: '$1' (use auto|major|minor|patch|vX.Y.Z)" ;;
  esac
}

SEMVER="$(resolve_version "$SELECTOR")"
SEMVER="$(printf '%s' "$SEMVER" | tr -d '[:space:]')"

# A version flows into git tags and file contents; keep it a strict semver core.
case "$SEMVER" in
  [0-9]*.[0-9]*.[0-9]*) ;;
  *) die "resolved version '$SEMVER' is not X.Y.Z" ;;
esac

TAG="v$SEMVER"
CURRENT="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | sed 1q)"
log "→ current version : ${CURRENT:-<none>}"
log "→ target  version : $SEMVER  (selector: $SELECTOR)"

if [ "$SEMVER" = "$CURRENT" ]; then
  log "  note: target equals the current version; nothing new to bump but continuing so the changelog regenerates"
fi

# Only the top-level `version = "…"` in each manifest starts at column 0; dependency pins are
# indented or written `package = { version = … }`, so this anchored substitution is unambiguous.
stamp_version() {
  local file="$1"
  [ -f "$file" ] || die "expected $file at repo root"
  grep -q '^version = "' "$file" || die "no top-level 'version = \"…\"' found in $file"
  # Portable in-place edit (GNU and BSD sed) via a temp file.
  local tmp
  tmp="$(mktemp)"
  sed "s/^version = \"[^\"]*\"/version = \"$SEMVER\"/" "$file" >"$tmp"
  mv "$tmp" "$file"
  log "  stamped $file"
}

stamp_version Cargo.toml
stamp_version pixi.toml

# The CLI crate pins its sibling libraries by an exact `path = …, version = "X.Y.Z"` requirement.
# Those must move with the workspace version or cargo refuses to resolve the bumped members
# ("candidate versions found which didn't match"). Only rewrite the pin on the internal-dep lines.
MEMBER_MANIFEST="crates/pixi-sandbox/Cargo.toml"
if grep -qE 'pixi-sandbox-(core|git).*path *=' "$MEMBER_MANIFEST" 2>/dev/null; then
  tmp="$(mktemp)"
  sed -E "/pixi-sandbox-(core|git).*path *=/ s/version = \"[^\"]*\"/version = \"$SEMVER\"/" \
    "$MEMBER_MANIFEST" >"$tmp"
  mv "$tmp" "$MEMBER_MANIFEST"
  log "  stamped $MEMBER_MANIFEST (internal dependency pins)"
fi

# Keep Cargo.lock's workspace-member versions in step so a later `cargo build --locked` (or CI)
# does not trip over a stale lock. `--workspace` touches only the members, not external pins;
# prefer offline (vendor/cache is enough because no new dependency is introduced).
if command -v cargo >/dev/null 2>&1 && [ -f Cargo.lock ]; then
  log "→ syncing Cargo.lock (cargo update --workspace)"
  cargo update --workspace --offline >/dev/null 2>&1 ||
    cargo update --workspace >/dev/null 2>&1 ||
    log "  warning: could not refresh Cargo.lock automatically — check it before releasing"
fi

# convco owns the changelog format (.versionrc). The changelog is generated *before* the tag
# exists, so name the pending section after the version being cut (`--unreleased X.Y.Z` titles it
# with that version) instead of leaving today's commits under a generic "Unreleased".
log "→ regenerating CHANGELOG.md (convco changelog --unreleased $SEMVER)"
convco changelog --unreleased "$SEMVER" >CHANGELOG.md

log "→ prepared release $TAG"
printf '%s\n' "$TAG"
