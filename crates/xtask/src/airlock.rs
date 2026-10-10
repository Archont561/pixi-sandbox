//! The airlock proof's machinery (task-36): everything `.github/workflows/airlock.yml` used
//! to do in ~136 lines of per-runner shell, as tested Rust.
//!
//! Five commands, and each replaces shell for a reason:
//! - `airlock-matrix` — the plan binary's JSON has a shape contract (only `.include` is a
//!   matrix) that the shell trusted blindly;
//! - `resolve-release-tag` — 60 lines of override/published/latest/fallback folklore with
//!   its own error handling, every branch of it an incident (rate limits, mid-release 404s,
//!   JSON error bodies captured as tags);
//! - `airlock-pack` — a comma-list install loop plus a conditional `--cargo-vendor` flag,
//!   which a plain task cannot express;
//! - `airlock-fetch` — git, therefore through `pixi-sandbox-git` (D9);
//! - `deny-egress` — the OS case-dance (`unshare -n` / `sandbox-exec`) that must fail
//!   loudly on anything else.
//!
//! Spawned processes are thin adapters over pure, tempdir-tested cores (D10).

use anyhow::{Context, Result, bail};
use pixi_sandbox_core::platform::Platform;
use pixi_sandbox_git::ShellGit;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command as StdCommand, Stdio};
use std::str::FromStr;

/// The sandbox-exec profile that allows everything except outbound network — macOS has no
/// unshare, and this is the equivalent the Tier A step has always used.
#[doc(hidden)] // test boundary (tests/airlock.rs)
pub const DARWIN_EGRESS_PROFILE: &str = "(version 1)(allow default)(deny network-outbound)";

// ---------------------------------------------------------------------------
// airlock-matrix
// ---------------------------------------------------------------------------

/// The argv for the plan binary: override mode when `envs` is non-empty (a manual dispatch
/// names its target), config mode otherwise (the reviewed `.pixi-sandbox.toml`).
#[must_use]
pub fn plan_argv(
    bin: &str,
    config: &str,
    envs: &str,
    platform: &str,
    bundle: &str,
    branch_prefix: &str,
    cargo_vendor: &str,
) -> Vec<String> {
    if envs.is_empty() {
        return [bin, "plan", "--config", config, "--json"]
            .iter()
            .map(|s| (*s).to_string())
            .collect();
    }
    [
        bin,
        "plan",
        "--bundle",
        if bundle.is_empty() { "custom" } else { bundle },
        "--envs",
        envs,
        "--platform",
        platform,
        "--branch-prefix",
        if branch_prefix.is_empty() {
            "sandbox"
        } else {
            branch_prefix
        },
        "--cargo-vendor",
        if cargo_vendor.is_empty() {
            "true"
        } else {
            cargo_vendor
        },
        "--json",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// Validate the plan JSON and return it unchanged: `matrix=` must carry the whole object,
/// because the workflow reads `fromJSON(...).include` — only `.include` is a matrix, and
/// Actions would read every other key (the scalar `schema`) as a dimension that must be an
/// array, silently yielding zero jobs.
///
/// # Errors
///
/// Fails when `json` is not JSON, has no `include` array, or has an empty `include` — an empty matrix would prove nothing on no runner.
pub fn validate_matrix(json: &str) -> Result<String> {
    let parsed: Value = serde_json::from_str(json)
        .with_context(|| format!("the plan output is not JSON: {}", json.trim()))?;
    let include = parsed
        .get("include")
        .context("the plan output has no `include` array")?;
    let empty = include.as_array().is_none_or(Vec::is_empty);
    if empty {
        bail!("the plan produced an empty include — no native runner to prove anything on");
    }
    Ok(json.trim().to_string())
}

/// Build the matrix from the plan binary's stdout and hand it to the workflow.
///
/// # Errors
///
/// Fails when the plan binary cannot be started or exits non-zero, or when its output is rejected by [`validate_matrix`].
pub fn airlock_matrix(
    bin: &str,
    config: &str,
    envs: &str,
    platform: &str,
    bundle: &str,
    branch_prefix: &str,
    cargo_vendor: &str,
) -> Result<()> {
    let argv = plan_argv(
        bin,
        config,
        envs,
        platform,
        bundle,
        branch_prefix,
        cargo_vendor,
    );
    let output = StdCommand::new(&argv[0])
        .args(&argv[1..])
        .output()
        .with_context(|| format!("starting the plan binary {}", argv[0]))?;
    if !output.status.success() {
        bail!(
            "the plan binary failed ({}): {}",
            argv.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let json = validate_matrix(&String::from_utf8_lossy(&output.stdout))?;
    crate::util::github_output("matrix", &json);
    println!("{json}");
    Ok(())
}

// ---------------------------------------------------------------------------
// resolve-release-tag
// ---------------------------------------------------------------------------

/// A `v<digit>…` tag: the resolved value flows into a download URL, so anything not shaped
/// like a tag is refused before it can reach one.
fn shaped_like_tag(tag: &str) -> bool {
    let mut bytes = tag.bytes();
    bytes.next() == Some(b'v') && bytes.next().is_some_and(|b| b.is_ascii_digit())
}

/// Resolve which release the proof should exercise. `published`/`latest` are injected so
/// every branch is testable without the network: they answer with the tag or `None`.
///
/// Order (each branch is an incident, kept from the shell this replaces):
/// 1. an explicit override is a request, not a guess — honour it and let a 404 be a real
///    error (`vars.SANDBOX_RELEASE_VERSION`);
/// 2. the declared version, confirmed against the releases API — `auto-release` pushes the
///    bump to main *before* dispatching the build, so mid-release the checkout declares a
///    tag whose assets do not exist yet;
/// 3. the newest published release, so a mid-release proof proves the last real thing;
/// 4. nothing reachable — an error, never a silent skip.
///
/// # Errors
///
/// Fails when an explicit override is not a published release (a 404 is a real error, not a skip), when no declared, published, or latest release can be found, or when the resolved tag is not `vX.Y.Z`.
pub fn resolve_tag(
    declared: &str,
    override_tag: &str,
    published: &dyn Fn(&str) -> Option<String>,
    latest: &dyn Fn() -> Option<String>,
) -> Result<(String, String)> {
    let resolved = if override_tag.is_empty() {
        match published(declared) {
            Some(tag) => tag,
            None => latest().context(
                "could not reach the releases API — neither the declared release nor the latest one answered",
            )?,
        }
    } else {
        override_tag.to_string()
    };
    if !shaped_like_tag(&resolved) {
        bail!("resolved release tag '{resolved}' is not a vX.Y.Z tag");
    }
    let summary = if override_tag.is_empty() {
        format!("proving {resolved} (declared {declared})")
    } else {
        format!("proving {resolved} (SANDBOX_RELEASE_VERSION override)")
    };
    Ok((resolved, summary))
}

/// The `gh api` adapter: the tag name, or `None` when the call fails. Capture-then-check,
/// not `|| true`: on a 404 `gh api` prints the JSON error body to stdout, so a caller that
/// trusted exit-status-then-stdout would hand `{"message":"Not Found"…}` on as a tag.
fn gh_tag(repo: &str, endpoint: &str) -> Option<String> {
    let output = StdCommand::new("gh")
        .args([
            "api",
            &format!("repos/{repo}/{endpoint}"),
            "--jq",
            ".tag_name",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!tag.is_empty()).then_some(tag)
}

/// Resolve and report: `resolved=` to `$GITHUB_OUTPUT`, the proving line to the step summary.
///
/// # Errors
///
/// Fails when the declared version cannot be read from the workspace `Cargo.toml`, when no release can be resolved (see [`resolve_tag`]), or when the `GITHUB_OUTPUT` or step summary cannot be written.
pub fn resolve_release_tag(root: &Path, repo: &str, override_tag: &str) -> Result<()> {
    let version = crate::version::workspace_version(root)
        .context("could not read the declared version from the workspace Cargo.toml")?;
    let declared = format!("v{version}");
    let (resolved, summary) = resolve_tag(
        &declared,
        override_tag,
        &|tag| gh_tag(repo, &format!("releases/tags/{tag}")),
        &|| gh_tag(repo, "releases/latest"),
    )?;
    if resolved != declared && override_tag.is_empty() {
        eprintln!(
            "::notice::{declared} is declared but not published yet (mid-release?); proving the newest published release {resolved}"
        );
    }
    crate::util::github_output("resolved", &resolved);
    crate::util::github_summary(&summary);
    println!("{resolved}");
    Ok(())
}

// ---------------------------------------------------------------------------
// airlock-self-bin
// ---------------------------------------------------------------------------

/// The release asset name for a Pixi platform id. Delegates to
/// [`pixi_sandbox_core::platform::Platform`] (task-55/task-59) instead of restating the five
/// names xtask also stages and checksums (`release_assets.rs`).
///
/// # Errors
///
/// Fails when `platform` has no static pixi-sandbox release asset.
pub fn static_asset_for_platform(platform: &str) -> Result<&'static str> {
    Platform::from_str(platform)
        .map(Platform::asset_name)
        .map_err(|_| {
            anyhow::anyhow!("no static pixi-sandbox release asset is known for {platform}")
        })
}

/// Download the released standalone binary for `platform` at `tag` into `out`.
///
/// # Errors
///
/// Fails when the output directory cannot be created, when `gh release download` cannot be started, or when it exits non-zero for `tag`.
pub fn airlock_self_bin(repo: &str, tag: &str, platform: &str, out: &Path) -> Result<()> {
    airlock_self_bin_with_gh(Path::new("gh"), repo, tag, platform, out)
}

#[doc(hidden)] // test boundary (tests/airlock.rs)
pub fn airlock_self_bin_with_gh(
    gh: &Path,
    repo: &str,
    tag: &str,
    platform: &str,
    out: &Path,
) -> Result<()> {
    let asset = static_asset_for_platform(platform)?;
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    let status = StdCommand::new(gh)
        .args([
            "release",
            "download",
            tag,
            "--repo",
            repo,
            "--pattern",
            asset,
            "--output",
        ])
        .arg(out)
        .arg("--clobber")
        .status()
        .with_context(|| format!("starting gh release download for {asset}"))?;
    if !status.success() {
        bail!("downloading static release asset {asset} from {repo}@{tag} failed");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(out)
            .with_context(|| format!("reading {}", out.display()))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        std::fs::set_permissions(out, permissions)
            .with_context(|| format!("making {} executable", out.display()))?;
    }
    println!("downloaded {asset} to {}", out.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// airlock-pack
// ---------------------------------------------------------------------------

/// Split the matrix `environments` value (`default,web`) into names. Empty names are
/// dropped; zero names left is the same error the plan CLI raises for `--envs ""`.
///
/// # Errors
///
/// Fails when no environment names remain once empty entries are dropped.
pub fn split_envs(envs: &str) -> Result<Vec<String>> {
    let names: Vec<String> = envs
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(str::to_string)
        .collect();
    if names.is_empty() {
        bail!("the matrix named no environments to pack");
    }
    Ok(names)
}

/// Resolve `bin` against a PATH value (not the process PATH — a parameter, so it is
/// testable). The airlock proof packs with the *released* binary the job installed, and the
/// shell this replaces found it with `command -v pixi-sandbox`.
#[must_use]
pub fn resolve_on_path(bin: &str, path_value: &std::ffi::OsStr) -> Option<PathBuf> {
    for dir in std::env::split_paths(path_value) {
        let candidate = dir.join(bin);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

/// `pixi global install` exposes commands through tiny trampolines next to
/// `trampoline_configuration/<name>.json`. A transport cannot embed that trampoline by itself:
/// once restored under `.pixi/tools/<platform>/`, it looks for a sibling config directory that
/// the packer never shipped. Follow the config to the real package binary before `pack --self-bin`
/// embeds it.
#[doc(hidden)] // test boundary (tests/airlock.rs)
#[must_use]
pub fn resolve_pixi_trampoline(bin: &Path) -> PathBuf {
    let Some(name) = bin.file_name().and_then(|name| name.to_str()) else {
        return bin.to_path_buf();
    };
    let config = bin
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("trampoline_configuration")
        .join(format!("{name}.json"));
    let Ok(text) = std::fs::read_to_string(&config) else {
        return bin.to_path_buf();
    };
    let Ok(json) = serde_json::from_str::<Value>(&text) else {
        return bin.to_path_buf();
    };
    find_executable_named(&json, name).unwrap_or_else(|| bin.to_path_buf())
}

fn find_executable_named(value: &Value, name: &str) -> Option<PathBuf> {
    match value {
        Value::String(candidate) => {
            let path = PathBuf::from(candidate);
            (path.file_name().and_then(|file| file.to_str()) == Some(name) && is_executable(&path))
                .then_some(path)
        }
        Value::Array(items) => items
            .iter()
            .find_map(|item| find_executable_named(item, name)),
        Value::Object(fields) => fields
            .values()
            .find_map(|item| find_executable_named(item, name)),
        _ => None,
    }
}

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path).is_ok_and(|m| m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The pack argv, with the `--cargo-vendor` flag only when asked — a flag with no value
/// form, which is exactly the conditional a plain task cannot express.
#[must_use]
pub fn pack_argv(envs: &str, out: &Path, cargo_vendor: bool, self_bin: &Path) -> Vec<String> {
    let mut argv: Vec<String> = ["pack", "--repo-root", ".", "--envs", envs, "--output-dir"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    argv.push(out.display().to_string());
    argv.extend(["--fetch-tools".to_string(), "--self-bin".to_string()]);
    argv.push(self_bin.display().to_string());
    if cargo_vendor {
        argv.push("--cargo-vendor".to_string());
    }
    argv
}

/// Materialise every named environment (`pixi install --frozen`, the loop the shell did),
/// then pack the transport with the released binary found on PATH (or `self_bin`).
///
/// # Errors
///
/// Fails when an environment cannot be installed with `pixi install --frozen`, or when the transport cannot be packed with the chosen `pixi-sandbox` binary.
pub fn airlock_pack(
    repo_root: &Path,
    envs: &str,
    out: &Path,
    cargo_vendor: &str,
    self_bin: Option<&Path>,
) -> Result<()> {
    airlock_pack_with_pixi(
        Path::new("pixi"),
        repo_root,
        envs,
        out,
        cargo_vendor,
        self_bin,
    )
}

#[doc(hidden)] // test boundary (tests/airlock.rs)
pub fn airlock_pack_with_pixi(
    pixi: &Path,
    repo_root: &Path,
    envs: &str,
    out: &Path,
    cargo_vendor: &str,
    self_bin: Option<&Path>,
) -> Result<()> {
    for environment in split_envs(envs)? {
        eprintln!("installing environment {environment} (frozen)");
        let status = StdCommand::new(pixi)
            .args(["install", "--frozen", "-e"])
            .arg(&environment)
            .current_dir(repo_root)
            .status()
            .context("starting pixi — the proof runner must have it on PATH")?;
        if !status.success() {
            bail!("pixi install --frozen -e {environment} failed");
        }
    }

    let vendor = match cargo_vendor {
        "" | "true" => true,
        "false" => false,
        other => bail!("--cargo-vendor must be true or false, not '{other}'"),
    };
    let self_bin = match self_bin {
        Some(path) => path.to_path_buf(),
        None => resolve_on_path(
            "pixi-sandbox",
            &std::env::var_os("PATH").unwrap_or_default(),
        )
        .context("pixi-sandbox is not on PATH — install the released package first")?,
    };
    let self_bin = resolve_pixi_trampoline(&self_bin);

    // The transport directory is scratch under the runner's temp; pack refuses a dirty dir,
    // so start it empty exactly like the `rm -rf` in the shell this replaces.
    if out.exists() {
        std::fs::remove_dir_all(out).with_context(|| format!("clearing {}", out.display()))?;
    }

    let argv = pack_argv(envs, out, vendor, &self_bin);
    eprintln!("pixi-sandbox {}", argv.join(" "));
    let status = StdCommand::new(&self_bin)
        .args(&argv)
        .current_dir(repo_root)
        .status()
        .context("starting the released pixi-sandbox")?;
    if !status.success() {
        bail!("the released pixi-sandbox failed to pack the transport");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// airlock-fetch
// ---------------------------------------------------------------------------

/// Fetch the published branch the way a developer's machine would: a fresh host repo, a
/// remote, one shallow fetch, a linked worktree — all through `pixi-sandbox-git` (D9).
///
/// # Errors
///
/// Fails when the host directory cannot be cleared or created, or when any git step (adding the remote, the shallow fetch, the linked worktree) fails.
pub fn airlock_fetch(remote: &str, branch: &str, host_dir: &Path, worktree: &Path) -> Result<()> {
    if host_dir.exists() {
        std::fs::remove_dir_all(host_dir)
            .with_context(|| format!("clearing the previous host dir {}", host_dir.display()))?;
    }
    std::fs::create_dir_all(host_dir)
        .with_context(|| format!("creating the host dir {}", host_dir.display()))?;

    let git = ShellGit::new();
    git.init_repo(host_dir)?;
    git.remote_add(host_dir, "origin", remote)?;
    git.fetch_shallow(host_dir, "origin", &format!("{branch}:{branch}"))?;
    git.worktree_add(host_dir, worktree, branch, true)?;
    println!("fetched {branch} from {remote} into {}", worktree.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// deny-egress
// ---------------------------------------------------------------------------

/// The egress-denial wrapper for a command: `sudo unshare -n` on Linux (a private network
/// namespace — no route, no DNS, nothing to fall back to), `sudo sandbox-exec` with the
/// outbound-denied profile on macOS, and a loud error anywhere else, because a proof that
/// silently ran with the network reachable would be a green build that proves nothing.
///
/// # Errors
///
/// Fails for an operating system with no known egress-denial mechanism. Silently running with the network reachable would be a green build that proves nothing.
pub fn denial_argv(os: &str, command: &[String]) -> Result<Vec<String>> {
    let mut argv: Vec<String> = match os {
        "linux" => ["sudo", "unshare", "-n", "--"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        "macos" => vec![
            "sudo".to_string(),
            "sandbox-exec".to_string(),
            "-p".to_string(),
            DARWIN_EGRESS_PROFILE.to_string(),
        ],
        other => bail!("no egress-denial mechanism known for {other}"),
    };
    argv.extend(command.iter().cloned());
    Ok(argv)
}

#[doc(hidden)] // test boundary (tests/airlock.rs)
pub fn command_with_absolute_program(
    command: &[String],
    path_value: &std::ffi::OsStr,
) -> Result<Vec<String>> {
    let Some(program) = command.first() else {
        bail!("deny-egress needs a command after `--`");
    };
    if program.contains('/') || program.contains('\\') {
        return Ok(command.to_vec());
    }
    let Some(path) = resolve_on_path(program, path_value) else {
        bail!("{program} is not on PATH; deny-egress cannot run it under sudo's sanitized path");
    };
    let mut resolved = command.to_vec();
    resolved[0] = path.display().to_string();
    Ok(resolved)
}

/// Re-exec `command` under the platform's egress denial, inheriting stdio, and exit with
/// whatever it exited with — the gate's failure must look like the gate's failure.
///
/// # Errors
///
/// Fails when the egress-denial wrapper cannot be built (see [`denial_argv`]) or the denied command cannot be started. A command that runs and fails is not an error here: its exit status is passed through.
pub fn deny_egress(command: &[String]) -> Result<()> {
    let command =
        command_with_absolute_program(command, &std::env::var_os("PATH").unwrap_or_default())?;
    let argv = denial_argv(std::env::consts::OS, &command)?;
    eprintln!("egress denied: {}", argv.join(" "));
    let status = StdCommand::new(&argv[0])
        .args(&argv[1..])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .context("starting the egress-denied command")?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}
