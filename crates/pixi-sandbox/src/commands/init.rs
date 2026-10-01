//! Generate the connected-side project files that publish and consume an offline sandbox.

use crate::cli::InitArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::generated::{GENERATED_MARKER, GithubWorkflowOptions, render_github_workflow};
use std::fs;
use std::path::{Path, PathBuf};

const PREFERRED_CONFIG: &str = "pixi-sandbox.toml";
const LEGACY_CONFIG: &str = ".pixi-sandbox.toml";

#[derive(Debug, Clone, Copy)]
enum LauncherKind {
    Posix,
    PowerShell,
}

impl LauncherKind {
    fn current() -> Self {
        if cfg!(windows) {
            Self::PowerShell
        } else {
            Self::Posix
        }
    }

    fn default_path(self) -> &'static Path {
        match self {
            Self::Posix => Path::new("restore.sh"),
            Self::PowerShell => Path::new("restore.ps1"),
        }
    }

    fn render(self, branch: &str, config: &Path) -> String {
        match self {
            Self::Posix => posix_restore(branch, config),
            Self::PowerShell => powershell_restore(&windows_branch(branch), config),
        }
    }
}

pub fn run(args: InitArgs) -> Result<()> {
    let root = support::existing_dir(&args.project_root, "--project-root")?;
    let workflow = resolve(&root, &args.github_workflow_path);
    let config = select_config(&root, args.config.as_deref());
    let config_reference = project_reference(&root, &config);
    let launcher_kind = LauncherKind::current();
    let script_argument = args
        .script_path
        .as_deref()
        .unwrap_or_else(|| launcher_kind.default_path());
    let script = resolve(&root, script_argument);

    if workflow == script || workflow == config || script == config {
        bail!(
            "generated workflow, launcher, and config paths must be distinct (workflow {}, launcher {}, config {})",
            workflow.display(),
            script.display(),
            config.display()
        );
    }
    for path in [&workflow, &script] {
        ensure_replaceable(path, args.force)?;
    }

    write(
        &workflow,
        &render_github_workflow(GithubWorkflowOptions {
            version: env!("CARGO_PKG_VERSION"),
            config_path: &config_reference.to_string_lossy(),
        }),
    )?;
    if !config.exists() {
        write(&config, &default_config(current_platform()?))?;
    }
    write(
        &script,
        &launcher_kind.render(&args.branch, &config_reference),
    )?;
    if matches!(launcher_kind, LauncherKind::Posix) {
        support::make_executable(&script)?;
    }

    println!(
        "generated GitHub sandbox workflow at {}",
        workflow.display()
    );
    println!("using sandbox config at {}", config.display());
    println!("generated offline launcher at {}", script.display());
    Ok(())
}

fn resolve(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn select_config(root: &Path, explicit: Option<&Path>) -> PathBuf {
    if let Some(path) = explicit {
        return resolve(root, path);
    }
    let preferred = root.join(PREFERRED_CONFIG);
    let legacy = root.join(LEGACY_CONFIG);
    if preferred.exists() || !legacy.exists() {
        preferred
    } else {
        legacy
    }
}

fn project_reference(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root)
        .map_or_else(|_| path.to_path_buf(), Path::to_path_buf)
}

fn ensure_replaceable(path: &Path, force: bool) -> Result<()> {
    if !path.exists() || force {
        return Ok(());
    }
    let generated = fs::read_to_string(path)
        .map(|text| {
            text.lines()
                .take(3)
                .any(|line| line.contains(GENERATED_MARKER))
        })
        .unwrap_or(false);
    if generated {
        return Ok(());
    }
    bail!(
        "{} already exists and is not owned by pixi-sandbox — pass --force to replace it",
        path.display()
    )
}

fn write(path: &Path, content: &str) -> Result<()> {
    let parent = path.parent().expect("generated file always has a parent");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::write(path, content).with_context(|| format!("writing {}", path.display()))
}

fn current_platform() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("linux", "x86_64") => Ok("linux-64"),
        ("linux", "aarch64") => Ok("linux-aarch64"),
        ("macos", "x86_64") => Ok("osx-64"),
        ("macos", "aarch64") => Ok("osx-arm64"),
        ("windows", "x86_64") => Ok("win-64"),
        (os, arch) => bail!("init does not support host platform {os}-{arch}"),
    }
}

fn default_config(platform: &str) -> String {
    format!(
        "schema = 1\nbranch_prefix = \"sandbox\"\ncargo_vendor = true\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"{platform}\"]\n"
    )
}

fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_./-".contains(&byte))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\"'\"'"))
    }
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn posix_config(config: &Path) -> String {
    let quoted = shell_quote(&config.to_string_lossy());
    if config.is_absolute() {
        quoted
    } else {
        format!("$ROOT/{quoted}")
    }
}

fn powershell_config(config: &Path) -> String {
    let quoted = powershell_quote(&config.to_string_lossy());
    if config.is_absolute() {
        quoted
    } else {
        format!("Join-Path $Root {quoted}")
    }
}

fn posix_restore(branch: &str, config: &Path) -> String {
    // The launcher stays a bootstrap: POSIX sh, no downloads, and no dependency on the tool it
    // is about to unpack. Branch identity comes from the same selected config as the publisher,
    // with the branch reviewed at init time as a fallback when the config cannot decide.
    let template = r#"#!/bin/sh
# __GENERATED_MARKER__
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
CONFIG=__CONFIG__
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
    template
        .replace("__GENERATED_MARKER__", GENERATED_MARKER)
        .replace("__BRANCH__", branch)
        .replace("__CONFIG__", &posix_config(config))
}

fn windows_branch(branch: &str) -> String {
    branch
        .strip_suffix("linux-64")
        .map_or_else(|| branch.to_string(), |prefix| format!("{prefix}win-64"))
}

fn powershell_restore(branch: &str, config: &Path) -> String {
    let template = r#"# __GENERATED_MARKER__
$ErrorActionPreference = 'Stop'
$Root = (git rev-parse --show-toplevel).Trim()
$DefaultBranch = '__BRANCH__'
$Branch = $env:PIXI_SANDBOX_BRANCH
$Config = __CONFIG__
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
    template
        .replace("__GENERATED_MARKER__", GENERATED_MARKER)
        .replace("__BRANCH__", branch)
        .replace("__CONFIG__", &powershell_config(config))
}

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_CONFIG, PREFERRED_CONFIG, posix_restore, powershell_restore, select_config,
        windows_branch,
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn config_resolution_prefers_explicit_then_new_then_legacy() {
        let root = tempfile::tempdir().unwrap();
        let legacy = root.path().join(LEGACY_CONFIG);
        fs::write(&legacy, "legacy").unwrap();
        assert_eq!(select_config(root.path(), None), legacy);

        let preferred = root.path().join(PREFERRED_CONFIG);
        fs::write(&preferred, "preferred").unwrap();
        assert_eq!(select_config(root.path(), None), preferred);

        assert_eq!(
            select_config(root.path(), Some(Path::new("config/custom.toml"))),
            root.path().join("config/custom.toml")
        );
    }

    #[test]
    fn launchers_are_offline_bootstraps_not_installers() {
        let shell = posix_restore("sandbox/developer-linux-64", Path::new(PREFERRED_CONFIG));
        assert!(shell.contains("git -C \"$ROOT\" archive"));
        assert!(!shell.contains("curl"));
        let powershell =
            powershell_restore("sandbox/developer-win-64", Path::new(PREFERRED_CONFIG));
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
    fn launchers_read_the_branch_off_the_selected_plan() {
        let shell = posix_restore(
            "sandbox/developer-linux-64",
            Path::new("config/sandbox plan.toml"),
        );
        assert!(shell.contains("CONFIG=$ROOT/'config/sandbox plan.toml'"));
        assert!(shell.contains("branch_prefix"));
        assert!(shell.contains("BRANCH=${PREFIX:-sandbox}/$BUNDLES-$PLATFORM"));
        assert!(shell.contains("BRANCH=${BRANCH:-$DEFAULT_BRANCH}"));
        assert!(shell.contains("DEFAULT_BRANCH=sandbox/developer-linux-64"));

        let powershell = powershell_restore(
            "sandbox/developer-win-64",
            Path::new("config/sandbox plan.toml"),
        );
        assert!(powershell.contains("Join-Path $Root 'config/sandbox plan.toml'"));
        assert!(powershell.contains("branch_prefix"));
        assert!(powershell.contains("$Branch = \"$Prefix/$($Bundles[0])-win-64\""));
        assert!(powershell.contains("if (-not $Branch) { $Branch = $DefaultBranch }"));
        assert!(powershell.contains("$DefaultBranch = 'sandbox/developer-win-64'"));
    }

    #[test]
    fn an_explicit_branch_still_wins_over_the_config() {
        let shell = posix_restore("sandbox/developer-linux-64", Path::new(PREFERRED_CONFIG));
        let override_at = shell.find("BRANCH=${PIXI_SANDBOX_BRANCH:-}").unwrap();
        let derive_at = shell.find("CONFIG=$ROOT/pixi-sandbox.toml").unwrap();
        assert!(override_at < derive_at);
        assert!(shell.contains("if [ -z \"$BRANCH\" ] && [ -r \"$CONFIG\" ]; then"));
    }
}
