//! Generate the connected-side project files that publish and consume an offline sandbox.

use crate::cli::InitArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(args: InitArgs) -> Result<()> {
    match args.provider {
        crate::cli::InitProvider::Github => {}
    }
    let root = support::existing_dir(&args.project_root, "--project-root")?;
    let workflow = resolve(&root, &args.workflow);
    let config = resolve(&root, &args.config);
    let shell = resolve(&root, &args.restore_script);
    let powershell = resolve(&root, &args.powershell_script);

    for path in [&workflow, &shell, &powershell] {
        if path.exists() && !args.force {
            bail!(
                "{} already exists — pass --force to replace generated files",
                path.display()
            );
        }
    }

    let version = env!("CARGO_PKG_VERSION");
    let action_sha = latest_action_sha()?;
    write(&workflow, &github_workflow(version, &action_sha))?;
    if !config.exists() {
        write(&config, &default_config())?;
    }
    write(&shell, &posix_restore(&args.branch))?;
    support::make_executable(&shell)?;
    write(
        &powershell,
        &powershell_restore(&windows_branch(&args.branch)),
    )?;

    println!(
        "generated GitHub sandbox workflow at {}",
        workflow.display()
    );
    println!(
        "generated offline launchers at {} and {}",
        shell.display(),
        powershell.display()
    );
    Ok(())
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn write(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().expect("generated file always has a parent");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

fn default_config() -> String {
    "schema = 1\nbranch_prefix = \"sandbox\"\ncargo_vendor = true\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n".to_string()
}

fn latest_action_sha() -> Result<String> {
    if let Ok(sha) = std::env::var("PIXI_SANDBOX_ACTION_SHA") {
        return validate_sha(&sha);
    }

    let body = ureq::get("https://api.github.com/repos/Archont561/pixi-sandbox/commits/main")
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "pixi-sandbox-init")
        .call()
        .context("resolving the latest pixi-sandbox action commit")?
        .body_mut()
        .read_to_string()
        .context("reading the latest pixi-sandbox action commit")?;
    let response: serde_json::Value =
        serde_json::from_str(&body).context("parsing the latest pixi-sandbox action commit")?;
    let sha = response["sha"]
        .as_str()
        .context("GitHub's latest-commit response has no sha")?;
    validate_sha(sha)
}

fn validate_sha(sha: &str) -> Result<String> {
    if sha.len() == 40 && sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(sha.to_ascii_lowercase())
    } else {
        bail!("pixi-sandbox action SHA must be a full 40-character commit SHA")
    }
}

fn github_workflow(version: &str, action_sha: &str) -> String {
    let template = r#"name: publish sandbox

on:
  workflow_dispatch:
  push:
    branches: [main]

permissions:
  contents: write

jobs:
  plan:
    runs-on: ubuntu-latest
    outputs:
      matrix: ${{ steps.plan.outputs.matrix }}
    steps:
      - uses: actions/checkout@v7.0.1
      - id: sandbox
        uses: Archont561/pixi-sandbox/setup@__ACTION_SHA__
        with:
          version: v__VERSION__
      - id: plan
        shell: bash
        run: echo "matrix=$(pixi-sandbox plan --config .pixi-sandbox.toml --json)" >> "$GITHUB_OUTPUT"

  publish:
    needs: plan
    strategy:
      fail-fast: false
      matrix: ${{ fromJSON(needs.plan.outputs.matrix).include }}
    runs-on: ${{ matrix.runner }}
    steps:
      - uses: actions/checkout@v7.0.1
        with:
          persist-credentials: false
      - uses: prefix-dev/setup-pixi@v0.10.2
        with:
          cache: true
          environments: ${{ matrix.environments }}
      - id: sandbox
        uses: Archont561/pixi-sandbox/setup@__ACTION_SHA__
        with:
          version: v__VERSION__
      - uses: Archont561/pixi-sandbox/publish@__ACTION_SHA__
        with:
          project: .
          environments: ${{ matrix.environments }}
          platform: ${{ matrix.platform }}
          branch: ${{ matrix.branch }}
          cargo-vendor: ${{ matrix.cargo_vendor }}
          self-bin: ${{ steps.sandbox.outputs.path }}
          remote: ${{ github.server_url }}/${{ github.repository }}.git
          output-dir: ${{ runner.temp }}/pixi-sandbox-transport
          push-token: ${{ github.token }}
"#;
    template
        .replace("__VERSION__", version)
        .replace("__ACTION_SHA__", action_sha)
}

fn posix_restore(branch: &str) -> String {
    // The launcher stays a bootstrap: POSIX sh, no downloads, and no dependency on the tool it
    // is about to unpack. It prefers the branch declared in `.pixi-sandbox.toml` so a config
    // change (new bundle, renamed prefix) reaches every checkout without regenerating this file,
    // and falls back to the branch reviewed at init time when the config cannot decide.
    let template = r#"#!/bin/sh
set -eu
ROOT=$(git rev-parse --show-toplevel)
case $(uname -s)-$(uname -m) in
  Linux-x86_64) PLATFORM=linux-64 ;;
  Linux-aarch64|Linux-arm64) PLATFORM=linux-aarch64 ;;
  Darwin-arm64) PLATFORM=osx-arm64 ;;
  Darwin-x86_64) PLATFORM=osx-64 ;;
  *) echo "unsupported airlock platform: $(uname -s)-$(uname -m)" >&2; exit 2 ;;
esac
DEFAULT_BRANCH=__BRANCH__
case $DEFAULT_BRANCH in *linux-64) DEFAULT_BRANCH=${DEFAULT_BRANCH%linux-64}$PLATFORM ;; esac
BRANCH=${PIXI_SANDBOX_BRANCH:-}
CONFIG=$ROOT/.pixi-sandbox.toml
if [ -z "$BRANCH" ] && [ -r "$CONFIG" ]; then
  # <branch_prefix>/<bundle>-<platform>, read off the same reviewed plan the publisher uses.
  PREFIX=$(sed -n "s/^[[:space:]]*branch_prefix[[:space:]]*=[[:space:]]*[\"']\([^\"']*\).*/\1/p" "$CONFIG" | sed 1q)
  BUNDLES=$(tr '\n' ' ' <"$CONFIG" | sed 's/\[\[[[:space:]]*bundle[[:space:]]*\]\]/\
/g' | grep -E "platforms[^]]*[\"']$PLATFORM[\"']" |
    sed -n "s/.*name[[:space:]]*=[[:space:]]*[\"']\([^\"']*\).*/\1/p" || true)
  if [ -n "${PIXI_SANDBOX_BUNDLE:-}" ]; then
    BUNDLES=$(printf '%s\n' "$BUNDLES" | grep -Fx "$PIXI_SANDBOX_BUNDLE" || true)
  fi
  if [ "$(printf '%s' "$BUNDLES" | grep -c . || true)" = 1 ]; then
    BRANCH=${PREFIX:-sandbox}/$BUNDLES-$PLATFORM
  elif [ -n "$BUNDLES" ]; then
    echo "several bundles publish $PLATFORM; set PIXI_SANDBOX_BUNDLE to choose" >&2
  fi
fi
BRANCH=${BRANCH:-$DEFAULT_BRANCH}
if ! git -C "$ROOT" rev-parse --verify "$BRANCH^{commit}" >/dev/null 2>&1; then BRANCH=origin/$BRANCH; fi
TRANSPORT="$ROOT/.pixi/.restore-transport"
rm -rf "$TRANSPORT"
mkdir -p "$TRANSPORT"
git -C "$ROOT" archive "$BRANCH" | tar -x -C "$TRANSPORT"
BIN="$TRANSPORT/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox"
exec "$BIN" restore --branch-location "$TRANSPORT" --output-path "$ROOT" --force "$@"
"#;
    template.replace("__BRANCH__", branch)
}

fn windows_branch(branch: &str) -> String {
    branch
        .strip_suffix("linux-64")
        .map_or_else(|| branch.to_string(), |prefix| format!("{prefix}win-64"))
}

fn powershell_restore(branch: &str) -> String {
    let template = r#"$ErrorActionPreference = 'Stop'
$Root = (git rev-parse --show-toplevel).Trim()
$DefaultBranch = '__BRANCH__'
$Branch = $env:PIXI_SANDBOX_BRANCH
$Config = Join-Path $Root '.pixi-sandbox.toml'
if (-not $Branch -and (Test-Path $Config)) {
    # <branch_prefix>/<bundle>-win-64, read off the same reviewed plan the publisher uses.
    $Text = Get-Content -Raw $Config
    $Prefix = if ($Text -match '(?m)^\s*branch_prefix\s*=\s*["'']([^"'']*)') { $Matches[1] } else { 'sandbox' }
    $Bundles = @()
    foreach ($Chunk in ($Text -split '\[\[\s*bundle\s*\]\]')) {
        if ($Chunk -match 'platforms[^\]]*["'']win-64["'']' -and $Chunk -match 'name\s*=\s*["'']([^"'']*)') {
            $Bundles += $Matches[1]
        }
    }
    if ($env:PIXI_SANDBOX_BUNDLE) { $Bundles = @($Bundles | Where-Object { $_ -eq $env:PIXI_SANDBOX_BUNDLE }) }
    if ($Bundles.Count -eq 1) {
        $Branch = "$Prefix/$($Bundles[0])-win-64"
    } elseif ($Bundles.Count -gt 1) {
        Write-Error -Message 'several bundles publish win-64; set PIXI_SANDBOX_BUNDLE to choose' -ErrorAction Continue
    }
}
if (-not $Branch) { $Branch = $DefaultBranch }
git -C $Root rev-parse --verify "$Branch^{commit}" 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { $Branch = "origin/$Branch" }
$Transport = Join-Path $Root '.pixi/.restore-transport'
$Archive = Join-Path $Root '.pixi/.restore-transport.tar'
Remove-Item -Recurse -Force $Transport,$Archive -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Transport | Out-Null
git -C $Root archive --format=tar --output=$Archive $Branch
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
tar -xf $Archive -C $Transport
$Binary = Join-Path $Transport '.pixi-sandbox/tools/win-64/pixi-sandbox.exe'
& $Binary restore --branch-location $Transport --output-path $Root --force @args
exit $LASTEXITCODE
"#;
    template.replace("__BRANCH__", branch)
}

#[cfg(test)]
mod tests {
    use super::{posix_restore, powershell_restore, windows_branch};

    #[test]
    fn launchers_are_offline_bootstraps_not_installers() {
        let shell = posix_restore("sandbox/developer-linux-64");
        assert!(shell.contains("git -C \"$ROOT\" archive"));
        assert!(!shell.contains("curl"));
        let powershell = powershell_restore("sandbox/developer-win-64");
        assert!(powershell.contains("git -C $Root archive"));
        assert!(!powershell.contains("Invoke-WebRequest"));
    }

    #[test]
    fn conventional_linux_branch_maps_to_windows() {
        assert_eq!(
            windows_branch("sandbox/developer-linux-64"),
            "sandbox/developer-win-64"
        );
    }

    #[test]
    fn launchers_read_the_branch_off_the_reviewed_plan() {
        let shell = posix_restore("sandbox/developer-linux-64");
        // Branch identity comes from the config so it cannot drift from the publisher, while the
        // reviewed init-time branch stays as the fallback.
        assert!(shell.contains("CONFIG=$ROOT/.pixi-sandbox.toml"));
        assert!(shell.contains("branch_prefix"));
        assert!(shell.contains("BRANCH=${PREFIX:-sandbox}/$BUNDLES-$PLATFORM"));
        assert!(shell.contains("BRANCH=${BRANCH:-$DEFAULT_BRANCH}"));
        assert!(shell.contains("DEFAULT_BRANCH=sandbox/developer-linux-64"));

        let powershell = powershell_restore("sandbox/developer-win-64");
        assert!(powershell.contains(".pixi-sandbox.toml"));
        assert!(powershell.contains("branch_prefix"));
        assert!(powershell.contains("$Branch = \"$Prefix/$($Bundles[0])-win-64\""));
        assert!(powershell.contains("if (-not $Branch) { $Branch = $DefaultBranch }"));
        assert!(powershell.contains("$DefaultBranch = 'sandbox/developer-win-64'"));
    }

    #[test]
    fn an_explicit_branch_still_wins_over_the_config() {
        let shell = posix_restore("sandbox/developer-linux-64");
        // PIXI_SANDBOX_BRANCH is read before the config is consulted.
        let override_at = shell.find("BRANCH=${PIXI_SANDBOX_BRANCH:-}").unwrap();
        let derive_at = shell.find("CONFIG=$ROOT/.pixi-sandbox.toml").unwrap();
        assert!(override_at < derive_at);
        assert!(shell.contains("if [ -z \"$BRANCH\" ] && [ -r \"$CONFIG\" ]; then"));
    }
}
