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
# 3. task-2 follow-up — version references cannot drift. Two regimes, one predicate
#    (`scripts/release-refs.sh`, shared with the fix in `prepare-release.sh`):
#    the README family carries literals (GitHub renders it raw, nothing can derive there), so
#    every hardcoded `vX.Y.Z` must be the version the manifests declare; the docs site derives
#    its version at build time (scripts/version.sh → SANDBOX_VERSION → docs/astro.config.mjs
#    substitutes `__VERSION__`), so there any literal tag of ours is the defect — correct today,
#    silently stale after the next cut. And 3b: the conda package manifest is the one file whose
#    format demands a restated literal version, so it must equal Cargo.toml's.
# 4. every third-party `uses:` is a full commit SHA with a trailing release label, so a moved tag
#    cannot change what CI runs. actionlint checks workflow syntax, not what a `uses:` resolves to.
# 5. the `install.sh` release asset is rendered from `templates/install.sh` at the tag, so a published
#    one-liner cannot default to a version other than its own (task-2).
# 6. no workflow pins a literal `vX.Y.Z` release tag, so no proof silently keeps running against
#    the previous release after a cut.
# 7. task-12 — every published action path (`action.yml`, `setup/action.yml`, `publish/action.yml`)
#    is a self-contained composite action, and the two generated copies still match the root they
#    are rendered from. A relative `uses:` is what broke `owner/repo/setup@ref` in v0.3.1.

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

# Delegated to scripts/release-refs.sh, which is also what prepare-release.sh calls to FIX the
# drift. That sharing is the point: while the predicate lived only here, the release bumped the
# manifests and left ~90 documentation references behind, and this check could report the problem
# but nothing could fix it — every release was a guaranteed red build until someone hand-edited the
# docs. One definition, used by the reporter and the fixer, so they cannot disagree about which
# references are ours (and about the third-party pins that must never be touched).
stale_versions="$(bash scripts/release-refs.sh scan || true)"
if [ -n "$stale_versions" ]; then
  current="v$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$CARGO" | sed 1q)"
  fail "documentation pins a version the manifests do not declare (task-2):"
  printf '  %s\n' "$stale_versions" >&2
  echo "  README-family drift: run 'bash scripts/prepare-release.sh $current' (or mark the line stale-ref-allowed)" >&2
  echo "  docs/ literals: replace the tag with v__VERSION__ — the site derives the version at build time" >&2
fi

# 3b. every manifest that must carry a literal version agrees with Cargo.toml. The workspace
# members inherit (`version.workspace = true`) and the docs site derives (scripts/version.sh →
# SANDBOX_VERSION → docs/astro.config.mjs), so exactly one file is left that restates the
# version because its format demands a literal: the conda package manifest, a standalone
# pixi-build workspace that cannot inherit. It sat at 0.2.0 for two releases while the
# workspace said 0.3.2 — a `pixi publish` would have shipped a wrongly-versioned .conda with
# no red build anywhere. Same pattern as the install-template check below: where a literal is
# unavoidable, prepare-release.sh stamps it and this lint proves the stamp landed.
CONDA_MANIFEST=crates/pixi-sandbox/pixi.toml
workspace_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$CARGO" | sed 1q)"
conda_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' "$CONDA_MANIFEST" | sed 1q)"
if [ -z "$conda_version" ]; then
  fail "$CONDA_MANIFEST has no parsable [package] version — 'pixi publish' would refuse or guess"
elif [ "$conda_version" != "$workspace_version" ]; then
  fail "$CONDA_MANIFEST declares $conda_version but $CARGO declares $workspace_version — the published .conda would carry the wrong version"
  echo "  prepare-release.sh stamps this file; for a manual fix set version = \"$workspace_version\" in $CONDA_MANIFEST" >&2
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
          # length() + a character-class test, not /^[0-9a-f]{40}$/: mawk (the default awk on
          # Debian-family hosts, including a restored airlock) is built without interval
          # expressions, so the braced form never matches and every correctly pinned SHA was
          # reported as mutable. The check has to run where the work happens, not only on
          # ubuntu-latest.
          if (length(ref) != 40 || ref ~ /[^0-9a-f]/)
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

# ---------------------------------------------------------------- 5. install one-liner

# The `install.sh` release asset is RENDERED from templates/install.sh at the tag, not copied from
# a committed script. A committed copy carries a VERSION default that only the release knows the
# right value for, so it lags: the v0.3.0 asset shipped a v0.2.0 default and every user who followed
# the documented one-liner silently got the previous binaries (task-2). Both halves are load-bearing,
# so both are checked here — the render is reproducible and the template still carries a placeholder.
if [ ! -f templates/install.sh ]; then
  fail "templates/install.sh is missing — release.yml renders the install.sh asset from it"
else
  # The template must still carry a placeholder, and rendering the DECLARED version must produce a
  # script whose only difference is that one substitution. Anything else means the template and
  # scripts/render-install.sh disagree — the drift this check exists to make impossible.
  if ! grep -q '__VERSION__' templates/install.sh; then
    fail "templates/install.sh has no __VERSION__ placeholder — the release would ship a hardcoded default (task-2)"
  fi
  rendered="$(mktemp)"
  trap 'rm -f "$rendered"' EXIT
  declared="v$(grep -m1 '^version = ' "$CARGO" | sed 's/.*"\(.*\)"/\1/')"
  if ! bash scripts/render-install.sh "$declared" "$rendered" 2>/dev/null; then
    fail "scripts/render-install.sh failed on templates/install.sh — the install one-liner asset cannot be built"
  elif ! grep -q "^VERSION=\${PIXI_SANDBOX_VERSION:-$declared}$" "$rendered"; then
    fail "rendering templates/install.sh does not install the declared release ($declared)"
  fi
  # Everything except the VERSION line must survive the render untouched.
  if ! diff -q <(grep -v '^VERSION=' templates/install.sh) <(grep -v '^VERSION=' "$rendered") >/dev/null; then
    fail "rendering templates/install.sh changes more than the VERSION line — the template and scripts/render-install.sh disagree"
  fi
fi
# A committed, ready-to-run install.sh anywhere outside templates/ is the drift this prevents.
if [ -e scripts/install.sh ] || [ -e init.sh ]; then
  fail "a committed install.sh exists outside templates/ — it would ship a hardcoded VERSION default that lags the release (task-2)"
fi

# ---------------------------------------------------------------- 6. no hardcoded release tags in CI

# A workflow that names a specific `vX.Y.Z` of ours is stale the moment a release lands, and it
# fails silently: the job keeps running against old binaries and still reports green. That is the
# worst failure mode a proof can have, and it is exactly what the airlock workflow's
# `vars.SANDBOX_RELEASE_VERSION || 'v0.3.0'` did — the weekly proof was exercising the previous
# release while claiming to prove the current one. Derive it (from the checkout, or from a
# repository variable a human can set without editing YAML) instead.
#
# Scoped to a `version:` value, so an example in a description or a comment is not a false
# positive; a genuine opt-out is `stale-ref-allowed` on the line.
pinned_tags="$(
  find .github/workflows -type f \( -name '*.yml' -o -name '*.yaml' \) -print0 |
    xargs -0 awk '
      FNR == 1 { prev = "" }
      {
        # A `version:` key, not any line containing the word — a `description: Explicit version to
        # cut (e.g. v1.2.3)` is help text, not a pin. Then any tag on that line counts, because the
        # real shape is `version: ${{ vars.X || '"'"'v0.3.0'"'"' }}`: the tag is separated from the
        # key by an expression, so a pattern anchored at the quote matched nothing and the check
        # passed on exactly the line it was written for.
        if ($0 !~ /stale-ref-allowed/ && prev !~ /stale-ref-allowed/ &&
            $0 ~ /(^|[[:space:]])version:[[:space:]]/ && $0 !~ /description:/) {
          line = $0
          while (match(line, /v[0-9]+\.[0-9]+\.[0-9]+/)) {
            printf "%s:%d: pins the literal %s\n", FILENAME, FNR, substr(line, RSTART, RLENGTH)
            line = substr(line, RSTART + RLENGTH)
          }
        }
        prev = $0
      }
    ' || true
)"
if [ -n "$pinned_tags" ]; then
  fail "a workflow pins a literal release tag, so it will silently prove a stale release after the next cut:"
  printf '  %s\n' "$pinned_tags" >&2
  echo "  read the tag from the checked-out Cargo.toml, or take it from a repository variable" >&2
fi

# ---------------------------------------------------------------- 7. self-contained actions

# v0.3.1 published two action paths that could not run. `setup/action.yml` and `publish/action.yml`
# each contained a single step, `uses: ../action.yml`, and a relative reference inside a *remote*
# composite action does not resolve against the repository that defines the action: `../` is
# rejected by the runner's reference parser ("Expected format {org}/{repo}[/path]@ref"), and `./`
# would resolve in the consumer's own checkout. So every `Archont561/pixi-sandbox/setup@<ref>`
# caller failed during job setup (issue #37). actionlint does not see this — it lints workflows,
# not action manifests — and no cargo test may read this repository's own files (D10), so the
# check belongs here.
#
# The fix makes each published path a complete composite action, generated from the root by
# scripts/render-action-shims.sh. Generation without verification is just a slower copy-paste, so
# both halves are checked: the copies re-render identically, and no published path contains a
# relative `uses:` (which would silently reintroduce the bug in a new place).
ACTION_PATHS="action.yml setup/action.yml publish/action.yml"
for action in $ACTION_PATHS; do
  if [ ! -f "$action" ]; then
    fail "$action is missing — it is a published action path (owner/repo[/path]@ref) (task-12)"
    continue
  fi
  relative="$(grep -nE '^[[:space:]]*(-[[:space:]]+)?uses:[[:space:]]*\.' "$action" || true)"
  if [ -n "$relative" ]; then
    fail "$action uses a relative action reference, which cannot resolve from a remote ref (task-12):"
    printf '  %s\n' "$relative" >&2
    echo "  a published action path must be self-contained; edit action.yml and run 'bash scripts/render-action-shims.sh'" >&2
  fi
  grep -q '^  using: composite$' "$action" ||
    fail "$action is not a composite action — a published action path must run on its own (task-12)"
done

for variant in setup publish; do
  generated="$variant/action.yml"
  [ -f "$generated" ] || continue
  rendered_action="$(mktemp)"
  if ! bash scripts/render-action-shims.sh "$variant" "$rendered_action" 2>/dev/null; then
    fail "scripts/render-action-shims.sh cannot render $generated from action.yml (task-12)"
  elif ! diff -u "$generated" "$rendered_action" >/dev/null; then
    fail "$generated has drifted from action.yml (task-12):"
    diff -u "$generated" "$rendered_action" | sed -n '1,20p' | sed 's/^/  /' >&2
    echo "  regenerate with 'bash scripts/render-action-shims.sh'" >&2
  fi
  rm -f "$rendered_action"
done

# ----------------------------------------------------------------

if [ "$failures" -gt 0 ]; then
  printf 'repo consistency: %d check(s) failed\n' "$failures" >&2
  exit 1
fi
echo "repo consistency: crates/ is free of prototype references; platform and version claims agree; action pins are immutable; the install one-liner is rendered from templates/; no workflow pins a literal release tag; every published action path is self-contained and matches action.yml"
