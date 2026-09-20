# `setup-pixi-sandbox`

A dependency-free composite GitHub Action that downloads a standalone `pixi-sandbox` release
binary, verifies its SHA-256 checksum before execution, and adds it to `PATH`. Implemented in
pure POSIX shell (Linux/macOS) and PowerShell (Windows) with zero Python dependency.

Like `prefix-dev/setup-pixi`, you can use it from the repo root:

```yaml
- uses: Archont561/pixi-sandbox@v0.2.0
  id: sandbox
  with:
    version: v0.2.0
- run: pixi-sandbox --version
```

Or via explicit path (pin to immutable SHA for supply-chain security):

```yaml
- uses: Archont561/pixi-sandbox/.github/actions/setup-pixi-sandbox@<immutable-commit-sha>
  id: sandbox
  with:
    repository: Archont561/pixi-sandbox
    version: v0.2.0

- run: pixi-sandbox --version
```

`repository` defaults to `Archont561/pixi-sandbox` and `version` defaults to `latest` (warns), so minimal usage is just `uses: Archont561/pixi-sandbox@v0.2.0`.

## Release Asset Contract

For target triple `x86_64-unknown-linux-musl`, a release tag must publish:

```text
pixi-sandbox-x86_64-unknown-linux-musl
SHA256SUMS
```

The default asset template is `pixi-sandbox-{target}{exe}`. `{exe}` is `.exe` on Windows and
empty elsewhere. `SHA256SUMS` must contain exactly one standard `sha256sum` line or BSD-style line for each
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

## Security Model

- `repository` and `version` are strictly validated before making network requests.
- Automatic install paths stay below the runner temporary directory.
- The action verifies downloaded bytes against `SHA256SUMS` before granting execution permissions.
- Emits outputs: `path`, resolved release `version`, `sha256`, and `target`.
- Exercised by the native Rust integration test suite in `crates/pixi-sandbox/tests/actions.rs`.
