#!/usr/bin/env bash
# Install the conda package(s) that `pixi run package` just built and run the packaged binary
# on *this* runner (task-23, AC#2).
#
# Usage: bash scripts/smoke-conda-package.sh [output-dir]
#
# Why a script and not a task line: the interesting part is the failure message, and a failure
# that names the platform it happened on is the whole point — a package that builds on five
# runners but runs on four is exactly the failure a single-platform build can never see.
#
# `pixi global install --path <file.conda>` is used rather than a channel URL because it takes
# a plain filesystem path, so the identical line works on the Unix runners and on windows-latest
# (where a `file://` URL would have to be assembled from a Windows path). The installed binary
# is then run from the global bin directory as the `trampoline`, which is where pixi exposes it
# and which does not require `$PIXI_HOME/bin` to be on the caller's PATH.
set -euo pipefail

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${1:-.publish/out}"

log() { printf '%s\n' "$*" >&2; }
die() {
  printf '::error::%s\n' "$*" >&2
  exit 1
}

command -v pixi >/dev/null 2>&1 || die "pixi is not on PATH — install it before smoke-testing the package"

# The version the tree declares is the only version a package built from this tree may carry.
# `scripts/lint-repo-consistency.sh` check 3b proves the package *manifest* agrees; this proves
# the artifact that was actually produced agrees, which is the claim the release makes.
expected_version="$(sed -n 's/^version = "\([^"]*\)".*/\1/p' Cargo.toml | sed 1q)"
[ -n "$expected_version" ] || die "cannot read the workspace version from Cargo.toml"

mapfile -t packages < <(find "$OUT_DIR" -maxdepth 1 -type f -name '*.conda' | sort)
[ "${#packages[@]}" -gt 0 ] ||
  die "no .conda in $OUT_DIR — \`pixi run package\` produced nothing on this runner"

for package in "${packages[@]}"; do
  log "installing $(basename "$package")"
  pixi global install --path "$package" --force-reinstall >/dev/null ||
    die "the package for this platform does not install: $package"

  bin="${PIXI_HOME:-$HOME/.pixi}/bin/pixi-sandbox"
  [ -x "$bin" ] || bin="$bin.exe"
  [ -x "$bin" ] || die "pixi installed $package but exposed no runnable binary at $bin"

  reported="$("$bin" --version | awk '{print $NF}')" ||
    die "the packaged binary does not run on this platform: $package"
  [ "$reported" = "$expected_version" ] ||
    die "packaged binary reports $reported, but this tree is $expected_version ($package)"

  # --version proves it links and starts; a subcommand proves the CLI surface survived packaging.
  "$bin" --help >/dev/null || die "the packaged binary cannot parse its own arguments ($package)"

  log "ok: $(basename "$package") runs on this runner and reports $reported"
done
