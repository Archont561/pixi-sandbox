//! Generate the connected-side project files that publish and consume an offline sandbox.

use crate::cli::InitArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::generated::{
    GENERATED_MARKER, GithubWorkflowOptions, RelockWorkflowOptions, embedded_pixi_pin,
    plan_vendors_cargo, render_github_workflow, render_relock_workflow,
};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, Item, RawString, Value};

const PREFERRED_CONFIG: &str = "pixi-sandbox.toml";
const LEGACY_CONFIG: &str = ".pixi-sandbox.toml";
const ARCHONT561_CHANNEL: &str = "https://prefix.dev/archont561";

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
    let relock = resolve(&root, &args.relock_workflow_path);
    let config = select_config(&root, args.config.as_deref());
    let config_reference = project_reference(&root, &config);
    let launcher_kind = LauncherKind::current();
    let script_argument = args
        .script_path
        .as_deref()
        .unwrap_or_else(|| launcher_kind.default_path());
    let script = resolve(&root, script_argument);

    let generated = [&workflow, &relock, &script, &config];
    if generated
        .iter()
        .enumerate()
        .any(|(index, path)| generated[index + 1..].contains(path))
    {
        bail!(
            "generated workflow, relock workflow, launcher, and config paths must be distinct (workflow {}, relock {}, launcher {}, config {})",
            workflow.display(),
            relock.display(),
            script.display(),
            config.display()
        );
    }
    for path in [&workflow, &relock, &script] {
        ensure_replaceable(path, args.force)?;
    }

    ensure_archont561_channel(&root)?;

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
        &relock,
        &render_relock_workflow(RelockWorkflowOptions {
            pixi_version: &embedded_pixi_pin()
                .context("the embedded tools lock declares no pixi pin")?,
            cargo: plan_vendors_cargo(&config),
            ci_workflow: &args.relock_ci_workflow,
        }),
    )?;
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
    println!(
        "generated lockfile-refresh workflow at {}",
        relock.display()
    );
    println!("using sandbox config at {}", config.display());
    println!("generated offline launcher at {}", script.display());
    Ok(())
}

fn normalized_channel(value: &str) -> String {
    value.trim().trim_end_matches('/').to_ascii_lowercase()
}

/// Add the publisher namespace by mutating only the channel array. `DocumentMut` preserves
/// comments and formatting in every untouched part of a consumer-maintained manifest.
fn ensure_archont561_channel(root: &Path) -> Result<()> {
    let path = root.join("pixi.toml");
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "reading project Pixi configuration {}; run init from a Pixi project",
            path.display()
        )
    })?;
    let mut manifest: DocumentMut = text.parse().with_context(|| {
        format!(
            "parsing project Pixi configuration {}; fix the TOML before running init",
            path.display()
        )
    })?;
    let workspace = manifest
        .get_mut("workspace")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{} has no [workspace] table; cannot safely configure the Archont561 channel",
                path.display()
            )
        })?;
    if workspace.get("channels").is_none() {
        workspace.insert("channels", Item::Value(Value::Array(Array::new())));
    }
    let channels = workspace
        .get_mut("channels")
        .and_then(Item::as_array_mut)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "{}.workspace.channels must be an array of channel URLs",
                path.display()
            )
        })?;
    let canonical = normalized_channel(ARCHONT561_CHANNEL);
    for channel in channels.iter() {
        let value = channel.as_str().ok_or_else(|| {
            anyhow::anyhow!(
                "{}.workspace.channels contains a non-string entry; cannot safely update it",
                path.display()
            )
        })?;
        if normalized_channel(value) == canonical {
            return Ok(());
        }
    }
    append_channel(channels, &text);
    write(&path, &manifest.to_string())
        .with_context(|| format!("updating project Pixi configuration {}", path.display()))
}

/// Append to multiline arrays before their preserved closing whitespace. A trailing comment after
/// the old final comma belongs to that entry, so it must move ahead of the new value rather than
/// following it.
fn append_channel(channels: &mut Array, source: &str) {
    let trailing = source_text(source, channels.trailing()).unwrap_or_default();
    let last_index = channels.len().checked_sub(1);
    let last_suffix = last_index
        .and_then(|index| channels.get(index))
        .and_then(|channel| channel.decor().suffix())
        .and_then(|suffix| source_text(source, suffix))
        .unwrap_or_default();
    let (decoration, clear_last_suffix) = if trailing.contains('\n') {
        (trailing.as_str(), false)
    } else if last_suffix.contains('\n') {
        (last_suffix.as_str(), true)
    } else {
        channels.push(ARCHONT561_CHANNEL);
        return;
    };
    let Some((before_closing, closing)) = decoration.rsplit_once('\n') else {
        channels.push(ARCHONT561_CHANNEL);
        return;
    };
    let Some(indent) = channels
        .iter()
        .last()
        .and_then(|channel| channel.decor().prefix())
        .and_then(|prefix| source_text(source, prefix))
    else {
        channels.push(ARCHONT561_CHANNEL);
        return;
    };
    // A value prefix may also carry preceding standalone comments. Only its final newline and
    // indentation belong to the new value; the comments remain attached to their old entry.
    let indent = if let Some((_, whitespace)) = indent.rsplit_once('\n') {
        format!("\n{whitespace}")
    } else {
        indent
    };
    if clear_last_suffix {
        channels
            .get_mut(last_index.expect("a channel suffix requires a channel"))
            .expect("a channel suffix has a matching channel")
            .decor_mut()
            .set_suffix("");
    }

    let mut namespace = Value::from(ARCHONT561_CHANNEL);
    namespace
        .decor_mut()
        .set_prefix(format!("{before_closing}{indent}"));
    let remaining_trailing = if clear_last_suffix {
        trailing.as_str()
    } else {
        ""
    };
    channels.set_trailing(format!("\n{closing}{remaining_trailing}"));
    channels.push_formatted(namespace);
}

fn source_text(source: &str, raw: &RawString) -> Option<String> {
    raw.as_str().map(str::to_owned).or_else(|| {
        raw.span()
            .and_then(|span| source.get(span))
            .map(str::to_owned)
    })
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
    // The launcher stays a bootstrap: POSIX sh and no dependency on the tool it is about to
    // unpack. Branch identity comes from the same selected config as the publisher, with the
    // branch reviewed at init time as a fallback when the config cannot decide. If a connected
    // clone lacks the sandbox refs, it fetches only that namespace; airlocks can opt out with
    // PIXI_SANDBOX_FETCH=skip and rely on the branch already being present.
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
REF=$BRANCH
if ! git -C "$ROOT" rev-parse --verify "$REF^{commit}" >/dev/null 2>&1; then REF=origin/$BRANCH; fi
if ! git -C "$ROOT" rev-parse --verify "$REF^{commit}" >/dev/null 2>&1; then
  if [ "${PIXI_SANDBOX_FETCH:-auto}" = skip ]; then
    echo "sandbox branch $BRANCH is not present locally (PIXI_SANDBOX_FETCH=skip); fetch refs/heads/sandbox/* before restoring" >&2
    exit 2
  fi
  git -C "$ROOT" fetch --depth 1 origin "refs/heads/$BRANCH:refs/remotes/origin/$BRANCH" || {
    echo "cannot fetch $BRANCH from origin; run: git fetch origin 'refs/heads/sandbox/*:refs/remotes/origin/sandbox/*'" >&2
    exit 2
  }
  REF=origin/$BRANCH
fi
TRANSPORT="$ROOT/.pixi/.restore-transport"
rm -rf "$TRANSPORT"
mkdir -p "$TRANSPORT"
git -C "$ROOT" archive "$REF" | tar -x -C "$TRANSPORT"
BIN="$TRANSPORT/.pixi-sandbox/tools/$PLATFORM/pixi-sandbox"
# User-tool registration is selected here, explicitly (task-33): a person restoring an
# airlock gets `pixi` and `pixi sandbox` in a per-user bin by default; a locked-down or
# shared host opts out with PIXI_SANDBOX_USER_TOOLS=skip. The policy travels as an
# environment variable, never a flag: the binary this script executes comes from the
# packed branch, which may predate --user-tools, and an unknown variable is ignored
# where an unknown flag is a hard error. An operator's trailing --user-tools skip still
# wins — later arguments override the environment.
PIXI_SANDBOX_USER_TOOLS="${PIXI_SANDBOX_USER_TOOLS:-register}"
export PIXI_SANDBOX_USER_TOOLS
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
$Ref = $Branch
git -C $Root rev-parse --verify "$Ref^{commit}" 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { $Ref = "origin/$Branch" }
git -C $Root rev-parse --verify "$Ref^{commit}" 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    if ($env:PIXI_SANDBOX_FETCH -eq 'skip') {
        Write-Error -Message "sandbox branch $Branch is not present locally (PIXI_SANDBOX_FETCH=skip); fetch refs/heads/sandbox/* before restoring" -ErrorAction Continue
        exit 2
    }
    git -C $Root fetch --depth 1 origin "refs/heads/${Branch}:refs/remotes/origin/${Branch}"
    if ($LASTEXITCODE -ne 0) {
        Write-Error -Message "cannot fetch $Branch from origin; run: git fetch origin 'refs/heads/sandbox/*:refs/remotes/origin/sandbox/*'" -ErrorAction Continue
        exit 2
    }
    $Ref = "origin/$Branch"
}
$Transport = Join-Path $Root '.pixi/.restore-transport'
$Archive = Join-Path $Root '.pixi/.restore-transport.tar'
Remove-Item -Recurse -Force $Transport,$Archive -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force $Transport | Out-Null
git -C $Root archive --format=tar --output=$Archive $Ref
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
tar -xf $Archive -C $Transport
$Binary = Join-Path $Transport '.pixi-sandbox/tools/win-64/pixi-sandbox.exe'
# User-tool registration is selected here, explicitly (task-33): a person restoring an
# airlock gets `pixi` and `pixi sandbox` on the user PATH by default; a locked-down or
# shared host opts out with PIXI_SANDBOX_USER_TOOLS=skip. The policy travels as an
# environment variable, never a flag: the binary this script executes comes from the
# packed branch, which may predate --user-tools, and an unknown variable is ignored
# where an unknown flag is a hard error. An operator's trailing --user-tools skip still
# wins — later arguments override the environment.
$env:PIXI_SANDBOX_USER_TOOLS = if ($env:PIXI_SANDBOX_USER_TOOLS) { $env:PIXI_SANDBOX_USER_TOOLS } else { 'register' }
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
    fn launchers_are_bootstraps_not_installers() {
        let shell = posix_restore("sandbox/developer-linux-64", Path::new(PREFERRED_CONFIG));
        assert!(shell.contains("git -C \"$ROOT\" archive \"$REF\""));
        assert!(shell.contains("PIXI_SANDBOX_FETCH:-auto"));
        assert!(shell.contains("refs/heads/$BRANCH:refs/remotes/origin/$BRANCH"));
        assert!(shell.contains("fetch refs/heads/sandbox/* before restoring"));
        assert!(!shell.contains("curl"));
        // task-33: the launcher selects the user-tool policy explicitly, and the operator
        // can still override it — env var through the shell default, or a trailing
        // argument, which the CLI lets win. The policy travels as an environment variable,
        // never a flag: the binary the launcher executes comes from the packed branch and
        // may predate --user-tools (the airlock job proved the skew the hard way), while an
        // unknown variable is simply ignored.
        assert!(shell.contains("PIXI_SANDBOX_USER_TOOLS=\"${PIXI_SANDBOX_USER_TOOLS:-register}\""));
        assert!(shell.contains("exec \"$BIN\" restore --branch-location \"$TRANSPORT\" --output-path \"$ROOT\" --force \"$@\""));
        assert!(!shell.contains("--user-tools \""));
        let powershell =
            powershell_restore("sandbox/developer-win-64", Path::new(PREFERRED_CONFIG));
        assert!(powershell.contains("git -C $Root archive --format=tar --output=$Archive $Ref"));
        assert!(powershell.contains("$env:PIXI_SANDBOX_FETCH -eq 'skip'"));
        assert!(powershell.contains("refs/heads/${Branch}:refs/remotes/origin/${Branch}"));
        assert!(!powershell.contains("Invoke-WebRequest"));
        assert!(
            powershell.contains("$env:PIXI_SANDBOX_USER_TOOLS = if ($env:PIXI_SANDBOX_USER_TOOLS)")
        );
        assert!(powershell.contains(
            "& $Binary restore --branch-location $Transport --output-path $Root --force @args"
        ));
        assert!(!powershell.contains("--user-tools $"));
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
