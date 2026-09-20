# `setup-pixi-sandbox`

A dependency-free composite GitHub Action that downloads a standalone `pixi-sandbox` release
binary, verifies it before execution, and adds it to `PATH`.

```yaml
- uses: OWNER/pixi-sandbox/.github/actions/setup-pixi-sandbox@<immutable-commit-sha>
  id: sandbox
  with:
    repository: OWNER/pixi-sandbox
    version: v0.2.0

- run: pixi-sandbox --version
```

## Release asset contract

For target triple `x86_64-unknown-linux-musl`, a release tag must publish:

```text
pixi-sandbox-x86_64-unknown-linux-musl
SHA256SUMS
```

The default asset template is `pixi-sandbox-{target}{exe}`. `{exe}` is `.exe` on Windows and
empty elsewhere. `SHA256SUMS` must contain exactly one normal `sha256sum` line or BSD-style line for each
selected asset, for example:

```text
0123...cdef  pixi-sandbox-x86_64-unknown-linux-musl
```

Ambiguous duplicate entries are rejected rather than resolved by file order.

After native airlock proof has accepted the release assets, generate this file with
`python3 scripts/write_release_checksums.py --output release/SHA256SUMS release/pixi-sandbox-*`
and upload both it and the matching binaries to the same release tag.

Supported automatic target mappings are Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows
x86_64/aarch64. Pass `target` and/or `asset-template` explicitly for a custom release layout.
An explicit target must be one Rust target-triple-like filename component (letters, digits, dots,
underscores, and hyphens): it cannot contain a path separator or alter the temporary install path.

## Security model

`repository` and `version` are required. `repository` is restricted to a canonical `OWNER/REPO`
coordinate, release asset names are single filenames, and automatic install paths stay below the
runner temporary directory. Use an immutable action commit SHA and an immutable release tag in
production. The action permits `version: latest` only when it is written explicitly, and emits a
warning because it is not a reproducible supply-chain input. An explicit `sha256` input can
replace the release checksum manifest when an organization distributes checksums through another
trusted channel.

The action verifies the downloaded bytes before making the file executable, then runs
`pixi-sandbox --version`. It emits `path`, resolved release `version`, `sha256`, and `target`.

The repository's `tests/actions/test_release_actions.py` starts a local fake release API to
exercise this checksum-before-execution behavior without GitHub credentials or a real release.
