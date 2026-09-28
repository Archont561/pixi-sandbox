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
    write(&workflow, &github_workflow(version))?;
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

fn github_workflow(version: &str) -> String {
    format!(
        "name: publish sandbox\n\non:\n  workflow_dispatch:\n  push:\n    branches: [main]\n\npermissions:\n  contents: write\n\njobs:\n  publish:\n    uses: Archont561/pixi-sandbox/.github/workflows/publish-sandbox.yml@v{version}\n    with:\n      config: .pixi-sandbox.toml\n      release-repository: Archont561/pixi-sandbox\n      release-version: v{version}\n    secrets: inherit\n"
    )
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
