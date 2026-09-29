#!/usr/bin/env bash
# Render `setup/action.yml` and `publish/action.yml` from the single implementation in the
# repository-root `action.yml`.
#
# Usage: bash scripts/render-action-shims.sh [setup|publish] [OUTPUT]
#   no arguments   rewrite both setup/action.yml and publish/action.yml in place
#   <variant>      print that variant to stdout (or to OUTPUT when given)
#
# Why a renderer and not a two-line shim that calls the root action (task-12, issue #37):
# `setup/action.yml` and `publish/action.yml` used to contain one step, `uses: ../action.yml`.
# That cannot work from a remote ref. A relative `uses:` inside a composite action is resolved
# against the *workflow run's workspace*, not against the repository that defines the action,
# and a `../` prefix is rejected outright by the runner's reference parser:
#
#   Expected format {org}/{repo}[/path]@ref. Actual '../action.yml'
#
# so every `Archont561/pixi-sandbox/setup@<ref>` caller died during job setup. `./action.yml`
# would be worse than the error: it resolves in the caller's checkout, so it either fails with
# "Can't find 'action.yml'" or silently runs a same-named file belonging to the consumer.
# The proposed `$/` same-repo syntax has not shipped. Hardcoding `Archont561/pixi-sandbox@<sha>`
# inside the shim defeats the caller's own SHA pin (the consumer pins one ref; the shim would
# run a different one) and cannot be tested before it is tagged.
#
# That leaves one honest option: each published action path is a complete composite action.
# Duplication is the mechanism, not the design — the copies are generated here and
# `scripts/lint-repo-consistency.sh` (check 7) re-renders them and fails on any drift, exactly
# as `templates/install.sh` + `scripts/render-install.sh` make a stale release asset
# unconstructible (task-2). Edit `action.yml`, then run this script.
#
# The transform is deliberately tiny, so a reviewer can hold it in their head:
#   1. the root's own header (name/description/author/branding + its explanatory comment) is
#      replaced by the variant's header, because the marketplace metadata is the one thing
#      that legitimately differs between the three entry points;
#   2. everything from `inputs:` to EOF is copied byte for byte;
#   3. for `publish`, the `subpath` input's default flips from `setup` to `publish`, which is
#      what selects the behaviour at run time.
set -euo pipefail

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || dirname "$SCRIPT_DIR")"
ROOT_ACTION="$REPO_ROOT/action.yml"

[ -f "$ROOT_ACTION" ] || die "no root action.yml at $ROOT_ACTION"

header() {
  # $1 = variant. The banner names the source and the renderer so that an editor who opens the
  # generated file first is told where to go, and the lint that enforces it.
  cat <<EOF
# GENERATED FILE — do not edit. Source: ../action.yml (rendered by scripts/render-action-shims.sh).
# Regenerate with: bash scripts/render-action-shims.sh
#
# This is a full copy of the root action, not a shim that calls it: a relative \`uses:\` inside a
# remote composite action resolves against the caller's workspace, so \`uses: ../action.yml\` made
# every \`Archont561/pixi-sandbox/$1@<ref>\` caller fail at job setup (issue #37). Check 7 of
# scripts/lint-repo-consistency.sh re-renders this file and fails if it has drifted from ../action.yml.
EOF
  if [ "$1" = setup ]; then
    cat <<'EOF'
name: Setup pixi-sandbox
description: Download, checksum-verify, and expose a standalone pixi-sandbox release binary. Use as Archont561/pixi-sandbox/setup@vX (or root Archont561/pixi-sandbox@vX).
author: pixi-sandbox
branding:
  icon: package
  color: orange
EOF
  else
    cat <<'EOF'
name: Publish Pixi sandbox bundle
description: Install requested Pixi environments, pack with a verified release binary, verify, and publish one orphan branch. Use as Archont561/pixi-sandbox/publish@vX.
author: pixi-sandbox
branding:
  icon: upload-cloud
  color: blue
EOF
  fi
  printf '\n'
}

render() {
  variant="$1"
  header "$variant"
  awk -v variant="$variant" '
    # Copy `inputs:` to EOF verbatim, flipping only the `subpath` default for the publish copy.
    body {
      if (variant == "publish" && !flipped && $0 == "    default: setup") {
        print "    default: publish"
        flipped = 1
        next
      }
      print
      next
    }
    /^inputs:$/ { body = 1; print; next }
    END {
      if (!body) { print "::error::action.yml has no top-level inputs: block" > "/dev/stderr"; exit 3 }
      if (variant == "publish" && !flipped) {
        print "::error::action.yml has no `default: setup` for the subpath input" > "/dev/stderr"
        exit 3
      }
    }
  ' "$ROOT_ACTION"
}

check() {
  # A generated action that is not self-contained, or that lost the mode switch, is the bug this
  # renderer exists to prevent: refuse to emit it.
  file="$1"
  variant="$2"
  grep -q '^  using: composite$' "$file" || die "rendered $variant action is not a composite action"
  grep -q "^    default: $variant\$" "$file" || die "rendered $variant action does not default to subpath=$variant"
  if grep -nE '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]*\.' "$file"; then
    die "rendered $variant action contains a relative uses:, which cannot resolve from a remote ref"
  fi
}

emit() {
  variant="$1"
  output="$2"
  case "$variant" in
    setup | publish) ;;
    *) die "unknown variant '$variant' (expected setup or publish)" ;;
  esac
  tmp="$(mktemp)"
  # shellcheck disable=SC2064 # expand $tmp now: the trap must survive the variable being reused
  trap "rm -f '$tmp'" EXIT
  render "$variant" >"$tmp"
  check "$tmp" "$variant"
  if [ -z "$output" ]; then
    cat "$tmp"
  else
    mkdir -p "$(dirname "$output")"
    cat "$tmp" >"$output"
    log "→ $output (subpath default: $variant)"
  fi
  rm -f "$tmp"
  trap - EXIT
}

VARIANT="${1:-}"
OUTPUT="${2:-}"

if [ -z "$VARIANT" ]; then
  emit setup "$REPO_ROOT/setup/action.yml"
  emit publish "$REPO_ROOT/publish/action.yml"
else
  emit "$VARIANT" "$OUTPUT"
fi
