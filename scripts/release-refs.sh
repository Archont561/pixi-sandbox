#!/usr/bin/env bash
# The single definition of "a reference to OUR release", used by both the lint that reports the
# drift and the release script that fixes it.
#
# Usage:
#   bash scripts/release-refs.sh root
#   bash scripts/release-refs.sh scan
#   bash scripts/release-refs.sh rewrite <old-tag> <new-tag>
#
# Why this is one script and not a predicate pasted into each caller: the bug this exists to kill
# is a release commit that bumps the manifests and leaves the ~90 documentation references on the
# previous tag. The check (lint-repo-consistency.sh) and the fix (prepare-release.sh) must agree on
# exactly which references are ours, and two hand-maintained copies of the same predicate are two
# things that drift. Here they are one function.
#
# What counts as OURS — a line qualifies when it names this project:
#   * `Archont561/pixi-sandbox[/subpath]@vX.Y.Z`   (a `uses:` ref or a submodule example)
#   * `releases/download/vX.Y.Z/<asset>`            (the install one-liner and its siblings)
#   * `version: vX.Y.Z`                             (the setup action's version input)
#   * `PIXI_SANDBOX_VERSION=vX.Y.Z`                 (the environment override)
#   * `` `uses: @vX.Y.Z` ``                         (the bare form used in prose)
# A third-party pin on the same line (`actions/checkout@v7.0.1`, `setup-pixi@v0.10.2`) is someone
# else's release: the repository is deliberately not the authority on `setup-pixi`'s version, so
# such a line is skipped entirely rather than rewritten. That is also why this can never be a
# blind `grep -r v[0-9]`.
#
# A reference that must stay on an older release (a deliberate upgrade walkthrough, a regression
# fixture) opts out with `stale-ref-allowed` on the line or the line above it, and is left alone by
# both modes.

set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# ---------------------------------------------------------------- the file set
#
# Markdown and MDX only, and never the places that are historical by construction: a changelog and a
# closed task record are *supposed* to name old releases. `.knowledge/` holds design history.
# Excluding them is what lets `scan` assert "everything you publish says 0.3.0" without tripping
# over the sentence that documents the regression it just fixed.
#
# The exclusion list is a deliberate control, not a convenience: a file that legitimately must cite
# an older tag opts out with `stale-ref-allowed` on the line, which is visible in a diff and
# reviewable. Silently excluding a path here would hide a real reference from both modes.

# Prints the null-separated set of files to inspect.
ref_files() {
  find . -type f \( -name '*.md' -o -name '*.mdx' \) \
    -not -path './.git/*' \
    -not -path './.knowledge/*' \
    -not -path './node_modules/*' \
    -not -path './docs/node_modules/*' \
    -not -path './docs/dist/*' \
    -not -path './backlog/*' \
    -not -name 'CHANGELOG.md' \
    -print0
}

# The declared version, read the way prepare-release.sh writes it: the single top-level
# `version = "…"` that starts at column 0. Dependency pins are indented, so this cannot match one.
declared_version() {
  sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | sed 1q
}

# The canonical project identity. `action.yml`'s `repository` input default is the authority: it is
# what the setup action downloads from, so a fork that changes the input has already changed which
# repository the docs are *supposed* to point at.
project_root() {
  sed -n '/^  repository:/,/^  [a-z-]*:$/p' action.yml |
    sed -n 's/^    default: //p' | sed 1q
}

# ---------------------------------------------------------------- shared predicate
#
# One awk program, three modes, so `scan` and `rewrite` cannot disagree about what is ours.
#   mode=report   print every token that is not the declared version
#   mode=mark     print `FILENAME LINE` for each line that qualifies (the rewrite worklist)
#   mode=rewrite  unused
#
# The division of labour is deliberate: awk decides WHICH lines are ours, and `sed` does the actual
# substitution. An earlier version had awk write a replacement file itself and hit two byte-level
# traps. First, it wrote only the lines it matched, which truncated every file it touched (1978
# lines of documentation deleted in one release run). Second, writing the whole file with `print`
# appends a newline, so a file whose last line has none silently gained one and compared as
# "changed" — and the obvious fix, awk's `RT`, is a gawk extension that is silently empty under
# mawk, which this box runs, collapsing every file onto a single line. `sed` has neither problem:
# it edits in place and preserves a missing trailing newline, which is verified rather than assumed.
read -r -d '' AWK_PROGRAM <<'AWK' || true
FNR == 1 { prev = "" }
{
  ours = (index($0, root) || $0 ~ /releases\/download\/v[0-9]/ || $0 ~ /version: v[0-9]/ ||
          $0 ~ /PIXI_SANDBOX_VERSION=v[0-9]/ || $0 ~ /`uses: @v[0-9]/)
  allowed = ($0 ~ /stale-ref-allowed/ || prev ~ /stale-ref-allowed/)
  if (ours && !allowed) {
    if (mode == "mark") {
      printf "%s %d\n", FILENAME, FNR
    } else {
      line = $0
      while (match(line, /v[0-9]+\.[0-9]+\.[0-9]+/)) {
        tok = substr(line, RSTART, RLENGTH)
        if (tok != cur) printf "%s:%d: pins %s, manifests declare %s\n", FILENAME, FNR, tok, cur
        line = substr(line, RSTART + RLENGTH)
      }
    }
  }
  prev = $0
}
AWK

# ---------------------------------------------------------------- modes

cmd_root() {
  project_root
}

cmd_scan() {
  local current root
  current="$(declared_version)"
  if [ -z "$current" ]; then
    printf '::error::Cargo.toml has no parsable top-level version\n' >&2
    return 1
  fi
  root="$(project_root)"
  if [ -z "$root" ]; then
    printf "::error::action.yml has no parsable 'repository' default — cannot tell our refs from a third party's\n" >&2
    return 1
  fi
  ref_files |
    xargs -0 awk -v mode=report -v cur="v$current" -v root="$root" "$AWK_PROGRAM" || true
}

cmd_rewrite() {
  local old_tag="$1" new_tag="$2"
  if [ -z "$old_tag" ] || [ -z "$new_tag" ]; then
    printf 'usage: release-refs.sh rewrite <old-tag> <new-tag>\n' >&2
    return 1
  fi
  # The rewrite is destination-driven: every version token on a line that names this project
  # becomes `new_tag`, whatever it was. `old_tag` is therefore not a filter — it is the caller's
  # claim about what is being replaced, and it is checked against reality so a stale or wrong
  # claim fails loudly instead of quietly repinning a tree that was not where the caller thought.
  # (Not filtering on it is deliberate: a doc that had already drifted to some third version is
  # exactly what `scan` exists to report, and refusing to fix it would leave the drift in place.)
  local root current
  root="$(project_root)"
  if [ -z "$root" ]; then
    printf "::error::action.yml has no parsable 'repository' default — refusing to guess what to rewrite\n" >&2
    return 1
  fi
  current="$(declared_version)"
  if [ -z "$current" ]; then
    printf '::error::Cargo.toml has no parsable top-level version\n' >&2
    return 1
  fi
  if [ "v$current" != "$old_tag" ]; then
    printf '::error::asked to replace %s but the tree declares %s — re-read the current version\n' "$old_tag" "v$current" >&2
    return 1
  fi

  # awk decides which lines are ours; sed edits exactly those, in place, byte-faithfully.
  # Grouping the worklist by file keeps this to one sed per file rather than one per line.
  #
  # A subshell, not `local` + EXIT trap: the trap would fire when this function returns, at which
  # point the `local` names are already out of scope and `set -u` turned the cleanup into an
  # "unbound variable" abort — after the work was done and before the caller was told.
  local count
  count="$(
    worklist="$(mktemp)"
    touched="$(mktemp)"
    # shellcheck disable=SC2064  # expand now: the paths are per-run, and the trap runs in here
    trap 'rm -f "$worklist" "$touched"' EXIT

    ref_files |
      xargs -0 awk -v mode=mark -v root="$root" "$AWK_PROGRAM" >"$worklist" || true

    local file lines
    # Filenames cannot contain a newline, so a space is an unambiguous field separator here.
    while IFS= read -r file; do
      [ -n "$file" ] || continue
      lines="$(awk -v f="$file" '$1 == f { printf "%s,", $2 }' "$worklist")"
      lines="${lines%,}"
      [ -n "$lines" ] || continue
      # One `-e` per line number. sed wants each address as its own script, and the two obvious
      # compact spellings (`-e '17;31s/…/'` and `'17,31s/…/'`) are both syntax errors — the second
      # reports `unknown command: ','`, which the loop would otherwise swallow into a rewrite that
      # silently touched nothing while still reporting files as moved.
      local -a script=()
      local n
      for n in ${lines//,/ }; do
        script+=(-e "${n} s/v[0-9]+\\.[0-9]+\\.[0-9]+/${new_tag}/g")
      done
      # `-i` keeps this in place and, unlike `sed > tmp && mv`, preserves ownership and a missing
      # trailing newline. The addresses are line numbers, so a substitution can never cascade onto
      # a line the predicate did not approve.
      if ! sed -i -E "${script[@]}" "$file"; then
        printf '::error::failed to repin %s\n' "$file" >&2
        return 1
      fi
      printf '%s\n' "$file" >>"$touched"
    done < <(cut -d' ' -f1 "$worklist" | sort -u)

    wc -l <"$touched"
  )"

  printf 'repin: %s file(s) moved to %s\n' "$count" "$new_tag" >&2
  return 0
}

case "${1:-}" in
root) cmd_root ;;
scan) cmd_scan ;;
rewrite) shift; cmd_rewrite "${1:-}" "${2:-}" ;;
*)
  printf 'usage: release-refs.sh {root|scan|rewrite <old> <new>}\n' >&2
  exit 2
  ;;
esac
