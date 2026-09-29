#!/usr/bin/env bash
# Validate the workflow that `pixi-sandbox init github` GENERATES, not the ones committed here.
#
# Usage: bash scripts/lint-generated-workflow.sh   (or `pixi run lint-generated-workflow`)
#
# `pixi run lint-actions` runs actionlint over `.github/workflows/`, which is this repository's
# own CI. The generated workflow is a different artifact with a different failure mode, and
# nothing looked at it: v0.3.1 shipped an `init github` that emitted
# `matrix: ${{ fromJSON(needs.plan.outputs.matrix).include }}`, handing `strategy.matrix` an
# array where Actions requires an object. Every consumer's plan job went green and the publish
# job was never created (issue #37, task-12). The generator is Rust, so a unit test pins the
# text — but only a real Actions linter can say the *result* is a valid workflow, so it is run
# here against a throwaway project, in the same `lint` gate CI already runs.
#
# What this does NOT do, measured rather than assumed: actionlint accepts the broken
# `matrix: ${{ fromJSON(...).include }}` form (exit 0 on a workflow built to carry exactly that
# bug), because the value is an expression it will not evaluate. The matrix *shape* is therefore
# pinned by `tests/cli.rs::init_github_generates_an_object_shaped_publish_matrix`; this gate
# covers everything else about the generated file — YAML validity, unknown keys, bad `runs-on`,
# expression syntax, undefined `needs`.
#
# Offline by construction: `PIXI_SANDBOX_ACTION_SHA` is what `init` uses instead of asking
# api.github.com for the latest action commit, so this task never reaches the network (the
# airlock has none).
set -euo pipefail

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel 2>/dev/null || dirname "$SCRIPT_DIR")"
cd "$REPO_ROOT"

command -v actionlint >/dev/null 2>&1 || die "actionlint is not on PATH — run this inside the pixi environment"

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

# A real-looking 40-character SHA: `init` validates the shape, and pinning this repository's own
# HEAD keeps the generated `uses:` lines representative of what a user gets.
ACTION_SHA="$(git rev-parse HEAD 2>/dev/null || true)"
case "$ACTION_SHA" in
  [0-9a-f]*) [ "${#ACTION_SHA}" -eq 40 ] || ACTION_SHA="" ;;
  *) ACTION_SHA="" ;;
esac
[ -n "$ACTION_SHA" ] || ACTION_SHA="0123456789abcdef0123456789abcdef01234567"

PIXI_SANDBOX_ACTION_SHA="$ACTION_SHA" \
  cargo run -q -p pixi-sandbox -- init github \
  --project-root "$WORK" \
  --branch sandbox/developer-linux-64 >/dev/null ||
  die "pixi-sandbox init github failed — the generator cannot produce a project to lint"

GENERATED="$WORK/.github/workflows/publish-sandbox.yml"
[ -f "$GENERATED" ] || die "init github did not write $GENERATED"

# actionlint resolves config relative to the working directory, so run it where the generated
# project lives; a bare path outside a repository is linted with defaults.
(cd "$WORK" && actionlint .github/workflows/publish-sandbox.yml) ||
  die "actionlint rejects the workflow 'init github' generates — fix github_workflow() in crates/pixi-sandbox/src/commands/init.rs"

log "generated workflow: actionlint clean (strategy.matrix shape is asserted by tests/cli.rs)"
