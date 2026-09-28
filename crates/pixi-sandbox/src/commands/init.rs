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
    format!(
        "#!/bin/sh\nset -eu\nROOT=$(git rev-parse --show-toplevel)\ncase $(uname -s)-$(uname -m) in\n  Linux-x86_64) PLATFORM=linux-64 ;;\n  Linux-aarch64|Linux-arm64) PLATFORM=linux-aarch64 ;;\n  Darwin-arm64) PLATFORM=osx-arm64 ;;\n  Darwin-x86_64) PLATFORM=osx-64 ;;\n  *) echo \"unsupported airlock platform: $(uname -s)-$(uname -m)\" >&2; exit 2 ;;\nesac\nDEFAULT_BRANCH={branch}\ncase $DEFAULT_BRANCH in *linux-64) DEFAULT_BRANCH=${{DEFAULT_BRANCH%linux-64}}$PLATFORM ;; esac\nBRANCH=${{PIXI_SANDBOX_BRANCH:-$DEFAULT_BRANCH}}\nif ! git -C \"$ROOT\" rev-parse --verify \"$BRANCH^{{commit}}\" >/dev/null 2>&1; then BRANCH=origin/$BRANCH; fi\nTRANSPORT=\"$ROOT/.pixi/.restore-transport\"\nrm -rf \"$TRANSPORT\"\nmkdir -p \"$TRANSPORT\"\ngit -C \"$ROOT\" archive \"$BRANCH\" | tar -x -C \"$TRANSPORT\"\nBIN=\"$TRANSPORT/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox\"\nexec \"$BIN\" restore --branch-location \"$TRANSPORT\" --output-path \"$ROOT\" --force \"$@\"\n"
    )
}

fn windows_branch(branch: &str) -> String {
    branch
        .strip_suffix("linux-64")
        .map_or_else(|| branch.to_string(), |prefix| format!("{prefix}win-64"))
}

fn powershell_restore(branch: &str) -> String {
    format!(
        "$ErrorActionPreference = 'Stop'\n$Root = (git rev-parse --show-toplevel).Trim()\n$Branch = if ($env:PIXI_SANDBOX_BRANCH) {{ $env:PIXI_SANDBOX_BRANCH }} else {{ '{branch}' }}\ngit -C $Root rev-parse --verify \"$Branch^{{commit}}\" 2>$null | Out-Null\nif ($LASTEXITCODE -ne 0) {{ $Branch = \"origin/$Branch\" }}\n$Transport = Join-Path $Root '.pixi/.restore-transport'\n$Archive = Join-Path $Root '.pixi/.restore-transport.tar'\nRemove-Item -Recurse -Force $Transport,$Archive -ErrorAction SilentlyContinue\nNew-Item -ItemType Directory -Force $Transport | Out-Null\ngit -C $Root archive --format=tar --output=$Archive $Branch\nif ($LASTEXITCODE -ne 0) {{ exit $LASTEXITCODE }}\ntar -xf $Archive -C $Transport\n$Binary = Join-Path $Transport '.pixi-sandbox/tools/win-64/pixi-sandbox.exe'\n& $Binary restore --branch-location $Transport --output-path $Root --force @args\nexit $LASTEXITCODE\n"
    )
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
}
