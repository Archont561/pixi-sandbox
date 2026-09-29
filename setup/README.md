# `setup`

Download, checksum-verify, and expose a standalone `pixi-sandbox` release binary. A
dependency-free composite action: pure POSIX shell (Linux/macOS) and PowerShell (Windows),
zero Python dependency.

The implementation is the repository-root `action.yml`; this file is a shim that selects
setup behaviour via its `subpath` input.

```yaml
# Short reference (same repo, like prefix-dev/setup-pixi)
- uses: Archont561/pixi-sandbox/setup@v0.3.2
  id: sandbox
  with:
    version: v0.3.2
- run: pixi-sandbox --version

# The repo root has always meant setup, so this works too
- uses: Archont561/pixi-sandbox@v0.3.2
  with:
    version: v0.3.2
```

Or drive the root action directly, pinning an immutable commit SHA for supply-chain security:

```yaml
- uses: Archont561/pixi-sandbox@<immutable-commit-sha>
  id: sandbox
  with:
    subpath: setup
    repository: Archont561/pixi-sandbox
    version: v0.3.2
```

`repository` defaults to `Archont561/pixi-sandbox` and `version` defaults to `latest` (warns),
so minimal usage is just `uses: Archont561/pixi-sandbox@v0.3.2`.

The publish counterpart is `Archont561/pixi-sandbox/publish@vX` (see `publish/README.md`).

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
`(cd release && sha256sum pixi-sandbox-* > SHA256SUMS)` and upload both it and the matching
binaries to the same release tag.

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
