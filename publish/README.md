# `publish`

Install requested Pixi environments, pack with a verified release binary, verify every blob,
and force-push one orphan branch. Used by the reusable `publish-sandbox.yml` workflow and by CI
pipelines. Pure POSIX shell (Linux/macOS) and PowerShell (Windows), zero Python dependencies.

The implementation is the repository-root `action.yml`; this file is a shim that selects
publish behaviour via its `subpath` input. It deliberately publishes **one**
`(bundle, platform)` target on the native runner selected by `pixi-sandbox plan`:

1. Installs the selected Pixi environments with `--frozen`;
2. Invokes the exact `self-bin` executable path from the verified setup action for `pack --fetch-tools` (never an unverified binary from `$PATH`);
3. Validates the generated transport payload with `doctor --verify`;
4. Force-pushes the requested orphan branch to the remote repository.

```yaml
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev
    platform: linux-64
    branch: sandbox/dev-linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
```

Or drive the root action directly, pinning an immutable commit SHA:

```yaml
- uses: Archont561/pixi-sandbox@<immutable-commit-sha>
  with:
    subpath: publish
    project: .
    environments: dev
    platform: linux-64
    branch: sandbox/linux-64
    push-token: ${{ secrets.GITHUB_TOKEN }}
```

Pair with setup:

```yaml
- uses: Archont561/pixi-sandbox/setup@v0.2.0
  id: setup
  with:
    version: v0.2.0
- uses: Archont561/pixi-sandbox/publish@v0.2.0
  with:
    project: .
    environments: dev
    platform: linux-64
    branch: sandbox/linux-64
    self-bin: ${{ steps.setup.outputs.path }}
    remote: https://github.com/OWNER/REPO.git
    output-dir: /tmp/transport
```

Setup counterpart is `Archont561/pixi-sandbox@vX` or `Archont561/pixi-sandbox/setup@vX`.

## Security & Operational Model

- Refuses a runner whose native platform does not match the requested target platform.
- Push tokens are passed to Git via an in-memory `http.*.extraheader`, never exposed in remote URLs or command arguments.
- Branch and output paths are strictly validated against newlines and invalid path characters.
- Because the root action shares one `inputs:` block with setup, the publish inputs are
  optional there and validated at run time by the publish step; this shim still declares them
  `required: true` because setup callers never load it.
- Exercised by the native Rust integration test suite in `crates/pixi-sandbox/tests/actions.rs` and `crates/pixi-sandbox/tests/e2e.rs`.
