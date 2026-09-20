# Release-driven sandbox publication

## Goal

A project should publish only the sandbox bundles it has explicitly reviewed, using a verified
released `pixi-sandbox` executable and a native runner for every target platform. This avoids
two failure modes:

1. a loose root-level tool-lock sidecar that an installed CLI does not have; and
2. a single Linux job claiming it published or validated macOS/Windows environments.

The implementation consists of four pieces:

| piece | role |
| --- | --- |
| `crates/pixi-sandbox-core/assets/tools.lock.json` | canonical helper-tool pins compiled into every Rust binary |
| `.pixi-sandbox.toml` | project-owned declaration of publishable bundles, platforms, and optional runner overrides |
| `pixi-sandbox plan --json` | validates that declaration and emits an Actions-compatible matrix |
| `setup-pixi-sandbox` + `publish-sandboxes.yml` | download/verify a release binary, then pack/verify/publish one native branch per matrix entry |

## Embedded helper-tool pins

`pack --fetch-tools` uses the JSON compiled into `pixi-sandbox-core`; a source clone and an
installed `.conda` package therefore have the same secure default. The canonical file stays JSON
rather than becoming Rust structs so a pin update remains a readable review diff.

An external lock is still available deliberately:

```bash
pixi-sandbox pack --fetch-tools --tools-lock /opt/company/reviewed-tools.lock.json …
```

Use that only for a reviewed internal mirror, an organisation-specific release policy, or an
emergency pin update. It is not a fallback to arbitrary `PATH` tools.

## Project schema: `.pixi-sandbox.toml`

```toml
schema = 1
branch_prefix = "sandbox"
cargo_vendor = true

# Override hosted runner labels when a target requires a self-hosted or specialised runner.
[runners]
osx-arm64 = "macos-14"

[[bundle]]
name = "developer"
environments = ["dev", "docs"]
platforms = ["linux-64", "osx-arm64", "win-64"]

[[bundle]]
name = "minimal"
environments = ["default"]
platforms = ["linux-64"]
cargo_vendor = false
```

The planner emits one target for each bundle/platform pair. The preceding example makes:

```text
sandbox/developer-linux-64
sandbox/developer-osx-arm64
sandbox/developer-win-64
sandbox/minimal-linux-64
```

Environment selection is always explicit. The planner must never infer that every environment
in `pixi.toml` is publishable; repositories routinely use private test, benchmark, or
platform-specific environments.

Validate before enabling a workflow:

```bash
pixi-sandbox plan --config .pixi-sandbox.toml
pixi-sandbox plan --config .pixi-sandbox.toml --json
```

The `runner` emitted by the planner is a single Actions runner label. For platforms without a
safe hosted default—currently `linux-aarch64`—the config must provide `[runners]` explicitly.
An override is not enough by itself: `plan` also requires compiled pins for `pixi`, `pixi-pack`,
and `pixi-unpack`. This prevents a reviewable-looking custom runner from reaching a native job
that cannot download its verified helper tools. Runner labels cannot contain control characters or
outer whitespace, and every `[runners]` entry must be used by at least one bundle; a stale typo is
therefore a configuration error instead of dormant CI policy.

## `setup-pixi-sandbox` action

The public composite action lives at `.github/actions/setup-pixi-sandbox/`. It uses only Python's
standard library and therefore runs before Pixi or Rust is installed. It:

1. maps the current runner OS/architecture to a Rust target triple;
2. resolves a GitHub release tag (`latest` is allowed but warned about);
3. downloads the binary release asset and `SHA256SUMS`;
4. verifies SHA-256 before making anything executable;
5. runs `pixi-sandbox --version` and exposes the executable path, resolved tag, digest, and
   target as Action outputs.

Release asset contract for a Linux x86_64 build:

```text
pixi-sandbox-x86_64-unknown-linux-musl
SHA256SUMS
```

After native proof accepts an artifact, generate the checksum asset from the exact bytes to be
uploaded:

```bash
python3 scripts/write_release_checksums.py --output release/SHA256SUMS release/pixi-sandbox-*
(cd release && sha256sum -c SHA256SUMS)
```

The helper sorts entries and rejects names outside the default
`pixi-sandbox-{target}{exe}` contract. The setup action requires exactly one matching checksum
entry, confines automatic installation below runner temporary storage, and rejects path-like
repository, target, tag, or asset-name inputs. It does not build, upload, or certify an artifact;
those remain deliberately separate from platform-specific bootstrap proof.

Use immutable references in production:

```yaml
- uses: OWNER/pixi-sandbox/.github/actions/setup-pixi-sandbox@<commit-sha>
  with:
    repository: OWNER/pixi-sandbox
    version: v0.2.0
```

`latest` is only suitable for an explicitly accepted development workflow. It remains mutable
even though its selected bytes are checksum-verified after resolution.

## Immutable workflow dependencies

Every third-party `uses:` reference in `.github/workflows/` is pinned to a full commit SHA; the
trailing `# v…` comment records the human-facing upstream release label only. The release-driven
workflow separately requires its `release-ref` input to be a 40-character SHA before checking
out the custom composite Actions. This prevents a moving tag from silently changing code that can
download executables or write sandbox branches.

When updating a dependency, resolve the intended upstream tag to its Git commit through the
upstream GitHub release/ref API, replace the SHA and label together, run `actionlint`, and review
the action's own release notes. Do not replace a SHA with a convenient tag merely to make a
workflow shorter.

## Native publish workflow

`publish-sandboxes.yml` is a `workflow_call` entry point. It checks out both the caller project
and the pinned action source (the workflow rejects `release-ref` values that are not a 40-character
commit SHA), uses `setup-pixi-sandbox` to obtain a verified release, asks the
binary to plan the project matrix, and then uses the matching runner for every publish job.

Each job:

```text
pixi install --frozen -e <each selected environment>
→ pixi-sandbox pack --fetch-tools --self-bin <verified release>
→ pixi-sandbox doctor --verify
→ pixi-sandbox publish
```

The same verified standalone release is embedded as the branch bootstrap executable. The
publisher rejects a job whose detected native platform does not equal the planned target.

A GitHub token is supplied to `git` through an in-memory `http.*.extraheader`; it is not put in
the remote URL or command arguments. The workflow uses a concurrency group keyed by repository
and branch so competing publishes cannot race. The publisher invokes the exact verified setup
output as `self-bin`, rather than resolving `pixi-sandbox` through `PATH`; the hermetic action
test covers a malicious PATH-shadow regression.

## Release acceptance and limits

The workflow is ready for a standalone release asset, but the current experimental static GNU
binary remains known-bad and must not be published. Before changing defaults from the Python
bootstrap, follow [the Rust bootstrap release checklist](rust-bootstrap.md): static/native
executable validation, full network-severed proof, and evidence for each target.

The embedded helper pins now cover Linux x86_64/aarch64, macOS x86_64/aarch64, and Windows x86_64
for Pixi, pixi-pack, and pixi-unpack. That makes planning/fetching possible; it does **not**
turn macOS or Windows into proven airlock targets. Native release and airlock proof results still
must be recorded separately.
