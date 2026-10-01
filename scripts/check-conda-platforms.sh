#!/usr/bin/env bash
# Fail the release unless every supported platform contributed exactly one conda package.
#
# Usage: bash scripts/check-conda-platforms.sh [dir]     (dir defaults to dist/conda)
#
# task-23 AC#4. The release build matrix runs with `fail-fast: false` so one broken platform
# does not cancel the other four and hide which one broke. That makes the *release* job the
# only place that can still turn a missing platform into a red build: without this check, a
# runner that produced no package (an unsolvable manifest, a silently skipped step) would let
# the job upload four packages and attach a GitHub Release that looks complete.
#
# Layout this reads is `dir/conda-<platform>/*.conda`, which is what `actions/download-artifact`
# with `pattern: conda-*` and no `merge-multiple` produces. The five packages carry the *same*
# filename, so the per-platform directory is the only thing that tells them apart — a flat merge
# would overwrite four of them and this check would pass on one platform's bytes.
set -euo pipefail

PLATFORMS="linux-64 linux-aarch64 osx-64 osx-arm64 win-64"
DIR="${1:-dist/conda}"

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

[ -d "$DIR" ] || die "$DIR does not exist — the conda package artifacts were not downloaded"

missing=()
checked=0
for platform in $PLATFORMS; do
  packages=("$DIR/conda-$platform"/*.conda)
  if [ ! -e "${packages[0]}" ]; then
    missing+=("$platform")
    continue
  fi
  if [ "${#packages[@]}" -ne 1 ]; then
    die "conda-$platform holds ${#packages[@]} packages, expected exactly 1"
  fi
  checked=$((checked + 1))
  log "ok: $platform -> $(basename "${packages[0]}")"
done

if [ "${#missing[@]}" -ne 0 ]; then
  printf '::error::no conda package for: %s\n' "${missing[*]}" >&2
  log "every supported platform must build and smoke-test its own package (task-23);"
  log "read the failing platform's build job above for why it produced nothing"
  exit 1
fi

log "all $checked platform packages present"
