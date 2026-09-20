# Rust bootstrap release artifact

## Purpose

A sandbox branch needs an executable that can run `pixi-sandbox restore` on a fresh,
disconnected machine. Today that executable is the stdlib-only Python reference script
(`.knowledge/research/pixi_sandbox.py`). The Rust CLI implements the same restore behaviour and
has passed the full real transport smoke, but its release artifact must be independently usable
on every airlock target before it becomes the default embedded bootstrap.

This document defines how a release-produced Rust binary replaces that script.

## Current state

- Rust performs the connected-side `pack`, `doctor`, and `publish` steps in local tasks and CI.
- Rust `unpack` and `restore` are implemented and have restored the full real transport,
  including two Pixi environments and 168 vendored crates, with the network severed.
- The default branch self executable remains Python because it needs only `python3` and no
  Python packages.
- Do **not** use the current experimental `target/release/pixi-sandbox` artifact as a bootstrap:
  the static GNU-linking experiment produced a binary that immediately segfaulted.

The replacement is a deployment decision, not a behavioural-parity workaround. Rust restore is
functional; the unresolved work is producing and validating a portable release executable.

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

The packer names the supplied file `pixi-sandbox` in the transport regardless of its source
filename (and preserves the conventional `.exe` suffix for `win-*`):

```text
<transport>/.pixi-sandbox/tools/<platform>/pixi-sandbox
<transport>/.pixi-sandbox/tools/win-64/pixi-sandbox.exe
```

The airlock proof invokes that path with `restore`, so no Python is involved once a Rust release
binary has been embedded.

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

`reproduce.sh` defaults to the Python reference script, but accepts an override. Relative paths
are interpreted relative to the repository root:

```bash
PIXI_SANDBOX_SELF_BIN=artifacts/pixi-sandbox-linux-x86_64 \
  pixi run -e dev sandbox-proof
```

Or use an absolute path:

```bash
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
will be uploaded (the helper intentionally does **not** build or bless an artifact):

```bash
python3 scripts/write_release_checksums.py --output release/SHA256SUMS \
  release/pixi-sandbox-x86_64-unknown-linux-musl \
  release/pixi-sandbox-x86_64-apple-darwin \
  release/pixi-sandbox-aarch64-apple-darwin \
  release/pixi-sandbox-x86_64-pc-windows-msvc.exe
(cd release && sha256sum -c SHA256SUMS)
```

Attach each binary and the resulting `SHA256SUMS` to the same immutable GitHub release tag. The
setup Action accepts standard GNU or BSD SHA-256 lines; the helper emits deterministic GNU lines
sorted by asset filename. It rejects a filename that does not match the setup Action's default
`pixi-sandbox-{target}{exe}` convention, preventing a release layout typo from becoming a
runtime download failure.

## Reusable publishing workflow

`.github/workflows/publish-sandbox.yml` exposes a `self-binary` input. It becomes
`SANDBOX_SELF_BIN`, which the `ci-pack` task passes to `--self-bin`:

```yaml
jobs:
  sandbox:
    uses: OWNER/pixi-sandbox/.github/workflows/publish-sandbox.yml@<immutable-commit-sha>
    with:
      self-binary: artifacts/pixi-sandbox-linux-x86_64
```

The file must exist in the workflow checkout before `ci-pack` runs. A production workflow must
therefore build the binary first or download a verified release artifact into that path.

For configuration-driven native publishing, use the newer
`.github/workflows/publish-sandboxes.yml` instead. It calls `setup-pixi-sandbox` for a
checksum-verified release, uses `pixi-sandbox plan --json` to obtain the project matrix, and
passes that same verified binary as `--self-bin` for its one-native-platform publish job. Details
and configuration examples are in [`publish-automation.md`](publish-automation.md).

## Release acceptance checklist

For each supported platform:

1. Build the intended release artifact in trusted CI.
2. Check the binary locally (`--version`, expected architecture, and, on Linux, no dynamic ELF
   interpreter).
3. Pack with `--self-bin <artifact>` and run `doctor --verify`.
4. Run the full proof with `PIXI_SANDBOX_SELF_BIN=<artifact>` in a networkless namespace.
5. Verify the branch-contained executable, `pixi install --frozen --offline`, and
   `cargo build --offline` on a fresh project copy.
6. Record the artifact version, digest, target triple, and proof result in
   `research/EVIDENCE.md`.
7. Only then change the defaults in `reproduce.sh` and the reusable workflow from the Python
   script to the Rust release artifact.

Until every intended airlock target passes this checklist, retain the Python bootstrap as the
safe default.
