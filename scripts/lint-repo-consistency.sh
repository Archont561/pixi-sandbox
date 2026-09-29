#!/usr/bin/env bash
# Repo-level consistency lints: the two claims this repository makes about itself that no
# cargo test is allowed to check.
#
# Usage: bash scripts/lint-repo-consistency.sh   (or `pixi run lint-repo-consistency`)
#
# Why a lint task and not a `#[test]`: D10 — tests target `tests/fixtures/`, never this
# repository, and `tests/fixtures.rs::no_test_targets_the_repository_root` enforces it. A check
# whose subject *is* this repository's README and manifests therefore belongs in the lint gate,
# next to actionlint and taplo, where CI already runs it on every push.
#
# 1. task-6 — no reference under `crates/` to the Python prototype deleted in 0.2.0. It lived in
#    `.knowledge/research/pixi_sandbox.py`; an airlocked reader has no network to discover that
#    it is gone. A line that must name the thing to forbid it opts out with `stale-ref-allowed`.
# 2. task-7 — the README Platforms badge says exactly what `pixi.toml` declares, every platform
#    `.pixi-sandbox.toml` publishes is one of them, and the Windows gap (no conda-forge `bun`,
#    D11) is documented rather than advertised by the badge.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

README=README.md
PIXI=pixi.toml
PLAN=.pixi-sandbox.toml

failures=0
fail() {
  printf '::error::%s\n' "$1" >&2
  failures=$((failures + 1))
}

# ---------------------------------------------------------------- 1. stale prototype references

# `stale-ref-allowed` on the offending line, or on the line above it (rustfmt owns where a
# trailing comment ends up), is the documented opt-out — the test that pins the CLI wording has
# to spell the forbidden words out to forbid them.
stale_hits="$(
  find crates -type f \( -name '*.rs' -o -name '*.md' \) -print0 |
    xargs -0 awk '
      FNR == 1 { prev = "" }
      {
        if (tolower($0) ~ /python|prototype|knowledge\/research/ &&
            $0 !~ /stale-ref-allowed/ && prev !~ /stale-ref-allowed/) {
          printf "%s:%d:%s\n", FILENAME, FNR, $0
        }
        prev = $0
      }
    ' || true
)"
if [ -n "$stale_hits" ]; then
  fail "crates/ still references the Python prototype deleted in 0.2.0 (task-6):"
  printf '  %s\n' "$stale_hits" >&2
  echo "  point the text at .knowledge/design.md instead, or mark the line stale-ref-allowed" >&2
fi

# ---------------------------------------------------------------- 2. platform claims

# `platforms = ["a", "b"]` from the first (workspace) declaration.
declared="$(
  sed -n 's/^[[:space:]]*platforms[[:space:]]*=[[:space:]]*\[\(.*\)\].*/\1/p' "$PIXI" |
    sed 1q | tr -d '" ' | tr ',' ' '
)"
# The badge encodes them as `Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg`: `%20%7C%20`
# separates entries and shields.io doubles a literal hyphen.
badge="$(
  sed -n 's|.*img\.shields\.io/badge/Platforms-\(.*\)-[A-Za-z]*\.svg.*|\1|p' "$README" |
    sed 1q | sed 's/%20%7C%20/ /g; s/%20/ /g; s/%7C/ /g' | sed 's/--/-/g'
)"
published="$(
  tr '\n' ' ' <"$PLAN" |
    sed -n 's/.*platforms[[:space:]]*=[[:space:]]*\[\([^]]*\)\].*/\1/p' | tr -d '" ' | tr ',' ' '
)"

sorted() { printf '%s\n' $1 | sort | paste -sd' ' -; }

if [ -z "$badge" ]; then
  fail "$README has no parsable Platforms badge (task-7)"
elif [ "$(sorted "$badge")" != "$(sorted "$declared")" ]; then
  fail "the Platforms badge advertises '$(sorted "$badge")' but $PIXI declares '$(sorted "$declared")' (task-7)"
fi

for platform in $published; do
  case " $(sorted "$declared") " in
    *" $platform "*) ;;
    *) fail "$PLAN publishes $platform, which $PIXI does not declare (task-7)" ;;
  esac
done

# Windows is not dropped in silence: the badge must not claim it, and the README must say why
# it is missing (the conda-forge `bun` gap) and that D11 has not proven it.
case " $(sorted "$badge") " in
  *" win-64 "*) fail "the Platforms badge claims win-64, which is neither declared nor proven (task-7)" ;;
esac
# One line has to carry the whole explanation, or the reader has to assemble it: the platform,
# the blocker (`bun` as a word — `bundle` does not count) and the decision that gates it.
grep -qE "win-64.*[^a-z]bun([^a-z]|$).*[^A-Za-z]D11([^0-9]|$)" "$README" ||
  fail "$README does not document the Windows gap on one line: name win-64, the conda-forge bun blocker and D11 (task-7)"

# ----------------------------------------------------------------

if [ "$failures" -gt 0 ]; then
  printf 'repo consistency: %d check(s) failed\n' "$failures" >&2
  exit 1
fi
echo "repo consistency: crates/ is free of prototype references; platform claims agree"
