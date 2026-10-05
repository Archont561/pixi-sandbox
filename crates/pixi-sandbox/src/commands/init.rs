//! Generate the connected-side project files that publish and consume an offline sandbox.

use crate::cli::InitArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::generated::{
    GENERATED_MARKER, GithubWorkflowOptions, MARKER_WINDOW, RelockWorkflowOptions,
    embedded_pixi_pin, plan_vendors_cargo, render_github_workflow, render_relock_workflow,
    version_stamp_line,
};
use pixi_sandbox_core::sandbox_config::{
    CONFIG_SCHEMA, SandboxConfig, peek_schema, schema_supported,
};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};

const PREFERRED_CONFIG: &str = "pixi-sandbox.toml";
const LEGACY_CONFIG: &str = ".pixi-sandbox.toml";
const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

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

    fn render(self, version: &str, branch: &str, config: &Path) -> String {
        match self {
            Self::Posix => posix_restore(version, branch, config),
            Self::PowerShell => powershell_restore(version, &windows_branch(branch), config),
        }
    }
}

/// The three marker-carrying files `init` owns and `init --check` audits. The sandbox config is
/// deliberately not one of these: it is reviewed data `init` only ever seeds when absent, never
/// regenerates, so "drift" is not a concept that applies to it the way it does to these three
/// (AC#5 / D16) — its own check is [`config_schema_finding`].
struct Targets {
    workflow: (PathBuf, String),
    relock: (PathBuf, String),
    script: (PathBuf, String),
    config: PathBuf,
}

impl Targets {
    /// Every `(label, path, fresh render)` triple `init --check` compares against disk, in the
    /// order `init` itself writes them.
    fn owned(&self) -> [(&'static str, &Path, &str); 3] {
        [
            (
                "GitHub workflow",
                &self.workflow.0,
                self.workflow.1.as_str(),
            ),
            ("relock workflow", &self.relock.0, self.relock.1.as_str()),
            ("launcher", &self.script.0, self.script.1.as_str()),
        ]
    }
}

/// The generated publisher's `[workflow]`-table-derived CI policy (issue #79, task-53), already
/// resolved into exactly what the renderer needs — never a loaded [`SandboxConfig`] directly, so
/// the renderer stays config-shape-agnostic and testable with plain values. A missing or
/// unreadable config (first `init` on a fresh project, or a config this build cannot parse)
/// degrades to every default — the same graceful-degradation precedent `plan_vendors_cargo`
/// already uses — which is also exactly the pre-task-53 render (task-53 AC#1).
struct ResolvedWorkflowPolicy {
    push_paths: Vec<String>,
    scoped_permissions: bool,
    concurrency: Option<(String, bool)>,
    plan_timeout_minutes: Option<u32>,
    publish_timeout_minutes: Option<u32>,
    pixi_version: Option<String>,
    setup_pixi_cache: Option<bool>,
}

impl ResolvedWorkflowPolicy {
    fn resolve(config: &Path, repo_root: &Path, config_display: &str) -> Self {
        let loaded = SandboxConfig::load(config).ok();
        let push_paths = loaded
            .as_ref()
            .map(|c| c.resolved_push_paths(repo_root, config_display))
            .unwrap_or_default();
        let policy = loaded.as_ref().and_then(|c| c.workflow.as_ref());
        Self {
            push_paths,
            scoped_permissions: policy.is_some_and(|p| p.permissions),
            concurrency: policy
                .and_then(|p| p.concurrency.as_ref())
                .map(|c| (c.group.clone(), c.cancel_in_progress)),
            plan_timeout_minutes: policy
                .and_then(|p| p.timeouts.as_ref())
                .and_then(|t| t.plan),
            publish_timeout_minutes: policy
                .and_then(|p| p.timeouts.as_ref())
                .and_then(|t| t.publish),
            pixi_version: policy.and_then(|p| p.pixi_version.clone()),
            // Once `[workflow]` is present at all, cache defaults to off (decision D18) unless
            // explicitly re-enabled; absent the table entirely, `None` renders no `cache:` key.
            setup_pixi_cache: policy.map(|p| p.setup_pixi_cache.unwrap_or(false)),
        }
    }
}

/// Resolve every path `init` touches and render fresh content for everything it owns outright.
/// Shared by the writing path and `--check` so a render can never drift between the two: the
/// comparison `--check` makes is only meaningful against the exact bytes `init` would write.
fn render_targets(root: &Path, args: &InitArgs) -> Result<Targets> {
    let workflow = resolve(root, &args.github_workflow_path);
    let relock = resolve(root, &args.relock_workflow_path);
    let config = select_config(root, args.config.as_deref());
    let config_reference = project_reference(root, &config);
    let launcher_kind = LauncherKind::current();
    let script_argument = args
        .script_path
        .as_deref()
        .unwrap_or_else(|| launcher_kind.default_path());
    let script = resolve(root, script_argument);

    let paths = [&workflow, &relock, &script, &config];
    if paths
        .iter()
        .enumerate()
        .any(|(index, path)| paths[index + 1..].contains(path))
    {
        bail!(
            "generated workflow, relock workflow, launcher, and config paths must be distinct (workflow {}, relock {}, launcher {}, config {})",
            workflow.display(),
            relock.display(),
            script.display(),
            config.display()
        );
    }

    let workflow_reference = project_reference(root, &workflow);
    let relock_reference = project_reference(root, &relock);
    let script_reference = project_reference(root, &script);
    let workflow_policy =
        ResolvedWorkflowPolicy::resolve(&config, root, &config_reference.to_string_lossy());
    let workflow_content = render_github_workflow(GithubWorkflowOptions {
        version: CLI_VERSION,
        config_path: &config_reference.to_string_lossy(),
        workflow_path: &workflow_reference.to_string_lossy(),
        relock_workflow_path: &relock_reference.to_string_lossy(),
        relock_ci_workflow: &args.relock_ci_workflow,
        script_path: &script_reference.to_string_lossy(),
        branch: &args.branch,
        push_paths: &workflow_policy.push_paths,
        scoped_permissions: workflow_policy.scoped_permissions,
        concurrency: workflow_policy
            .concurrency
            .as_ref()
            .map(|(group, cancel)| (group.as_str(), *cancel)),
        plan_timeout_minutes: workflow_policy.plan_timeout_minutes,
        publish_timeout_minutes: workflow_policy.publish_timeout_minutes,
        pixi_version: workflow_policy.pixi_version.as_deref(),
        setup_pixi_cache: workflow_policy.setup_pixi_cache,
    });
    let relock_content = render_relock_workflow(RelockWorkflowOptions {
        cli_version: CLI_VERSION,
        pixi_version: &embedded_pixi_pin()
            .context("the embedded tools lock declares no pixi pin")?,
        cargo: plan_vendors_cargo(&config),
        relock_workflow: &relock_reference.to_string_lossy(),
        ci_workflow: &args.relock_ci_workflow,
        // The relock bot's third dispatch has to name the publisher this run is generating,
        // so it is the file name of the path `init` was handed — never a literal, or a renamed
        // `--workflow-path` leaves the bot dispatching a workflow that does not exist.
        publisher_workflow: &workflow
            .file_name()
            .context("publisher workflow path has no file name")?
            .to_string_lossy(),
    });
    let script_content = launcher_kind.render(CLI_VERSION, &args.branch, &config_reference);

    Ok(Targets {
        workflow: (workflow, workflow_content),
        relock: (relock, relock_content),
        script: (script, script_content),
        config,
    })
}

pub fn run(args: InitArgs) -> Result<()> {
    let root = support::existing_dir(&args.project_root, "--project-root")?;
    if args.check {
        return check(&root, &args);
    }
    ensure_pixi_project(&root)?;
    let targets = render_targets(&root, &args)?;
    for (path, _) in [&targets.workflow, &targets.relock, &targets.script] {
        ensure_replaceable(path, args.force)?;
    }

    let launcher_kind = LauncherKind::current();
    write(&targets.workflow.0, &targets.workflow.1)?;
    if !targets.config.exists() {
        write(&targets.config, &default_config(current_platform()?))?;
    }
    write(&targets.relock.0, &targets.relock.1)?;
    write(&targets.script.0, &targets.script.1)?;
    if matches!(launcher_kind, LauncherKind::Posix) {
        support::make_executable(&targets.script.0)?;
    }

    println!(
        "generated GitHub sandbox workflow at {}",
        targets.workflow.0.display()
    );
    println!(
        "generated lockfile-refresh workflow at {}",
        targets.relock.0.display()
    );
    println!("using sandbox config at {}", targets.config.display());
    println!(
        "generated offline launcher at {}",
        targets.script.0.display()
    );
    Ok(())
}

/// Whether `content`'s leading [`MARKER_WINDOW`] lines carry the ownership marker — the one
/// signal that decides whether a file is pixi-sandbox's to replace (`ensure_replaceable`) or to
/// report as drifted rather than foreign (`classify`). Kept as the single implementation both
/// read, so the two can never disagree about what "owned" means.
fn is_generated(content: &str) -> bool {
    content
        .lines()
        .take(MARKER_WINDOW)
        .any(|line| line.contains(GENERATED_MARKER))
}

/// How one owned file on disk compares to the render `init` would write for it right now.
#[derive(Debug, PartialEq, Eq)]
enum Drift {
    /// Does not exist yet; `init` would create it.
    Missing,
    /// Exists, carries no ownership marker, and is therefore not `init`'s to touch without
    /// `--force` — reported separately from drift because the remedy is different (AC#4).
    Foreign,
    /// Exists, is owned, and no longer matches a fresh render.
    Stale,
    /// Exists, is owned, and matches a fresh render exactly.
    Current,
}

fn classify(path: &Path, fresh: &str) -> Result<Drift> {
    if !path.exists() {
        return Ok(Drift::Missing);
    }
    let current =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    if !is_generated(&current) {
        return Ok(Drift::Foreign);
    }
    Ok(if current == fresh {
        Drift::Current
    } else {
        Drift::Stale
    })
}

/// Render fresh, write nothing, and name every finding (task-47 AC#4 / decision D16): this is
/// `init`'s drift gate, run by the generated workflow's upgrade job (AC#6) ahead of regenerating
/// anything, and usable by a consumer directly to audit a tree before deciding to run `init`.
fn check(root: &Path, args: &InitArgs) -> Result<()> {
    ensure_pixi_project(root)?;
    let targets = render_targets(root, args)?;

    let mut findings = Vec::new();
    for (label, path, fresh) in targets.owned() {
        match classify(path, fresh)? {
            Drift::Missing => findings.push(format!(
                "{label} {} is missing; run `pixi-sandbox init` to generate it",
                path.display()
            )),
            Drift::Foreign => findings.push(format!(
                "{label} {} exists and is not owned by pixi-sandbox; run `pixi-sandbox init --force` to replace it",
                path.display()
            )),
            Drift::Stale => findings.push(format!(
                "{label} {} no longer matches a fresh render; run `pixi-sandbox init` to regenerate it",
                path.display()
            )),
            Drift::Current => {}
        }
    }

    if targets.config.exists() {
        if let Some(finding) = config_schema_finding(&targets.config)? {
            findings.push(finding);
        }
    } else {
        findings.push(format!(
            "sandbox config {} is missing; run `pixi-sandbox init` to generate it",
            targets.config.display()
        ));
    }

    for finding in &findings {
        println!("{finding}");
    }
    if findings.is_empty() {
        println!("every owned file matches a fresh render; sandbox config schema is supported");
        return Ok(());
    }
    // Non-zero is the point: the generated upgrade job and any human audit key on this.
    bail!("{} finding(s); see above", findings.len())
}

/// Config is reviewed data `init` only ever seeds when absent (D17, AC#5): a schema mismatch is
/// reported as a named finding rather than surfacing `SandboxConfig::load`'s validation error
/// verbatim, because the right remedy differs by direction and `--check` must say so without
/// requiring the rest of the file to parse under today's exact field set.
fn config_schema_finding(config: &Path) -> Result<Option<String>> {
    let schema = peek_schema(config).with_context(|| {
        format!(
            "{} does not declare a schema; it is not a recognisable sandbox config",
            config.display()
        )
    })?;
    if schema_supported(schema) {
        return Ok(None);
    }
    Ok(Some(match schema.cmp(&CONFIG_SCHEMA) {
        Ordering::Greater => format!(
            "sandbox config {} declares schema {schema}, newer than this CLI's {CONFIG_SCHEMA} \
             — upgrade pixi-sandbox before running init or plan against it",
            config.display()
        ),
        _ => format!(
            "sandbox config {} declares schema {schema}, which this CLI has never understood \
             (supports 1..={CONFIG_SCHEMA}) — it is not a config this or any pixi-sandbox release \
             can read",
            config.display()
        ),
    }))
}

/// Init writes only files it owns (D17): the publisher workflow, the relock workflow, the
/// launcher, and the config when absent. The consumer's `pixi.toml` is never read beyond
/// this existence check — task-34's channel append is gone because the publisher namespace
/// root it added serves no repodata, so a consumer with any dependency failed `pixi lock`
/// right after init (issue #71). The guard stays: scaffolding a sandbox whose `default`
/// environment cannot exist is a mistake worth naming, not generating around.
fn ensure_pixi_project(root: &Path) -> Result<()> {
    let path = root.join("pixi.toml");
    if path.is_file() {
        return Ok(());
    }
    bail!(
        "no Pixi manifest at {}; run init from a Pixi project",
        path.display()
    )
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
        .map(|text| is_generated(&text))
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

/// Thin wrapper over [`pixi_sandbox_core::platform::Platform`], the single source of this
/// mapping (task-55); kept as its own function for the error message `init`'s callers expect.
fn current_platform() -> Result<&'static str> {
    pixi_sandbox_core::platform::Platform::current()
        .map(|platform| platform.as_str())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "init does not support host platform {}-{}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )
        })
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

fn posix_restore(version: &str, branch: &str, config: &Path) -> String {
    // The launcher stays a bootstrap: POSIX sh and no dependency on the tool it is about to
    // unpack. Branch identity comes from the same selected config as the publisher, with the
    // branch reviewed at init time as a fallback when the config cannot decide. If a connected
    // clone lacks the sandbox refs, it fetches only that namespace; airlocks can opt out with
    // PIXI_SANDBOX_FETCH=skip and rely on the branch already being present.
    let template = r#"#!/bin/sh
# __GENERATED_MARKER__
# __VERSION_STAMP__
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
        .replace("__VERSION_STAMP__", &version_stamp_line(version))
        .replace("__BRANCH__", branch)
        .replace("__CONFIG__", &posix_config(config))
}

fn windows_branch(branch: &str) -> String {
    branch
        .strip_suffix("linux-64")
        .map_or_else(|| branch.to_string(), |prefix| format!("{prefix}win-64"))
}

fn powershell_restore(version: &str, branch: &str, config: &Path) -> String {
    let template = r#"# __GENERATED_MARKER__
# __VERSION_STAMP__
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
        .replace("__VERSION_STAMP__", &version_stamp_line(version))
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
        let shell = posix_restore(
            "1.2.3",
            "sandbox/developer-linux-64",
            Path::new(PREFERRED_CONFIG),
        );
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
        let powershell = powershell_restore(
            "1.2.3",
            "sandbox/developer-win-64",
            Path::new(PREFERRED_CONFIG),
        );
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

    /// task-47 AC#3: both launchers carry a parseable version stamp beside the ownership
    /// marker, in the same window `ensure_replaceable` and `is_generated` already read.
    #[test]
    fn launchers_carry_a_parseable_version_stamp() {
        let shell = posix_restore(
            "7.8.9",
            "sandbox/developer-linux-64",
            Path::new(PREFERRED_CONFIG),
        );
        assert_eq!(
            pixi_sandbox::generated::parse_version_stamp(&shell),
            Some("7.8.9")
        );
        let powershell = powershell_restore(
            "7.8.9",
            "sandbox/developer-win-64",
            Path::new(PREFERRED_CONFIG),
        );
        assert_eq!(
            pixi_sandbox::generated::parse_version_stamp(&powershell),
            Some("7.8.9")
        );
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
            "1.2.3",
            "sandbox/developer-linux-64",
            Path::new("config/sandbox plan.toml"),
        );
        assert!(shell.contains("CONFIG=$ROOT/'config/sandbox plan.toml'"));
        assert!(shell.contains("branch_prefix"));
        assert!(shell.contains("BRANCH=${PREFIX:-sandbox}/$BUNDLES-$PLATFORM"));
        assert!(shell.contains("BRANCH=${BRANCH:-$DEFAULT_BRANCH}"));
        assert!(shell.contains("DEFAULT_BRANCH=sandbox/developer-linux-64"));

        let powershell = powershell_restore(
            "1.2.3",
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
        let shell = posix_restore(
            "1.2.3",
            "sandbox/developer-linux-64",
            Path::new(PREFERRED_CONFIG),
        );
        let override_at = shell.find("BRANCH=${PIXI_SANDBOX_BRANCH:-}").unwrap();
        let derive_at = shell.find("CONFIG=$ROOT/pixi-sandbox.toml").unwrap();
        assert!(override_at < derive_at);
        assert!(shell.contains("if [ -z \"$BRANCH\" ] && [ -r \"$CONFIG\" ]; then"));
    }

    /// task-47 AC#3/AC#4/AC#5 — `init --check` is the drift gate: render fresh, compare, write
    /// nothing, and exit non-zero naming each finding. These exercise `run()` itself (both
    /// modes), not just the pure renderers above, because the whole point of `--check` is that
    /// it sees exactly what `init` would have written.
    mod check {
        use super::super::{Drift, classify, config_schema_finding, run};
        use crate::cli::InitArgs;
        use std::fs;
        use std::path::{Path, PathBuf};

        fn project() -> tempfile::TempDir {
            let dir = tempfile::tempdir().unwrap();
            fs::write(dir.path().join("pixi.toml"), "[workspace]\nchannels = []\n").unwrap();
            dir
        }

        fn args(root: &Path) -> InitArgs {
            InitArgs {
                project_root: root.to_path_buf(),
                github_workflow_path: PathBuf::from(".github/workflows/publish-sandbox.yml"),
                relock_workflow_path: PathBuf::from(".github/workflows/relock.yml"),
                relock_ci_workflow: "ci.yml".to_string(),
                script_path: None,
                config: None,
                branch: "sandbox/developer-linux-64".to_string(),
                force: false,
                check: false,
            }
        }

        fn checked(root: &Path) -> InitArgs {
            InitArgs {
                check: true,
                ..args(root)
            }
        }

        /// The exact behaviour the generated upgrade job (AC#6) depends on: a tree `init` just
        /// wrote passes `--check` cleanly, and nothing on disk changes because `--check` ran.
        #[test]
        fn a_freshly_initialised_tree_passes_check_and_check_writes_nothing() {
            let project = project();
            run(args(project.path())).unwrap();

            let before =
                fs::read_to_string(project.path().join(".github/workflows/publish-sandbox.yml"))
                    .unwrap();
            run(checked(project.path())).unwrap();
            let after =
                fs::read_to_string(project.path().join(".github/workflows/publish-sandbox.yml"))
                    .unwrap();
            assert_eq!(before, after, "--check must never write");
        }

        /// A hand-edited (or stale) owned file is reported as drifted, named with its remedy,
        /// and `--check` still writes nothing even though it found a problem.
        #[test]
        fn a_hand_edited_owned_file_is_reported_as_drifted_not_silently_fixed() {
            let project = project();
            run(args(project.path())).unwrap();
            let workflow = project.path().join(".github/workflows/publish-sandbox.yml");
            let marked = fs::read_to_string(&workflow).unwrap();
            fs::write(&workflow, format!("{marked}# a stale local edit\n")).unwrap();

            let error = run(checked(project.path())).unwrap_err().to_string();
            assert!(error.contains("finding"), "{error}");

            let untouched = fs::read_to_string(&workflow).unwrap();
            assert!(
                untouched.ends_with("# a stale local edit\n"),
                "--check must not rewrite it"
            );
        }

        /// A file that exists but carries no ownership marker is foreign, not drifted: the
        /// remedy is `--force`, never a plain `init` that would otherwise silently clobber a
        /// file this CLI does not own.
        #[test]
        fn a_foreign_file_is_reported_separately_from_drift() {
            let project = project();
            fs::create_dir_all(project.path().join(".github/workflows")).unwrap();
            let workflow = project.path().join(".github/workflows/publish-sandbox.yml");
            fs::write(&workflow, "name: hand-written\n").unwrap();

            assert_eq!(
                classify(&workflow, "irrelevant fresh render").unwrap(),
                Drift::Foreign
            );
            let error = run(checked(project.path())).unwrap_err().to_string();
            assert!(error.contains("finding"), "{error}");
        }

        /// A missing owned file is its own finding, distinct from drift and from foreign
        /// ownership — the remedy (`pixi-sandbox init`) is the same as drift's, but the file
        /// never existed to compare against.
        #[test]
        fn a_missing_owned_file_is_its_own_finding() {
            let project = project();
            let error = run(checked(project.path())).unwrap_err().to_string();
            assert!(error.contains("finding"), "{error}");
        }

        /// Config is reviewed data: `init` never rewrites it once it exists, in either mode.
        #[test]
        fn init_never_rewrites_an_existing_config_in_either_mode() {
            let project = project();
            run(args(project.path())).unwrap();
            let config = project.path().join("pixi-sandbox.toml");
            fs::write(&config, "schema = 1\nbranch_prefix = \"custom\"\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n").unwrap();
            let before = fs::read_to_string(&config).unwrap();

            run(args(project.path())).unwrap();
            assert_eq!(
                fs::read_to_string(&config).unwrap(),
                before,
                "init must not rewrite it"
            );

            let _ = run(checked(project.path()));
            assert_eq!(
                fs::read_to_string(&config).unwrap(),
                before,
                "--check must not rewrite it"
            );
        }

        /// A config schema newer than this CLI understands is a named finding with its own
        /// remedy, never a generic parse-error crash (AC#5).
        #[test]
        fn a_newer_config_schema_is_a_named_finding() {
            let project = project();
            let config = project.path().join("pixi-sandbox.toml");
            fs::write(
                &config,
                "schema = 2\nbranch_prefix = \"sandbox\"\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n",
            )
            .unwrap();
            let finding = config_schema_finding(&config).unwrap().unwrap();
            assert!(finding.contains("newer than this CLI's"), "{finding}");
        }

        /// The schema this CLI actually ships produces no finding at all.
        #[test]
        fn the_current_schema_produces_no_finding() {
            let project = project();
            let config = project.path().join("pixi-sandbox.toml");
            fs::write(
                &config,
                "schema = 1\nbranch_prefix = \"sandbox\"\n\n[[bundle]]\nname = \"developer\"\nenvironments = [\"default\"]\nplatforms = [\"linux-64\"]\n",
            )
            .unwrap();
            assert_eq!(config_schema_finding(&config).unwrap(), None);
        }
    }
}
