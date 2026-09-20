#!/usr/bin/env python3
"""Generate deterministic SHA256SUMS for release assets."""

import argparse
import hashlib
from pathlib import Path
import sys


def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while chunk := f.read(65536):
            h.update(chunk)
    return h.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", "-o", help="Output SHA256SUMS file path")
    parser.add_argument("assets", nargs="+", help="Release asset files")
    args = parser.parse_args()

    lines = []
    for asset_str in sorted(args.assets):
        asset = Path(asset_str)
        if not asset.is_file():
            print(f"Error: asset not found: {asset}", file=sys.stderr)
            return 1
        digest = sha256_file(asset)
        lines.append(f"{digest}  {asset.name}\n")

    output_content = "".join(lines)
    if args.output:
        out_path = Path(args.output)
        out_path.parent.mkdir(parents=True, exist_ok=True)
        out_path.write_text(output_content)
        print(f"Wrote checksums to {args.output}")
    else:
        sys.stdout.write(output_content)
    return 0


if __name__ == "__main__":
    sys.exit(main())
