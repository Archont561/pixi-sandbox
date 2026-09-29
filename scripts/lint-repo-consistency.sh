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
# 3. task-2 follow-up — every hardcoded `vX.Y.Z` in the user-facing docs is the version the
#    manifests declare. A release commit bumps `Cargo.toml` but cannot rewrite the ~100 doc
#    references, so the two drift silently and a reader following the docs installs the previous
#    release. This is the check that makes the drift a CI failure instead of a support issue.
# 4. every third-party `uses:` is a full commit SHA with a trailing release label, so a moved tag
#    cannot change what CI runs. actionlint checks workflow syntax, not what a `uses:` resolves to.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

README=README.md
PIXI=pixi.toml
PLAN=.pixi-sandbox.toml
CARGO=Cargo.toml

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

# ---------------------------------------------------------------- 3. version references

# The declared version, from the same anchored substitution prepare-release.sh writes.
current="$(
  sed -n 's/^version = "\([^"]*\)".*/\1/p' "$CARGO" | sed 1q
)"
if [ -z "$current" ]; then
  fail "$CARGO has no parsable top-level version — cannot check the doc references (task-2)"
else
  current="v$current"
  # Only OUR tags. A third-party pin on the same line (`prefix-dev/setup-pixi@v0.10.2`,
  # `actions/checkout@v7.0.1`) is someone else's release and must never be rewritten or failed:
  # the repository is deliberately not the authority on `setup-pixi`'s version. So a line counts
  # only when it names this project — an `Archont561/pixi-sandbox...@vX.Y.Z` ref, a
  # `releases/download/vX.Y.Z/` URL, or a bare `version: vX.Y.Z` input. Everything else is
  # skipped, which is also why this check cannot be a blind `grep v[0-9]`.
  #
  # A reference that must stay on an older release (a deliberate upgrade walkthrough, a
  # regression test) opts out with `stale-ref-allowed` on the line.
  stale_versions="$(
    find . -type f \( -name '*.md' -o -name '*.mdx' \) \
      -not -path './.git/*' \
      -not -path './.knowledge/*' \
      -not -path './node_modules/*' \
      -not -path './docs/node_modules/*' \
      -not -name 'CHANGELOG.md' \
      -not -path './backlog/*' \
      -print0 |
      xargs -0 awk -v cur="$current" -v root="Archont561/pixi-sandbox" '
        FNR == 1 { prev = "" }
        {
          if ($0 !~ /stale-ref-allowed/ && prev !~ /stale-ref-allowed/ &&
              (index($0, root) || $0 ~ /releases\/download\/v[0-9]/ || $0 ~ /version: v[0-9]/ ||
               $0 ~ /PIXI_SANDBOX_VERSION=v[0-9]/ || $0 ~ /`uses: @v[0-9]/)) {
            line = $0
            # Every vX.Y.Z on a project line must be the declared one.
            while (match(line, /v[0-9]+\.[0-9]+\.[0-9]+/)) {
              tok = substr(line, RSTART, RLENGTH)
              if (tok != cur) printf "%s:%d: pins %s, manifests declare %s\n", FILENAME, FNR, tok, cur
              line = substr(line, RSTART + RLENGTH)
            }
          }
          prev = $0
        }
      ' || true
  )"
  if [ -n "$stale_versions" ]; then
    fail "documentation pins a version the manifests do not declare (task-2):"
    printf '  %s\n' "$stale_versions" >&2
    echo "  repin to $current, or mark the line stale-ref-allowed if it must name an older release" >&2
  fi
fi

# ---------------------------------------------------------------- 4. action pins

# Every third-party `uses:` is a full commit SHA plus a trailing release label, so a compromised
# or force-moved tag cannot change what CI runs. `ci.yml`, `auto-release.yml` and `docs.yml` have
# always done this; `release.yml` and `publish-sandbox.yml` carried plain `@vX.Y.Z` tags, which
# made the rule a convention nobody enforced. actionlint validates the workflows' syntax and
# inputs but says nothing about what a `uses:` resolves to, so this is the only thing standing
# between a mutable tag and the release pipeline.
#
# Same-repository actions (`./setup`, `../action.yml`) are not third-party and carry no ref to
# pin. A deliberate exception — a pin that must name an older action release — opts out with
# `stale-ref-allowed` on the line.
unpinned="$(
  find .github/workflows -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 |
    xargs -0 awk '
      FNR == 1 { prev = "" }
      {
        if ($0 ~ /^[[:space:]]*#/ || $0 !~ /uses:/) { prev = $0; next }
        if (index($0, "stale-ref-allowed") || index(prev, "stale-ref-allowed")) { prev = $0; next }
        if (match($0, /uses:[[:space:]]*[^[:space:]#]+/)) {
          spec = substr($0, RSTART, RLENGTH)
          sub(/^uses:[[:space:]]*/, "", spec)
          if (spec ~ /^\.\.?\// || spec ~ /^docker:\/\// || !match(spec, /@/)) { prev = $0; next }
          ref = substr(spec, RSTART + 1)
          why = ""
          if (ref !~ /^[0-9a-f]{40}$/)
            why = "pins " ref ", a mutable ref, not a 40-character commit SHA"
          else if (substr($0, RSTART + RLENGTH) !~ /#[[:space:]]*v[0-9]/)
            why = "pins a SHA with no trailing release label, so the next person cannot tell what it is"
          if (why != "") printf "%s:%d:%s\n", FILENAME, FNR, why
        }
        prev = $0
      }
    ' || true
)"
if [ -n "$unpinned" ]; then
  fail "a third-party action is not pinned to a full commit SHA (see .knowledge/publish-automation.md):"
  printf '  %s\n' "$unpinned" >&2
  echo "  resolve the tag with 'gh api repos/OWNER/REPO/git/ref/tags/TAG', dereference it if it is an annotated tag, and pin it as owner/repo@<sha> # vX.Y.Z" >&2
fi

# ----------------------------------------------------------------

if [ "$failures" -gt 0 ]; then
  printf 'repo consistency: %d check(s) failed\n' "$failures" >&2
  exit 1
fi
echo "repo consistency: crates/ is free of prototype references; platform and version claims agree; action pins are immutable"
