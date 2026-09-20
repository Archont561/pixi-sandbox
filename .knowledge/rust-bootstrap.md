# Rust bootstrap release artifact

## Purpose

A sandbox branch needs an executable that can run `pixi-sandbox restore` on a fresh,
disconnected machine. The Rust CLI now provides static binaries for all tier-1 targets,
replacing the earlier Python reference implementation. This document defines the contract for
release-produced Rust binaries.

## Current state

- Rust performs all connected-side `pack`, `doctor`, `publish` steps and airlock `unpack`/`restore`.
- Rust `unpack` and `restore` have restored full transports including vendored crates with network severed.
- The default branch self executable is the static Rust binary (musl on Linux, native on macOS/Windows).
- Release binaries are built by `release.yml` (5 targets), stripped, SHA256SUMS verified.
- One-liner offline reconstruction is available via `scripts/restore.sh` / `pixi run sandbox-restore`.

## Replacement contract

The file supplied through `--self-bin` must:

1. implement the same `pixi-sandbox restore` CLI and be compatible with the transport schema;
2. target the branch platform and CPU architecture exactly (for example, Linux x86_64 for a
   `linux-64` branch);
3. be executable without dependencies that are absent on the airlock;
4. pass the packer's linkage policy;
5. pass the full disconnected restore proof before it becomes a default.

On Linux, the packer rejects an ELF executable with a `PT_INTERP` program header, i.e. a
normally dynamically linked executable. A release bootstrap therefore needs a working static
(or static-PIE) Linux build. On macOS and Windows the current linkage classifier reports
`system`; those targets require a real target-host proof rather than relying on the Linux check.

The packer names the supplied file `pixi-sandbox` in the manifest payload regardless of its
source filename (and preserves the conventional `.exe` suffix for `win-*`). It also copies the
same bytes to the branch root, where the generated launchers call it:

```text
<transport>/pixi-sandbox
<transport>/restore.sh
<transport>/restore.ps1
<transport>/.pixi-sandbox/tools/<platform>/pixi-sandbox
<transport>/.pixi-sandbox/tools/win-64/pixi-sandbox.exe
```

The nested copy remains the manifest-verified payload; the root copy is a convenience extra, and
Git stores identical copies as one blob. The airlock proof invokes the root path (or
`restore.sh`) with `restore`, so no Python is involved once a Rust release binary has been embedded.

## Embed a release binary directly

Supply the release artifact with `--self-bin` when creating a transport:

```bash
pixi-sandbox pack \
  --repo-root . \
  --envs dev,docs \
  --output-dir .sandbox-transport \
  --platform linux-64 \
  --fetch-tools \
  --cargo-vendor \
  --self-bin /absolute/path/to/pixi-sandbox-linux-x86_64
```

The Rust packer copies it into the transport, marks it executable on Unix, records its size and
linkage in the manifest, and rejects a dynamic Linux executable. `doctor --verify` should then
be run before publication:

```bash
pixi-sandbox doctor --branch-location .sandbox-transport --verify
```

`--self-bin` is for a locally supplied release artifact, not a helper fetched from the binary's
embedded tool catalogue. Consequently its manifest entry has no external `pinned_sha256`; current
verification checks presence, size, and linkage. Produce it in trusted release CI and retain
its signed release checksum/provenance until manifest signing is introduced.

## Use the release binary in the end-to-end proof

Supply a release binary via `PIXI_SANDBOX_SELF_BIN`:

```bash
PIXI_SANDBOX_SELF_BIN=artifacts/pixi-sandbox-linux-x86_64 \
  pixi run -e dev sandbox-proof
# or absolute path
PIXI_SANDBOX_SELF_BIN=/releases/pixi-sandbox-linux-x86_64 \
  pixi run -e dev sandbox-proof
```

A successful run proves that the branch embeds the Rust executable and that it performs the
network-severed restore, followed by both acceptance assertions:

```text
pixi install --frozen --offline
cargo build --offline
```

For CI's lean proof environment, the `ci-proof` Pixi task sets
`PIXI_SANDBOX_RUST_ENV=ci` internally. `PIXI_SANDBOX_SELF_BIN` may still be supplied from the
step environment to select the release artifact.

For offline reconstruction from an existing sandbox branch:

```bash
bash scripts/restore.sh sandbox/linux-64   # or: pixi run sandbox-restore
```

## GitHub release asset contract

The release-driven setup action expects one native executable per supported Rust target plus a
release-level `SHA256SUMS` file. For example:

```text
pixi-sandbox-x86_64-unknown-linux-musl
pixi-sandbox-aarch64-unknown-linux-musl
pixi-sandbox-x86_64-apple-darwin
pixi-sandbox-aarch64-apple-darwin
pixi-sandbox-x86_64-pc-windows-msvc.exe
SHA256SUMS
```

`SHA256SUMS` must list exactly one digest for each exact asset filename in standard `sha256sum` or
BSD syntax. The `setup-pixi-sandbox` composite action resolves a tag, downloads both the selected executable
and checksum manifest, verifies bytes *before* running `--version`, and adds only the verified
file to `PATH`. It accepts `latest` only when a caller explicitly asks for it; release tags and
the action source commit must be pinned in production.

Publishing a checksum is not airlock proof. Do not publish an asset as a default bootstrap until
it has passed the platform-specific checklist below. The known-bad experimental Linux static
artifact must remain unpublished as a bootstrap candidate.

After the native proof is accepted, prepare the release checksum asset from the exact bytes that
will be uploaded:

```bash
(cd release && sha256sum pixi-sandbox-* > SHA256SUMS && sha256sum -c SHA256SUMS)
```

Attach each binary and the resulting `SHA256SUMS` to the same immutable GitHub release tag. The
setup action accepts GNU SHA-256 lines sorted by filename, matching `pixi-sandbox-{target}{exe}`.

## Reusable publishing workflow

`publish-sandbox.yml` is the unified publisher. It supports:

- `workflow_call`: caller passes optional `self-binary` (becomes `SANDBOX_SELF_BIN` → `--self-bin`)
- `workflow_dispatch`: manual publish
- `workflow_run`: auto-publish after `ci.yml` success

```yaml
jobs:
  sandbox:
    uses: Archont561/pixi-sandbox/.github/workflows/publish-sandbox.yml@<immutable-sha>
    with:
      self-binary: artifacts/pixi-sandbox-linux-x86_64
```

For configuration-driven native publishing, it calls `setup-pixi-sandbox` for a
checksum-verified release, uses `pixi-sandbox plan --json` to obtain the project matrix, and
passes that same verified binary as `--self-bin` for its one-native-platform publish job. Details
and configuration examples are in [`publish-automation.md`](publish-automation.md).

## Release acceptance checklist

For each supported platform:

1. Build the intended release artifact in trusted CI (`release.yml`).
2. Check the binary locally (`--version`, expected architecture, and, on Linux, no dynamic ELF interpreter).
3. Pack with `--self-bin <artifact>` and run `doctor --verify`.
4. Run the full proof with `PIXI_SANDBOX_SELF_BIN=<artifact>` in a networkless namespace.
5. Verify the branch-contained executable, `pixi install --frozen --offline`, and `cargo build --offline` on a fresh project copy.
6. Record the artifact version, digest, target triple, and proof result in `research/EVIDENCE.md`.
7. Validate offline reconstruction one-liner `scripts/restore.sh` against published branch.

All tier-1 targets now pass this checklist; Python bootstrap is removed.
