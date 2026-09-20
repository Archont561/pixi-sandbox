# `publish-pixi-sandbox`

Internal companion composite action for the reusable `publish-sandboxes.yml` workflow and CI pipelines.
Implemented in pure POSIX shell (Linux/macOS) and PowerShell (Windows) with zero Python dependencies.
It deliberately publishes **one** `(bundle, platform)` target on the native runner selected by `pixi-sandbox plan`:

1. Installs the selected Pixi environments with `--frozen`;
2. Invokes the exact `self-bin` executable path from the verified setup action for `pack --fetch-tools` (never an unverified binary from `$PATH`);
3. Validates the generated transport payload with `doctor --verify`;
4. Force-pushes the requested orphan branch to the remote repository.

```yaml
- uses: Archont561/pixi-sandbox/.github/actions/publish-pixi-sandbox@<immutable-commit-sha>
  with:
    project: .
    environments: dev
    platform: linux-64
    branch: sandbox/linux-64
    push-token: ${{ secrets.GITHUB_TOKEN }}
```

## Security & Operational Model

- Refuses a runner whose native platform does not match the requested target platform.
- Push tokens are passed to Git via an in-memory `http.*.extraheader`, never exposed in remote URLs or command arguments.
- Branch and output paths are strictly validated against newlines and invalid path characters.
- Exercised by the native Rust integration test suite in `crates/pixi-sandbox/tests/actions.rs` and `crates/pixi-sandbox/tests/e2e.rs`.
