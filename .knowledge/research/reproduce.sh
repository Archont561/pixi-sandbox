#!/usr/bin/env bash
# reproduce.sh — cold-start proof of the whole design, from a bare checkout.
#
#   bash .knowledge/research/reproduce.sh [OUT_DIR] [--envs dev,docs] [--skip-install]
#
# What it does, in order:
#   1. installs the repository's own environments (skipped if already present);
#   2. packs them with the Rust CLI + `pixi-pack --directory-only`, vendors cargo dependencies,
#      and embeds pinned tools plus the portable bootstrap self executable;
#   3. publishes the transport as a single orphan-branch commit in a local bare repo;
#   4. extracts the branch into a fresh copy of the project and restores it **with the
#      network severed** (network namespace, when `unshare -rn` is available);
#   5. proves the result with the two acceptance assertions:
#        .pixi/tools/<platform>/pixi install --frozen --offline   (must be a no-op)
#        cargo build --offline                                    (must succeed)
#
# Sizes and timings are printed at every step; the numbers in EVIDENCE.md §10 come from
# this script. CI keeps it as the network-severed bootstrap proof while a static Rust release
# artifact is validated for every airlock target (design.md §9).
#
# The Rust binary is intentionally not required here: the branch carries the portable
# bootstrap program as its own `pixi-sandbox`. Once static Rust artifacts are available,
# `SANDBOX_SELF_BIN` can point at them and this proof can switch its restore step too.

set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
# The portable Python bootstrap is the default. A future validated static Rust binary can be
# selected deliberately without changing this proof script. Interpret an override relative to
# the project root, just like the CLI's other project paths.
SB="${PIXI_SANDBOX_SELF_BIN:-$HERE/pixi_sandbox.py}"
case "$SB" in
    /*) ;;
    *) SB="$ROOT/$SB" ;;
esac
# Local `sandbox-proof` runs from dev; CI's lean proof task overrides this to `ci`.
RUST_ENV="${PIXI_SANDBOX_RUST_ENV:-dev}"

OUT="${1:-$ROOT/.sandbox-proof}"
shift || true
ENVS="dev,docs"
SKIP_INSTALL=0
while [ $# -gt 0 ]; do
    case "$1" in
        --envs) ENVS="${2:?}"; shift 2 ;;
        --skip-install) SKIP_INSTALL=1; shift ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
done

PLATFORM="linux-64"
BRANCH="sandbox/$(echo "$ENVS" | tr ',' '+')-$PLATFORM"
TRANSPORT="$OUT/transport"
REMOTE="$OUT/remote.git"
AIRLOCK="$OUT/airlock"
CACHE="${PIXI_SANDBOX_TOOLS_CACHE:-$HOME/.cache/pixi-sandbox/tools}"
STARTED=$(date +%s)

step() { printf '\n\033[1m== %s\033[0m\n' "$*"; }
note() { printf '   %s\n' "$*"; }
mb()   { du -sm "$1" 2>/dev/null | cut -f1; }
# Connected-side proof commands run through the Rust CLI in the installed dev environment.
# The embedded self binary remains the portable bootstrap script until static Rust artifacts
# are available for every airlock platform (design.md §9).
rust_sandbox() { ( cd "$ROOT" && "$PIXI" run -e "$RUST_ENV" -- cargo run -p pixi-sandbox -- "$@" ); }

command -v python3 >/dev/null || { echo "python3 is required for the bootstrap self binary" >&2; exit 1; }
command -v git >/dev/null || { echo "git is required" >&2; exit 1; }
PIXI="$(command -v pixi || echo "$HOME/.pixi/bin/pixi")"
[ -x "$PIXI" ] || { echo "pixi is required (https://pixi.sh)" >&2; exit 1; }

step "0. preflight"
rm -rf "$OUT"
mkdir -p "$OUT"
note "repo    $ROOT"
note "output  $OUT"
note "envs    $ENVS ($PLATFORM)"
note "pixi    $("$PIXI" --version)"
if command -v unshare >/dev/null && unshare -rn true 2>/dev/null; then
    NET_SEVERED=1; note "network: the restore phase runs in a namespace with no interfaces (unshare -rn)"
else
    NET_SEVERED=0; note "network: unshare unavailable — the restore phase is NOT isolated (design proof is weaker)"
fi

if [ "$SKIP_INSTALL" = 0 ]; then
    step "1. install the project's environments"
    # Pixi 0.81 accepts one `-e` value at a time; a comma-separated list is an invalid
    # environment name, not an "install these" shorthand.
    for env in ${ENVS//,/ }; do
        ( cd "$ROOT" && time "$PIXI" install -e "$env" --frozen ) || {
            echo "pixi install failed for $env — run it once by hand to see the solver output" >&2; exit 1; }
    done
fi
for env in ${ENVS//,/ }; do
    [ -d "$ROOT/.pixi/envs/$env" ] || { echo "missing $ROOT/.pixi/envs/$env — run \`pixi install -e $env\`" >&2; exit 1; }
done

step "2. Rust pack + doctor (envs, cargo vendor, pinned tools)"
rust_sandbox pack --repo-root "$ROOT" --envs "$ENVS" --output-dir "$TRANSPORT" \
    --platform "$PLATFORM" --cargo-vendor --fetch-tools \
    --tools-cache "$CACHE" --self-bin "$SB"
rust_sandbox doctor --branch-location "$TRANSPORT" --verify
note "transport: $(mb "$TRANSPORT") MB"

step "3. Rust publish to an orphan branch"
git init -q --bare "$REMOTE"
rust_sandbox publish --input-dir "$TRANSPORT" --branch-name "$BRANCH" --remote "$REMOTE"

step "4. airlock: a fresh project copy + the branch, then restore"
rm -rf "$AIRLOCK"; mkdir -p "$AIRLOCK/project" "$AIRLOCK/branch"
# the project's *source*, without any generated weight — exactly what the airlock would have
tar -C "$ROOT" -cf - \
    --exclude=./.pixi --exclude=./target --exclude=./.sandbox-proof --exclude=./.sandbox-out \
    --exclude=./.sandbox-transport --exclude=./docs/node_modules --exclude=./docs/dist \
    --exclude=./.git --exclude=./.knowledge/research/.repro-out . | tar -C "$AIRLOCK/project" -xf -
git --git-dir="$REMOTE" archive "$BRANCH" | tar -x -C "$AIRLOCK/branch"
note "branch extracted: $(mb "$AIRLOCK/branch") MB"

restore_and_assert() {
    # This function is also evaluated in the separate airlock shell. Make failure propagation
    # explicit there too: `tail` must never hide a failed restore or acceptance assertion.
    set -euo pipefail
    local project="$1" branch="$2"
    local shipped="$branch/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox"
    chmod +x "$shipped" 2>/dev/null || true
    "$shipped" restore --branch-location "$branch" --path-to-main-repo-code "$project"
    # the two assertions the flow exists for
    ( cd "$project" && ".pixi/tools/$PLATFORM/pixi" install --frozen --offline ) \
        | tail -3
    (
        cd "$project"
        export PATH="$project/.pixi/envs/dev/bin:$PATH"
        export CARGO_HOME="$project/.pixi/.cargo-home"
        export CARGO_NET_OFFLINE=true
        time cargo build --offline
    ) | tail -3
}
if [ "$NET_SEVERED" = 1 ]; then
    # Pass paths positionally rather than interpolating them into shell source. `bash -euo
    # pipefail` is deliberate: the namespace boundary must retain the outer proof's strict
    # failure semantics.
    unshare -rn bash -euo pipefail -c "$(declare -f restore_and_assert); PLATFORM=\$1; restore_and_assert \"\$2\" \"\$3\"" \
        bash "$PLATFORM" "$AIRLOCK/project" "$AIRLOCK/branch"
else
    restore_and_assert "$AIRLOCK/project" "$AIRLOCK/branch"
fi

step "done in $(( $(date +%s) - STARTED ))s"
note "transport $(mb "$TRANSPORT") MB → branch $(mb "$AIRLOCK/branch") MB → restored envs + vendor in $AIRLOCK/project"
note "compare the numbers with EVIDENCE.md §10; they should be identical modulo the vendor tree's crate count"
