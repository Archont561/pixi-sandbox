#!/usr/bin/env bash
# One-liner offline reconstruction from orphan branch, with pixi launcher registration.
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
#   PIXI_SANDBOX_USER_TOOLS  user-tool registration policy after a verified restore:
#                            `register` (default) puts pixi and pixi-sandbox launchers in
#                            ~/.local/bin and adds it to the shell profile PATH; `skip`
#                            leaves HOME and every profile untouched (CI, shared accounts).
#
# The branch name is never guessed: it is read from the same reviewed publish plan the
# publisher uses, so a config change (new bundle, renamed prefix) reaches this script for
# free. After restore, pixi remains the sole entrypoint: open a new shell for the registered
# launcher (or prepend the managed user bin in this shell) and run repository commands through
# `pixi run ...`. This script never sources project activation hooks.

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

# The config is read with plain regexes, not a TOML parser: the restore path is bootstrap-only,
# so the binary that could print the real plan is the very thing this script is about to unpack,
# and an airlocked host is not assumed to have Python or pixi — only sh/sed/grep. That is enough
# here because `.pixi-sandbox.toml` is a flat, reviewed file (see
# crates/pixi-sandbox-core/src/sandbox_config.rs) whose names are restricted to [a-z0-9-].
# Every pattern anchors on the quotes around a value, so trailing `#` comments need no handling.
QUOTE='["'"'"']'

config_value() {
  # config_value <key> — first root-level scalar declared for that key.
  sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*$QUOTE\([^\"']*\).*/\1/p" "$CONFIG" | sed 1q
}

config_targets() {
  # Every declared target as `<bundle>\t<platform>\t<branch>`. Arrays may span lines, so the file
  # is flattened first and then cut into one record per [[bundle]] table; from there two regexes
  # read the bundle name and its platforms array.
  tr '\n' ' ' <"$CONFIG" |
    sed 's/\[\[[[:space:]]*bundle[[:space:]]*\]\]/\
/g' |
    while IFS= read -r record || [ -n "$record" ]; do # `||` keeps the unterminated last record
      name="$(printf '%s' "$record" | sed -n "s/.*name[[:space:]]*=[[:space:]]*$QUOTE\([^\"']*\).*/\1/p")"
      platforms="$(printf '%s' "$record" |
        sed -n "s/.*platforms[[:space:]]*=[[:space:]]*\[\([^]]*\)\].*/\1/p" | tr -d "\"' " | tr ',' ' ')"
      [ -n "$name" ] && [ -n "$platforms" ] || continue
      for platform in $platforms; do
        printf '%s\t%s\t%s/%s-%s\n' "$name" "$platform" "$PREFIX" "$name" "$platform"
      done
    done
}

declared_branches() {
  printf '%s\n' "$TARGETS" | cut -f3 | paste -sd' ' -
}

derive_branch() {
  if [ ! -f "$CONFIG" ]; then
    echo "::error::no $CONFIG to derive the branch from — pass it explicitly: bash scripts/restore.sh <branch>" >&2
    exit 2
  fi

  local schema matches count
  schema="$(sed -n 's/^[[:space:]]*schema[[:space:]]*=[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$CONFIG" | sed 1q)"
  if [ "$schema" != "1" ]; then
    echo "::error::$CONFIG declares schema ${schema:-<missing>}; this script only knows schema 1 branch naming — pass the branch explicitly" >&2
    exit 2
  fi

  PREFIX="$(config_value branch_prefix)"
  PREFIX="${PREFIX:-sandbox}"
  TARGETS="$(config_targets)"
  if [ -z "$TARGETS" ]; then
    echo "::error::$CONFIG declares no [[bundle]] targets" >&2
    exit 2
  fi

  matches="$(printf '%s\n' "$TARGETS" | awk -F'\t' -v platform="$PLATFORM" '$2 == platform')"
  if [ -n "${PIXI_SANDBOX_BUNDLE:-}" ]; then
    matches="$(printf '%s\n' "$matches" | awk -F'\t' -v bundle="$PIXI_SANDBOX_BUNDLE" '$1 == bundle')"
    if [ -z "$matches" ]; then
      echo "::error::$CONFIG has no bundle '$PIXI_SANDBOX_BUNDLE' for platform $PLATFORM" >&2
      printf '  declared: %s\n' "$(declared_branches)" >&2
      exit 2
    fi
  fi

  count="$(printf '%s\n' "$matches" | awk 'NF { total++ } END { print total + 0 }')"
  case "$count" in
    0)
      echo "::error::$CONFIG publishes nothing for this host ($PLATFORM)" >&2
      printf '  declared branches: %s\n' "$(declared_branches)" >&2
      echo "  add $PLATFORM to a [[bundle]], or pass a branch explicitly" >&2
      exit 2
      ;;
    1) ;;
    *)
      echo "::error::$CONFIG declares several bundles for $PLATFORM: $(printf '%s\n' "$matches" | cut -f1 | paste -sd' ' -)" >&2
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

# A branch name flows straight into git plumbing below. Keep this in lockstep with
# `pixi_sandbox_core::sandbox_config::is_safe_git_ref`: the property test in
# `crates/pixi-sandbox/tests/restore_script.rs` generates printable names and proves that the
# airlock's Bash check and Rust's publish-plan check agree. This remains shell-only because
# `--print-branch` must not need git or the binary it is about to restore.
is_safe_ref() {
  case "$1" in
    '' | -* | /* | */ | *'//'*) return 1 ;;
    *'..'* | *'@{'* | *[[:space:]]* | *'~'* | *'^'* | *':'* | *'?'* | *'*'* | *'['* | *'\'*) return 1 ;;
    .* | */.* | *. | *./* | *.lock | *.lock/*) return 1 ;;
  esac
  return 0
}

if ! is_safe_ref "$BRANCH"; then
  echo "::error::refusing unsafe branch name: $BRANCH" >&2
  exit 2
fi

if [ "$PRINT_ONLY" = 1 ]; then
  printf '%s\n' "$BRANCH"
  exit 0
fi

if [ "$DERIVED" = 1 ]; then
  echo "→ branch $BRANCH (derived from $(basename "$CONFIG") for $PLATFORM)"
else
  echo "→ branch $BRANCH"
fi

# Invariant 3: never use /tmp as a work dir — the branch checkout carries every packed blob
# and a small tmpfs fails mid-restore. The worktree lives under the restore's own work
# directory inside the output tree — the same `.pixi/.restore-work` the binary stages into,
# which `.gitignore` conventions cover — and children inherit that work dir as TMPDIR. The
# directory is created only once the branch is resolved, so a failed lookup leaves the output
# path untouched.
WORK="$OUTPUT/.pixi/.restore-work"
WORKTREE="$WORK/sb-$$"

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

mkdir -p "$WORK/tmp"
TMPDIR="$WORK/tmp"
export TMPDIR
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

# Informational, not enforced: `restore` verifies the branch again before it writes anything
# (invariant 1 is enforced inside the binary), so this pre-check exists to show the operator
# the verification table early. A failure here must not hide the restore's own verdict — on a
# genuinely corrupt branch the restore refuses to write, and that refusal is the enforcement.
echo "→ doctor $BIN (informational pre-check; restore re-verifies before writing)"
"$BIN" doctor --branch-location "$WORKTREE" --verify || true

echo "→ restore to $OUTPUT"
# The policy is explicit here rather than inherited from the CLI default (task-33): this
# script is the repo's own airlock path, so what it does to the user's home is written down.
# It travels as an environment variable, never a flag, because the binary driven here comes
# from the packed branch and may predate --user-tools — an unknown variable is ignored
# where an unknown flag is a hard error. A trailing --user-tools skip still wins; later
# arguments override the environment.
PIXI_SANDBOX_USER_TOOLS="${PIXI_SANDBOX_USER_TOOLS:-register}"
export PIXI_SANDBOX_USER_TOOLS
# --output-path is the modern flag; --path-to-main-repo-code is its old spelling, kept as an
# alias (cli.rs) so a bootstrap packed before the rename still restores. Only clap's usage
# error on that exact flag (exit 2, "unexpected argument") earns one retry with the alias. Any
# other failure is a real restore failure: reported once and never rerun — the old fallback
# retried every failure against the same binary, replaying a restore that had already failed.
FIRST_LOG="$WORK/restore-first-attempt.log"
set +e
"$BIN" restore --branch-location "$WORKTREE" --output-path "$OUTPUT" --force >"$FIRST_LOG" 2>&1
STATUS=$?
set -e
if [ "$STATUS" -eq 0 ]; then
  cat "$FIRST_LOG"
  rm -f "$FIRST_LOG"
elif [ "$STATUS" -eq 2 ] &&
  grep -q 'unexpected argument' "$FIRST_LOG" &&
  grep -q -- '--output-path' "$FIRST_LOG"; then
  echo "→ this bootstrap predates --output-path; retrying with --path-to-main-repo-code"
  rm -f "$FIRST_LOG"
  "$BIN" restore --branch-location "$WORKTREE" --path-to-main-repo-code "$OUTPUT" --force
else
  cat "$FIRST_LOG"
  echo "::error::restore failed (exit $STATUS); $WORKTREE is kept for inspection" >&2
  exit "$STATUS"
fi

# The branch was verified before anything was written and every blob was verified as it was
# written; this checks the tree that came out the other end against the manifest's per-file
# digests (D13). It fails closed on a mismatch — a wrong prefix must not be reported as a
# restore — and merely reports a notice for a schema-1 branch, which predates the oracle.
# Either way this runs before the worktree is cleaned up, and on failure the worktree stays
# as evidence.
if ! "$BIN" doctor --branch-location "$WORKTREE" --verify-restored "$OUTPUT"; then
  echo "::error::the restored project does not match the manifest; $WORKTREE is kept for inspection" >&2
  exit 1
fi

echo "→ cleanup worktree"
mkdir -p "$WORK/tmp"
git worktree remove "$WORKTREE" --force || rm -rf "$WORKTREE"
# The work dir is this restore's scratch, removed the way the binary's clean_work_dir removes
# its own: the named entries, then the directory itself only once it is empty — so a work dir
# explicitly shared with something else survives the restore that borrowed it.
rmdir "$WORK/tmp" 2> /dev/null || true
rmdir "$WORK" 2> /dev/null || true

# A running parent shell cannot inherit the profile edit restore just wrote. Report what
# happened and name the pixi entrypoint to use next; do not source an activation hook. Newer
# restores do not generate `.pixi/sandbox-env.sh`, and older ones are intentionally ignored here
# so the repo has one supported command path: pixi -> pixi tasks -> xtask/cargo/bun.
USER_BIN="${PIXI_SANDBOX_USER_BIN:-${HOME:-}/.local/bin}"
RESTORED_PIXI="$OUTPUT/.pixi/tools/$PLATFORM/pixi"
if [ "$PIXI_SANDBOX_USER_TOOLS" = skip ]; then
  echo "user tools: not registered (PIXI_SANDBOX_USER_TOOLS=skip); current shell unchanged"
  echo "  use $RESTORED_PIXI run --frozen <task> explicitly, or restore with registration enabled"
elif grep -q "managed by pixi-sandbox" "$USER_BIN/pixi" 2> /dev/null; then
  echo "user tools: registered pixi and pixi-sandbox in $USER_BIN for new shells"
  echo "  (set PIXI_SANDBOX_USER_TOOLS=skip to disable that on the next restore)"
  echo "  current shell: export PATH=\"$USER_BIN:\$PATH\""
  echo "  then use pixi as the only entrypoint: pixi run --frozen <task>"
else
  # The restored copy, not "$BIN": that one lives in the branch worktree, which was removed
  # above, so asking it for a version printed "unknown version" in the past.
  RESTORED_SELF="$OUTPUT/.pixi/tools/$PLATFORM/pixi-sandbox"
  BUNDLED_VERSION="$("$RESTORED_SELF" --version 2> /dev/null || echo 'bundled pixi-sandbox')"
  echo "user tools: NOT registered — the bundled $BUNDLED_VERSION predates --user-tools (0.3.7),"
  echo "  so PIXI_SANDBOX_USER_TOOLS was ignored. This script no longer sources or supports"
  echo "  .pixi/sandbox-env.sh; use $RESTORED_PIXI run --frozen <task> explicitly, or restore"
  echo "  a transport packed by 0.3.7 or newer to register pixi launchers."
fi
