#!/usr/bin/env bash
set -euo pipefail
BRANCH_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
OUTPUT_PATH="${1:-$PWD}"
if (( $# > 0 )); then shift; fi
exec "$BRANCH_DIR/pixi-sandbox" restore --branch-location "$BRANCH_DIR" --output-path "$OUTPUT_PATH" --force "$@"
