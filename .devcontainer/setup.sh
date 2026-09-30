#!/usr/bin/env bash
# Devcontainer post-create setup. Kept out of devcontainer.json so each step
# can be commented and the JSON stays a one-liner (same approach as
# Archont561/qgis-rs).
set -euo pipefail

pixi --version

# Baseline CLI tooling that the pixi image does not ship with.
pixi global install git gh

# A C toolchain is needed to build the Rust crates. The conda compiler
# binaries are prefixed with the target triple, so expose the unprefixed
# names (cc/gcc/ar) that build scripts expect.
pixi global install \
  --expose cc \
  --expose gcc \
  --expose ar=x86_64-conda-linux-gnu-ar \
  c-compiler

# Project environments, pinned to the lockfile for reproducibility.
pixi install --locked --all

# Docs site dependencies (bun install --frozen-lockfile in docs/).
pixi run docs-install

# Install the OpenCode CLI and refresh its model catalog.
pixi run setup-opencode
