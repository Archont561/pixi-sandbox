#!/usr/bin/env bash
# Offline airlock gate. Asserts that a restored sandbox needs nothing from the network.
#
# Usage: bash scripts/airlock-gate.sh <restored-project> [env,env] [--transport <dir>] [--skip-cargo]
#
# This is the reusable body behind `.github/workflows/airlock.yml`, which runs it twice: once
# directly, and once with egress actually blocked. Keeping it in a script rather than in YAML is
# the same rule design.md §6 sets for workflow commands — a check this specific does not belong in
# a YAML `run:` block where nobody can review it.
#
# What it proves:
#   1. the bundled helper tools are the ones on PATH, so no runner-installed pixi/cargo can
#      quietly satisfy the checks below;
#   2. the bundled static binary actually executes on this host;
#   3. each environment prefix is a real installed prefix, not an empty directory that pixi
#      would happily "install" from the network;
#   4. with --transport: the restored tree *is the tree the manifest describes* — every file,
#      symlink target, executable bit and the fingerprint marker, checked against the per-file
#      digests the packer recorded (doctor --verify-restored, D13). Without it, against a
#      schema-1 transport, or while the bundled pixi-sandbox predates the oracle, this script
#      honestly stays a shape check and says so;
#   5. `pixi install --frozen --offline` needs no byte it did not already have;
#   6. `cargo check --offline` builds against the vendored tree.
#
# Two honest limits, learned the hard way against a real transport:
#   * `--offline` is a REQUEST, not an enforcement. With a network reachable, `pixi install
#     --frozen --offline` will happily re-fetch a damaged prefix and report success. That is why
#     assertion 3 exists, why assertion 4 checks content rather than shape, and why the
#     workflow's blocked run is the authoritative one.
#   * pixi writes its own bookkeeping (conda-meta/pixi, conda-meta/history, ...) the first time
#     it installs into a prefix that lacks it. A healthy restore triggers exactly one such
#     write, so the no-op check below measures the *second* install, after that settle.
#
# So: with --transport this is a self-sufficiency *and* integrity gate; without it, self-sufficiency.

set -euo pipefail

if [ $# -lt 1 ]; then
  sed -n '3,27p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
  exit 2
fi

PROJECT="$1"
# The env list is optional and positional, so an option may sit in second place.
if [ $# -ge 2 ] && [ "${2#-}" = "$2" ]; then
  ENVS="$2"
  shift 2
else
  ENVS="default"
  shift
fi

SKIP_CARGO=0
TRANSPORT=""
fail() {
  echo "::error::$*" >&2
  exit 1
}
while [ $# -gt 0 ]; do
  case "$1" in
    --skip-cargo) SKIP_CARGO=1; shift ;;
    --transport)
      [ $# -ge 2 ] || fail "--transport needs a directory"
      TRANSPORT="$2"
      shift 2
      ;;
    *)
      echo "::error::unknown option $1" >&2
      exit 2
      ;;
  esac
done

[ -d "$TRANSPORT" ] || [ -z "$TRANSPORT" ] ||
  fail "--transport $TRANSPORT is not a directory; the extracted branch is what the restored tree is checked against"

[ -d "$PROJECT" ] || fail "no restored project at $PROJECT"
[ -f "$PROJECT/.pixi/sandbox-env.sh" ] ||
  fail "$PROJECT/.pixi/sandbox-env.sh is missing; restore did not complete"

# ---------------------------------------------------------------- environment on PATH
# shellcheck disable=SC1091
source "$PROJECT/.pixi/sandbox-env.sh"

TOOLS_DIR="$PROJECT/.pixi/tools"
TOOLS_BIN="$(printf '%s\n' "$PATH" | cut -d: -f1)"
case "$TOOLS_BIN" in
  "$TOOLS_DIR"/*) ;;
  *) fail "the bundled tools directory is not first on PATH (got '$TOOLS_BIN'); a runner-installed tool would make every check below meaningless" ;;
esac

PLATFORM="$(basename "$TOOLS_BIN")"
echo "platform $PLATFORM · tools $TOOLS_BIN"

# The tools an airlocked host actually needs, which is *not* the whole catalogue: `pixi-pack`
# creates an environment but is never needed to restore one, so `pack` embeds only `pixi` and
# `pixi-unpack` alongside the bootstrap binary (crates/pixi-sandbox/src/commands/pack.rs). Asking
# for `pixi-pack` here would fail on a perfectly good transport.
for tool in pixi pixi-unpack pixi-sandbox; do
  candidate="$TOOLS_BIN/$tool"
  [ -x "$candidate" ] || [ -x "$candidate.exe" ] ||
    fail "bundled $tool is missing or not executable at $candidate"
done

# ---------------------------------------------------------------- the bundled binary runs
# The whole airlock story rests on this: a static musl helper that execs on a machine with no
# prefix and no runtime. `head -1` is applied afterwards rather than in the pipeline: closing a
# pipe early can SIGPIPE the producer, which `pipefail` would turn into a bogus failure.
BUNDLED_VERSION="$(command "$TOOLS_BIN/pixi" --version 2>/dev/null || true)"
BUNDLED_VERSION="${BUNDLED_VERSION%%$'\n'*}"
[ -n "$BUNDLED_VERSION" ] || fail "the bundled pixi could not execute on this host"
echo "bundled pixi: $BUNDLED_VERSION"

# ---------------------------------------------------------------- prefixes are real prefixes
# Restore extracts a complete prefix with `pixi-unpack` (restore.rs), so a healthy environment has
# a populated `conda-meta/`. Checking that *before* invoking pixi is what stops a damaged prefix
# from being silently repaired over the network and counted as a pass.
assert_prefix() {
  local env_name="$1" when="$2"
  local prefix="$PROJECT/.pixi/envs/$env_name"

  [ -d "$prefix" ] || fail "environment '$env_name' has no prefix at $prefix ($when)"
  # `find` on an absent directory is a non-zero exit, and under `set -o pipefail` that would abort
  # the whole script with `set -e` before the check below could explain itself. Ask first.
  records=0
  if [ -d "$prefix/conda-meta" ]; then
    records="$(find "$prefix/conda-meta" -maxdepth 1 -name '*.json' | wc -l | tr -d ' ')"
  fi
  [ "$records" -gt 0 ] ||
    fail "environment '$env_name' has an empty conda-meta ($when); the prefix was never populated, and a reachable network would let pixi paper over that"
  files="$(find "$prefix" -type f | wc -l | tr -d ' ')"
  [ "$files" -gt "$records" ] ||
    fail "environment '$env_name' holds only its $records conda-meta records and no payload ($when)"
  echo "  $env_name: $records package records, $files files ($when)"
}

# ---------------------------------------------------------------- the vendored cargo tree
# A tool the *environment* provides must come from the transport, not from the image — otherwise
# `cargo check` below would be testing the runner's toolchain against a vendored tree, which can
# pass for reasons that have nothing to do with the branch. An environment that ships no cargo at
# all is not a failure, so the check is conditional on the transport actually providing one.
ENV_CARGO=""
for env_bin in "$PROJECT"/.pixi/envs/*/bin; do
  [ -d "$env_bin" ] || continue
  for name in cargo cargo.exe; do
    if [ -x "$env_bin/$name" ]; then
      ENV_CARGO="$env_bin/$name"
      break 2
    fi
  done
done

CARGO_BIN=""
if [ -n "$ENV_CARGO" ]; then
  RESOLVED="$(command -v cargo 2>/dev/null || true)"
  [ "$RESOLVED" = "$ENV_CARGO" ] ||
    fail "cargo resolves to '${RESOLVED:-nothing}' but the transport provides '$ENV_CARGO'; the runner's toolchain would satisfy the check below for the wrong reason"
  CARGO_BIN="$ENV_CARGO"
  echo "cargo from transport: $CARGO_BIN"
else
  # No cargo in the environment. An image-provided one is still a useful witness: `--offline` plus
  # `--locked` proves the vendored tree satisfies the lockfile without a fetch. It says nothing
  # about where the toolchain came from, so the log must not imply otherwise.
  CARGO_BIN="$(command -v cargo 2>/dev/null || true)"
  if [ -z "$CARGO_BIN" ]; then
    if [ "$SKIP_CARGO" = "1" ]; then
      # Not exiting here: --skip-cargo skips the cargo section only — the integrity and
      # self-sufficiency sections below must still run.
      echo "::notice::no cargo available; skipping the vendored-tree check"
    else
      fail "no cargo available from the transport or the image; pass --skip-cargo if this project genuinely has no Rust workspace"
    fi
  fi
  echo "::notice::the restored environment ships no cargo; using the image's $CARGO_BIN. This proves the vendored tree is complete, not that the toolchain came from the branch."
fi

# ---------------------------------------------------------------- the tree matches the manifest
# Integrity, not shape: the restored prefix is compared file-by-file against the digests the
# packer recorded after its own verification unpack (D13). The binary doing the checking is
# the one the transport carried — restored and verified like every other tool — so the oracle
# and the checker travel together. Two honest degradations, each said out loud rather than
# papered over:
#   * no --transport: shape and self-sufficiency are checked, integrity is not;
#   * a bundled pixi-sandbox that predates the per-file oracle (a transport packed by an
#     older release): there is no oracle in that manifest to check against, and failing the
#     airlock for the release transition would teach operators to ignore this gate.
if [ -n "$TRANSPORT" ]; then
  SANDBOX_BIN="$TOOLS_BIN/pixi-sandbox"
  [ -x "$SANDBOX_BIN" ] || [ -f "$SANDBOX_BIN" ] ||
    fail "the bundled pixi-sandbox is missing at $SANDBOX_BIN; the integrity check cannot run"
  # `doctor --help` is the capability probe: only a binary that knows --verify-restored can
  # run the check. Help text goes through a variable rather than a pipe so `set -o pipefail`
  # cannot turn an early-exiting grep into a phantom verdict.
  help_text="$("$SANDBOX_BIN" doctor --help 2>/dev/null || true)"
  case "$help_text" in
    *--verify-restored*)
      echo "doctor --verify-restored against $TRANSPORT"
      "$SANDBOX_BIN" doctor --branch-location "$TRANSPORT" --verify-restored "$PROJECT" --envs "$ENVS" ||
        fail "the restored project does not match the manifest in $TRANSPORT — see the failures above"
      ;;
    *)
      echo "::notice::the bundled pixi-sandbox predates the per-file oracle (D13); restored-tree integrity cannot be checked for this transport"
      ;;
  esac
else
  echo "::notice::no --transport given: prefix shape and self-sufficiency are checked, integrity is not (D13)"
fi

# ---------------------------------------------------------------- the gate proper
for env_name in ${ENVS//,/ }; do
  assert_prefix "$env_name" "after restore"

  # pixi writes its own bookkeeping the first time it installs into a prefix that lacks it
  # (conda-meta/pixi, conda-meta/history, ...): a healthy restore triggers exactly one such
  # write, measured live against a real transport. Let that settle, then measure the second
  # install — the claim under test is that pixi needs no byte it did not get from the branch,
  # and after the settle that is true with zero exceptions.
  echo "pixi install --frozen --offline -e $env_name (settle: pixi's own first-run markers)"
  # pixi resolves the project from the working directory, and this script must not care where
  # it was invoked from (CI runs it from a checkout that is itself a pixi project — the wrong
  # one), so every pixi call runs inside the restored project.
  (cd "$PROJECT" && "$TOOLS_BIN/pixi" install --frozen --offline -e "$env_name") ||
    fail "pixi install --frozen --offline -e $env_name failed: the transport is not self-sufficient"

  # Counted as files and on-disk size rather than a checksum walk: the prefix is hundreds of MiB,
  # and the claim under test is "no byte was needed", which a growth check answers as well as a
  # byte-for-byte diff would.
  before="$(find "$PROJECT/.pixi" -type f | wc -l | tr -d ' '):$(du -sk "$PROJECT/.pixi" | cut -f1)"

  echo "pixi install --frozen --offline -e $env_name"
  (cd "$PROJECT" && "$TOOLS_BIN/pixi" install --frozen --offline -e "$env_name") ||
    fail "pixi install --frozen --offline -e $env_name failed: the transport is not self-sufficient"

  after="$(find "$PROJECT/.pixi" -type f | wc -l | tr -d ' '):$(du -sk "$PROJECT/.pixi" | cut -f1)"
  [ "$before" = "$after" ] ||
    fail "pixi install was not a no-op for '$env_name' (.pixi went from '$before' to '$after' files/KiB); it wrote bytes instead of using only what the branch carried"
  echo "  $env_name: install was a no-op (.pixi unchanged at $after)"

  assert_prefix "$env_name" "after the offline install"
done

if [ "$SKIP_CARGO" = "1" ] || [ ! -f "$PROJECT/Cargo.toml" ]; then
  echo "cargo check skipped (no Cargo.toml, or --skip-cargo)"
  exit 0
fi

[ -f "$PROJECT/.cargo/config.toml" ] ||
  fail "no .cargo/config.toml in the restored project; the vendored tree was not wired in"
grep -q 'source.crates-io' "$PROJECT/.cargo/config.toml" ||
  fail ".cargo/config.toml does not redirect crates.io; --offline would have to hit the network"

echo "cargo check --offline --locked"
( cd "$PROJECT" && "$CARGO_BIN" check --offline --locked --quiet ) ||
  fail "cargo check --offline failed; the vendored crate tree is incomplete"

echo "airlock gate passed for $PROJECT ($ENVS)"
