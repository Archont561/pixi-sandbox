#!/usr/bin/env bash
# One-liner offline reconstruction from orphan branch, with PATH aliases.
#
# Usage: bash scripts/restore.sh [branch] [output-path]
#   branch        defaults to the branch declared for this host in .pixi-sandbox.toml,
#                 i.e. <branch_prefix>/<bundle>-<platform>. Pass `auto` to force
#                 derivation even when PIXI_SANDBOX_BRANCH is set.
#   output-path   defaults to .
#
# Environment:
#   PIXI_SANDBOX_BRANCH   same as the [branch] argument (argument wins)
#   PIXI_SANDBOX_BUNDLE   choose a bundle when the config declares several for this platform
#   PIXI_SANDBOX_CONFIG   config path (default: <repo root>/.pixi-sandbox.toml)
#
# The branch name is never guessed: it is read from the same reviewed publish plan the
# publisher uses, so a config change (new bundle, renamed prefix) reaches this script for
# free. After restore, sources .pixi/sandbox-env.sh and adds the default env to PATH.

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || dirname "$SCRIPT_DIR")"
CONFIG="${PIXI_SANDBOX_CONFIG:-$REPO_ROOT/.pixi-sandbox.toml}"

# Same mapping the generated launchers use (crates/pixi-sandbox/src/commands/init.rs), so a
# host resolves to one Pixi platform name everywhere.
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) PLATFORM=linux-64 ;;
  Linux-aarch64 | Linux-arm64) PLATFORM=linux-aarch64 ;;
  Darwin-arm64) PLATFORM=osx-arm64 ;;
  Darwin-x86_64) PLATFORM=osx-64 ;;
  MINGW* | MSYS* | CYGWIN*) PLATFORM=win-64 ;;
  *)
    echo "::error::unsupported airlock platform: $(uname -s)-$(uname -m)" >&2
    exit 2
    ;;
esac

# Minimal reader for the subset of TOML `.pixi-sandbox.toml` is allowed to use (flat root keys
# plus [[bundle]] tables — see crates/pixi-sandbox-core/src/sandbox_config.rs). It exists because
# the restore path must stay bootstrap-only: the binary that could print the real plan is the
# very thing this script is about to unpack, and no Python/pixi is assumed on an airlocked host.
read_config() {
  awk '
    function trim(s) { sub(/^[ \t\r]+/, "", s); sub(/[ \t\r]+$/, "", s); return s }
    function unquote(s,   q) {
      s = trim(s)
      q = substr(s, 1, 1)
      if ((q == "\"" || q == "'\''") && substr(s, length(s), 1) == q) {
        s = substr(s, 2, length(s) - 2)
      }
      return s
    }
    # Drop `#` comments, but only outside quoted strings.
    function strip_comment(line,   out, i, c, quote) {
      out = ""; quote = ""
      for (i = 1; i <= length(line); i++) {
        c = substr(line, i, 1)
        if (quote != "") { out = out c; if (c == quote) quote = ""; continue }
        if (c == "\"" || c == "'\''") { quote = c; out = out c; continue }
        if (c == "#") break
        out = out c
      }
      return out
    }
    function flush(   i, n, parts) {
      if (table != "bundle" || name == "") return
      n = split(platforms, parts, ",")
      for (i = 1; i <= n; i++) {
        if (parts[i] != "") print "target\t" name "\t" parts[i] "\t" prefix "/" name "-" parts[i]
      }
    }
    function parse_array(value,   i, n, parts, out) {
      sub(/^[ \t]*\[/, "", value); sub(/\][ \t]*$/, "", value)
      n = split(value, parts, ",")
      out = ""
      for (i = 1; i <= n; i++) {
        parts[i] = unquote(parts[i])
        if (parts[i] != "") out = (out == "" ? parts[i] : out "," parts[i])
      }
      return out
    }
    function handle(line,   key, value, eq) {
      if (line ~ /^\[\[[ \t]*bundle[ \t]*\]\]$/) { flush(); table = "bundle"; name = ""; platforms = ""; return }
      if (line ~ /^\[/) { flush(); table = "other"; name = ""; platforms = ""; return }
      eq = index(line, "=")
      if (eq == 0) return
      key = trim(substr(line, 1, eq - 1))
      value = trim(substr(line, eq + 1))
      if (table == "") {
        if (key == "schema") schema = unquote(value)
        else if (key == "branch_prefix") prefix = unquote(value)
      } else if (table == "bundle") {
        if (key == "name") name = unquote(value)
        else if (key == "platforms") platforms = parse_array(value)
      }
    }
    BEGIN { prefix = "sandbox"; schema = ""; table = ""; buffer = ""; pending = 0 }
    {
      line = trim(strip_comment($0))
      if (line == "" && !pending) next
      buffer = (buffer == "" ? line : buffer " " line)
      # Arrays may span lines; hold the logical line until the bracket closes.
      if (pending) { if (index(line, "]")) pending = 0; else next }
      else if (buffer ~ /=[ \t]*\[/ && index(buffer, "]") == 0) { pending = 1; next }
      handle(buffer); buffer = ""
    }
    END { if (buffer != "") handle(buffer); flush(); print "schema\t" schema }
  ' "$1"
}

derive_branch() {
  if [ ! -f "$CONFIG" ]; then
    echo "::error::no $CONFIG to derive the branch from — pass it explicitly: bash scripts/restore.sh <branch>" >&2
    exit 2
  fi

  local parsed schema targets matches bundles
  parsed="$(read_config "$CONFIG")"
  schema="$(printf '%s\n' "$parsed" | awk -F'\t' '$1 == "schema" { print $2 }')"
  if [ "$schema" != "1" ]; then
    echo "::error::$CONFIG declares schema ${schema:-<missing>}; this script only knows schema 1 branch naming — pass the branch explicitly" >&2
    exit 2
  fi

  targets="$(printf '%s\n' "$parsed" | awk -F'\t' '$1 == "target" { print $2 "\t" $3 "\t" $4 }')"
  if [ -z "$targets" ]; then
    echo "::error::$CONFIG declares no [[bundle]] targets" >&2
    exit 2
  fi

  matches="$(printf '%s\n' "$targets" | awk -F'\t' -v platform="$PLATFORM" '$2 == platform')"
  if [ -n "${PIXI_SANDBOX_BUNDLE:-}" ]; then
    matches="$(printf '%s\n' "$matches" | awk -F'\t' -v bundle="$PIXI_SANDBOX_BUNDLE" '$1 == bundle')"
    if [ -z "$matches" ]; then
      echo "::error::$CONFIG has no bundle '$PIXI_SANDBOX_BUNDLE' for platform $PLATFORM" >&2
      printf '  declared: %s\n' "$(printf '%s\n' "$targets" | cut -f3 | paste -sd' ' -)" >&2
      exit 2
    fi
  fi

  local count
  count="$(printf '%s\n' "$matches" | awk 'NF { total++ } END { print total + 0 }')"
  case "$count" in
    0)
      echo "::error::$CONFIG publishes nothing for this host ($PLATFORM)" >&2
      printf '  declared branches: %s\n' "$(printf '%s\n' "$targets" | cut -f3 | paste -sd' ' -)" >&2
      echo "  add $PLATFORM to a [[bundle]], or pass a branch explicitly" >&2
      exit 2
      ;;
    1) ;;
    *)
      bundles="$(printf '%s\n' "$matches" | cut -f1 | paste -sd' ' -)"
      echo "::error::$CONFIG declares several bundles for $PLATFORM: $bundles" >&2
      echo "  choose one: PIXI_SANDBOX_BUNDLE=<name> bash scripts/restore.sh" >&2
      exit 2
      ;;
  esac

  printf '%s\n' "$matches" | cut -f3
}

PRINT_ONLY=0
case "${1:-}" in
  -h | --help)
    sed -n '3,17p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 0
    ;;
  # Resolve and print the branch without touching the network or the working tree.
  --print-branch)
    PRINT_ONLY=1
    shift
    ;;
esac

BRANCH="${1:-${PIXI_SANDBOX_BRANCH:-auto}}"
OUTPUT="${2:-.}"
DERIVED=0
if [ "$BRANCH" = "auto" ] || [ -z "$BRANCH" ]; then
  BRANCH="$(derive_branch)"
  DERIVED=1
fi

# A branch name flows straight into git plumbing below; keep it to git-ref-safe characters.
case "$BRANCH" in
  -* | *' '* | *'..'* | *'~'* | *'^'* | *':'* | *'?'* | *'*'* | *'['* | *'\'* | *'@{'*)
    echo "::error::refusing unsafe branch name: $BRANCH" >&2
    exit 2
    ;;
esac

if [ "$PRINT_ONLY" = 1 ]; then
  printf '%s\n' "$BRANCH"
  exit 0
fi

if [ "$DERIVED" = 1 ]; then
  echo "→ branch $BRANCH (derived from $(basename "$CONFIG") for $PLATFORM)"
else
  echo "→ branch $BRANCH"
fi

TMPDIR="${TMPDIR:-/tmp}"
WORKTREE="$TMPDIR/sb-$$"

# Prefer objects that are already local: a disconnected host is the whole point of the airlock,
# so only reach for the network when the branch is genuinely missing.
REF=""
for candidate in "$BRANCH" "origin/$BRANCH" "refs/remotes/origin/$BRANCH"; do
  if git rev-parse --verify --quiet "$candidate^{commit}" >/dev/null 2>&1; then
    REF="$candidate"
    break
  fi
done

if [ -n "$REF" ]; then
  echo "→ using local $REF (no fetch needed)"
else
  echo "→ fetching $BRANCH"
  if git fetch origin "$BRANCH:refs/remotes/origin/$BRANCH" --depth 1 2>/dev/null ||
    git fetch origin "$BRANCH"; then
    :
  else
    echo "::error::cannot fetch $BRANCH from origin" >&2
    echo "  sandbox branches on origin:" >&2
    git ls-remote --heads origin 2>/dev/null | sed 's|.*refs/heads/|    |' >&2 ||
      echo "    (origin unreachable)" >&2
    exit 1
  fi
  REF="origin/$BRANCH"
  git rev-parse --verify --quiet "$REF^{commit}" >/dev/null || REF=FETCH_HEAD
fi

echo "→ worktree $WORKTREE"
rm -rf "$WORKTREE"
git worktree add "$WORKTREE" "$REF" --force

# Schema-1 branches keep their verified bootstrap in the manifest-owned tools tree. Prefer this
# host's binary, then fall back so an unusual host can still drive a restore.
BIN=""
for candidate in \
  "$WORKTREE/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox" \
  "$WORKTREE/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox.exe" \
  "$WORKTREE/.pixi-sandbox/tools/linux-64/pixi-sandbox" \
  "$WORKTREE/.pixi-sandbox/tools/linux-aarch64/pixi-sandbox" \
  "$WORKTREE/.pixi-sandbox/tools/osx-arm64/pixi-sandbox" \
  "$WORKTREE/.pixi-sandbox/tools/osx-64/pixi-sandbox" \
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
if "$BIN" restore --branch-location "$WORKTREE" --output-path "$OUTPUT" --force 2>&1; then
  :
else
  "$BIN" restore --branch-location "$WORKTREE" --path-to-main-repo-code "$OUTPUT" --force
fi

echo "→ cleanup worktree"
git worktree remove "$WORKTREE" --force || rm -rf "$WORKTREE"

# Wire PATH aliases like setup-pixi
if [ -f "$OUTPUT/.pixi/sandbox-env.sh" ]; then
  OUTPUT_ABS="$(cd -- "$OUTPUT" && pwd)"
  echo "→ sourcing $OUTPUT/.pixi/sandbox-env.sh"
  # shellcheck disable=SC1090,SC1091
  source "$OUTPUT/.pixi/sandbox-env.sh"
  export PATH="$OUTPUT_ABS/.pixi/envs/default/bin:$PATH"
  echo "PATH now includes:"
  echo "  $OUTPUT_ABS/.pixi/tools/$PLATFORM"
  echo "  $OUTPUT_ABS/.pixi/envs/default/bin"
  echo "  pixi() function → bundled pixi"
  echo "Try: pixi --version; cargo --version; cargo check --offline"
else
  echo "restore complete but no sandbox-env.sh found"
fi
