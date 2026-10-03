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
const DARWIN_EGRESS_PROFILE: &str = "(version 1)(allow default)(deny network-outbound)";

// ---------------------------------------------------------------------------
// airlock-matrix
// ---------------------------------------------------------------------------

/// The argv for the plan binary: override mode when `envs` is non-empty (a manual dispatch
/// names its target), config mode otherwise (the reviewed `.pixi-sandbox.toml`).
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
pub fn resolve_tag(
    declared: &str,
    override_tag: &str,
    published: &dyn Fn(&str) -> Option<String>,
    latest: &dyn Fn() -> Option<String>,
) -> Result<(String, String)> {
    let resolved = if !override_tag.is_empty() {
        override_tag.to_string()
    } else {
        match published(declared) {
            Some(tag) => tag,
            None => latest().context(
                "could not reach the releases API — neither the declared release nor the latest one answered",
            )?,
        }
    };
    if !shaped_like_tag(&resolved) {
        bail!("resolved release tag '{resolved}' is not a vX.Y.Z tag");
    }
    let summary = if !override_tag.is_empty() {
        format!("proving {resolved} (SANDBOX_RELEASE_VERSION override)")
    } else {
        format!("proving {resolved} (declared {declared})")
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
pub fn static_asset_for_platform(platform: &str) -> Result<&'static str> {
    Platform::from_str(platform)
        .map(Platform::asset_name)
        .map_err(|_| {
            anyhow::anyhow!("no static pixi-sandbox release asset is known for {platform}")
        })
}

pub fn airlock_self_bin(repo: &str, tag: &str, platform: &str, out: &Path) -> Result<()> {
    airlock_self_bin_with_gh(Path::new("gh"), repo, tag, platform, out)
}

fn airlock_self_bin_with_gh(
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
fn resolve_pixi_trampoline(bin: &Path) -> PathBuf {
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
        std::fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

/// The pack argv, with the `--cargo-vendor` flag only when asked — a flag with no value
/// form, which is exactly the conditional a plain task cannot express.
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

fn airlock_pack_with_pixi(
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

fn command_with_absolute_program(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command as StdCommand;

    #[test]
    fn plan_argv_is_config_mode_without_envs_and_override_mode_with_them() {
        assert_eq!(
            plan_argv(
                "target/release/pixi-sandbox",
                ".pixi-sandbox.toml",
                "",
                "",
                "",
                "",
                ""
            ),
            [
                "target/release/pixi-sandbox",
                "plan",
                "--config",
                ".pixi-sandbox.toml",
                "--json"
            ]
        );
        assert_eq!(
            plan_argv("BIN", "C", "default,web", "osx-arm64", "", "", "false"),
            [
                "BIN",
                "plan",
                "--bundle",
                "custom",
                "--envs",
                "default,web",
                "--platform",
                "osx-arm64",
                "--branch-prefix",
                "sandbox",
                "--cargo-vendor",
                "false",
                "--json"
            ]
        );
        // Empty bundle/prefix/cargo-vendor fall back to the same defaults the shell applied.
        assert_eq!(
            plan_argv("BIN", "C", "default", "linux-64", "proof", "ci", ""),
            [
                "BIN",
                "plan",
                "--bundle",
                "proof",
                "--envs",
                "default",
                "--platform",
                "linux-64",
                "--branch-prefix",
                "ci",
                "--cargo-vendor",
                "true",
                "--json"
            ]
        );
    }

    #[test]
    fn a_valid_matrix_passes_through_unchanged_and_an_empty_include_is_an_error() {
        let json = r#"{"schema":1,"include":[{"bundle":"developer"}]}"#;
        assert_eq!(validate_matrix(json).expect("valid"), json);

        for bad in [
            r#"{"schema":1,"include":[]}"#,
            r#"{"schema":1}"#,
            "not json at all",
        ] {
            let error = validate_matrix(bad).expect_err("must be rejected");
            assert!(
                format!("{error:#}").contains("include") || format!("{error:#}").contains("JSON"),
                "must name the problem: {error:#}"
            );
        }
    }

    #[test]
    fn an_override_is_honoured_without_any_api_call() {
        let called = std::sync::atomic::AtomicBool::new(false);
        let (resolved, summary) = resolve_tag(
            "v0.3.7",
            "v0.3.6",
            &|_| {
                called.store(true, std::sync::atomic::Ordering::Relaxed);
                Some("v0.3.7".to_string())
            },
            &|| None,
        )
        .expect("override wins");
        assert_eq!(resolved, "v0.3.6");
        assert_eq!(summary, "proving v0.3.6 (SANDBOX_RELEASE_VERSION override)");
        assert!(!called.load(std::sync::atomic::Ordering::Relaxed));
    }

    #[test]
    fn a_declared_but_unpublished_release_falls_back_to_the_newest_published_one() {
        let (resolved, summary) = resolve_tag(
            "v0.3.8",
            "",
            &|declared| {
                assert_eq!(declared, "v0.3.8");
                None // mid-release: the declared tag has no published assets yet
            },
            &|| Some("v0.3.7".to_string()),
        )
        .expect("fallback");
        assert_eq!(resolved, "v0.3.7");
        assert_eq!(summary, "proving v0.3.7 (declared v0.3.8)");
    }

    #[test]
    fn an_unreachable_releases_api_is_an_error_and_a_garbage_tag_is_refused() {
        let error = resolve_tag("v0.3.8", "", &|_| None, &|| None).expect_err("unreachable");
        assert!(
            format!("{error:#}").contains("could not reach the releases API"),
            "{error:#}"
        );
        for garbage in ["", "latest", "{\"message\":\"Not Found\"}"] {
            let error = resolve_tag("v0.3.8", "", &|_| Some(garbage.to_string()), &|| None)
                .expect_err("shape");
            assert!(
                format!("{error:#}").contains("not a vX.Y.Z tag"),
                "{garbage} must be refused: {error:#}"
            );
        }
    }

    #[test]
    fn release_assets_are_selected_from_the_pixi_platform() {
        assert_eq!(
            static_asset_for_platform("linux-64").expect("linux"),
            "pixi-sandbox-x86_64-unknown-linux-musl"
        );
        assert_eq!(
            static_asset_for_platform("osx-arm64").expect("mac"),
            "pixi-sandbox-aarch64-apple-darwin"
        );
        assert!(static_asset_for_platform("freebsd-64").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn self_bin_download_uses_the_static_asset_and_marks_it_executable() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let gh = dir.path().join("gh");
        write_executable(
            &gh,
            r#"#!/bin/sh
set -eu
log="$(dirname "$0")/gh.args"
: > "$log"
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$log"
done
out=""
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--output" ]; then
    shift
    out="$1"
  fi
  shift || true
done
printf '#!/bin/sh\nexit 0\n' > "$out"
chmod 600 "$out"
"#,
        );
        let out = dir.path().join("nested/pixi-sandbox");

        airlock_self_bin_with_gh(&gh, "owner/repo", "v0.4.0", "linux-64", &out).expect("download");

        let args = fs::read_to_string(dir.path().join("gh.args")).expect("args");
        assert!(args.contains("release\ndownload\nv0.4.0\n"), "{args}");
        assert!(args.contains("--repo\nowner/repo\n"), "{args}");
        assert!(
            args.contains("--pattern\npixi-sandbox-x86_64-unknown-linux-musl\n"),
            "{args}"
        );
        assert!(args.contains("--clobber\n"), "{args}");
        assert!(out.is_file());
        assert_ne!(
            fs::metadata(&out).expect("metadata").permissions().mode() & 0o111,
            0,
            "downloaded self binary must be executable"
        );
    }

    #[test]
    fn env_lists_split_on_commas_and_empty_names_are_dropped() {
        assert_eq!(
            split_envs("default,web").expect("split"),
            ["default", "web"]
        );
        assert_eq!(
            split_envs(" default , web ").expect("split"),
            ["default", "web"]
        );
        for empty in ["", " , "] {
            assert!(split_envs(empty).is_err(), "'{empty}' names nothing");
        }
    }

    #[test]
    fn pack_argv_carries_the_vendor_flag_only_when_asked() {
        let with = pack_argv(
            "default,web",
            Path::new("/t"),
            true,
            Path::new("/bin/pixi-sandbox"),
        );
        assert!(with.contains(&"--cargo-vendor".to_string()));
        let without = pack_argv(
            "default",
            Path::new("/t"),
            false,
            Path::new("/bin/pixi-sandbox"),
        );
        assert!(!without.contains(&"--cargo-vendor".to_string()));
        for argv in [&with, &without] {
            assert_eq!(
                &argv[..3],
                &[
                    "pack".to_string(),
                    "--repo-root".to_string(),
                    ".".to_string()
                ]
            );
            assert!(argv.contains(&"--fetch-tools".to_string()));
            assert!(argv.contains(&"/bin/pixi-sandbox".to_string()));
        }
    }

    #[cfg(unix)]
    #[test]
    fn airlock_pack_installs_named_envs_and_runs_the_static_self_binary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path().join("repo");
        fs::create_dir_all(&repo).expect("repo");
        let pixi = dir.path().join("pixi");
        write_executable(
            &pixi,
            r#"#!/bin/sh
set -eu
printf '%s\n' "$@" >> "$(dirname "$0")/pixi.args"
"#,
        );
        let self_bin = dir.path().join("pixi-sandbox");
        write_executable(
            &self_bin,
            r#"#!/bin/sh
set -eu
: > "$(dirname "$0")/self.args"
for arg in "$@"; do
  printf '%s\n' "$arg" >> "$(dirname "$0")/self.args"
done
"#,
        );
        let out = dir.path().join("transport");
        fs::create_dir_all(&out).expect("out");
        fs::write(out.join("stale"), b"old").expect("stale");

        airlock_pack_with_pixi(&pixi, &repo, "default, web", &out, "false", Some(&self_bin))
            .expect("pack");

        let pixi_args = fs::read_to_string(dir.path().join("pixi.args")).expect("pixi args");
        assert!(
            pixi_args.contains("install\n--frozen\n-e\ndefault\ninstall\n--frozen\n-e\nweb\n"),
            "{pixi_args}"
        );
        assert!(
            !out.join("stale").exists(),
            "packing starts from an empty transport dir"
        );
        let self_args = fs::read_to_string(dir.path().join("self.args")).expect("self args");
        assert!(self_args.contains("pack\n--repo-root\n.\n"), "{self_args}");
        assert!(
            self_args.contains(&format!("--output-dir\n{}\n", out.display())),
            "{self_args}"
        );
        assert!(
            self_args.contains(&format!("--self-bin\n{}\n", self_bin.display())),
            "{self_args}"
        );
        assert!(
            !self_args.contains("--cargo-vendor\n"),
            "false disables cargo vendoring: {self_args}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn self_bin_resolution_finds_an_executable_on_the_given_path_and_nothing_else() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let bin = dir.path().join("pixi-sandbox");
        fs::write(&bin, b"#!/bin/true\n").expect("write");
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).expect("chmod");
        let plain = dir.path().join("not-executable");
        fs::write(&plain, b"payload").expect("write");

        let path_value = std::env::join_paths([dir.path(), Path::new("/nonexistent")])
            .expect("join")
            .to_string_lossy()
            .into_owned();
        assert_eq!(
            resolve_on_path("pixi-sandbox", std::ffi::OsStr::new(&path_value)),
            Some(bin.clone())
        );
        assert_eq!(
            resolve_on_path("not-executable", std::ffi::OsStr::new(&path_value)),
            None,
            "a non-executable file is not a command"
        );
        assert_eq!(
            resolve_on_path("absent", std::ffi::OsStr::new(&path_value)),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn pixi_global_trampolines_resolve_to_the_real_package_binary() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let exposed = dir.path().join("bin");
        let package = dir.path().join("env/bin");
        fs::create_dir_all(exposed.join("trampoline_configuration")).expect("mkdir");
        fs::create_dir_all(&package).expect("mkdir");
        let shim = exposed.join("pixi-sandbox");
        let real = package.join("pixi-sandbox");
        fs::write(&shim, b"trampoline").expect("write shim");
        fs::write(&real, b"#!/bin/true\n").expect("write real");
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).expect("chmod shim");
        fs::set_permissions(&real, fs::Permissions::from_mode(0o755)).expect("chmod real");
        fs::write(
            exposed.join("trampoline_configuration/pixi-sandbox.json"),
            format!(r#"{{"executable":"{}","args":[]}}"#, real.display()),
        )
        .expect("write config");

        assert_eq!(resolve_pixi_trampoline(&shim), real);
    }

    #[cfg(unix)]
    #[test]
    fn malformed_or_unhelpful_trampoline_configs_keep_the_original_binary() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let exposed = dir.path().join("bin");
        fs::create_dir_all(exposed.join("trampoline_configuration")).expect("mkdir");
        let shim = exposed.join("pixi-sandbox");
        fs::write(&shim, b"trampoline").expect("write shim");
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).expect("chmod shim");

        assert_eq!(resolve_pixi_trampoline(&shim), shim);
        fs::write(
            exposed.join("trampoline_configuration/pixi-sandbox.json"),
            b"not-json",
        )
        .expect("write invalid json");
        assert_eq!(resolve_pixi_trampoline(&shim), shim);
        fs::write(
            exposed.join("trampoline_configuration/pixi-sandbox.json"),
            b"null",
        )
        .expect("write null json");
        assert_eq!(resolve_pixi_trampoline(&shim), shim);
        assert_eq!(resolve_pixi_trampoline(Path::new("/")), PathBuf::from("/"));
    }

    #[cfg(unix)]
    #[test]
    fn deny_egress_absolutizes_the_program_before_sudo_sanitizes_path() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let pixi = dir.path().join("pixi");
        fs::write(&pixi, b"#!/bin/true\n").expect("write");
        fs::set_permissions(&pixi, fs::Permissions::from_mode(0o755)).expect("chmod");
        let path_value = std::env::join_paths([dir.path()]).expect("join");

        assert_eq!(
            command_with_absolute_program(&["pixi".into(), "run".into()], &path_value)
                .expect("resolve"),
            [pixi.display().to_string(), "run".to_string()]
        );
        assert_eq!(
            command_with_absolute_program(&["/usr/bin/pixi".into()], &path_value)
                .expect("absolute unchanged"),
            ["/usr/bin/pixi".to_string()]
        );
        assert!(
            command_with_absolute_program(&[], &path_value)
                .expect_err("empty command")
                .to_string()
                .contains("needs a command")
        );
        assert!(
            command_with_absolute_program(&["missing".into()], &path_value)
                .expect_err("missing program")
                .to_string()
                .contains("not on PATH")
        );
    }

    #[test]
    fn egress_denial_wraps_per_os_and_fails_loudly_elsewhere() {
        assert_eq!(
            denial_argv("linux", &["bash".to_string(), "gate.sh".to_string()]).expect("linux"),
            ["sudo", "unshare", "-n", "--", "bash", "gate.sh"]
        );
        assert_eq!(
            denial_argv("macos", &["/bin/bash".to_string(), "gate.sh".to_string()]).expect("macos"),
            [
                "sudo",
                "sandbox-exec",
                "-p",
                DARWIN_EGRESS_PROFILE,
                "/bin/bash",
                "gate.sh"
            ]
        );
        let error = denial_argv("windows", &["cmd".to_string()]).expect_err("unknown os");
        assert!(
            format!("{error:#}").contains("no egress-denial mechanism known for windows"),
            "{error:#}"
        );
    }

    /// A real fetch through a real bare remote: the point is the developer-shaped sequence
    /// (init, remote add, shallow fetch, worktree), so the fixture uses real git.
    #[test]
    fn the_published_branch_is_fetched_into_a_worktree_the_way_a_developer_machine_would() {
        let dir = tempfile::tempdir().expect("tempdir");
        let remote_path = dir.path().join("remote.git");
        let publish_dir = dir.path().join("payload");
        fs::create_dir_all(&publish_dir).expect("mkdir");
        fs::write(publish_dir.join("blob.txt"), b"transport bytes").expect("write");
        run_git(
            &["init", "-q", "--bare"],
            dir.path(),
            &[&remote_path.to_string_lossy()],
        );
        run_git(&["init", "-q", "-b", "main"], &publish_dir, &[]);
        run_git(&["add", "."], &publish_dir, &[]);
        run_git(&["commit", "-q", "-m", "transport"], &publish_dir, &[]);
        // Remote and refspec are separate arguments: a colon inside one token is a URL
        // spelling, not a refspec, and the push silently targets the wrong thing.
        run_git(
            &["push", "-q"],
            &publish_dir,
            &[
                remote_path.to_string_lossy().as_ref(),
                "HEAD:refs/heads/sandbox/proof-linux-64",
            ],
        );

        let host = dir.path().join("host");
        let worktree = dir.path().join("checkout");
        airlock_fetch(
            &remote_path.to_string_lossy(),
            "sandbox/proof-linux-64",
            &host,
            &worktree,
        )
        .expect("fetch");

        assert!(worktree.join("blob.txt").is_file());
        assert_eq!(
            fs::read(worktree.join("blob.txt")).expect("read"),
            b"transport bytes"
        );
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, contents: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, contents).expect("write executable");
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("chmod executable");
    }

    fn run_git(args: &[&str], cwd: &Path, extras: &[&str]) {
        let mut command = StdCommand::new("git");
        command.args(args);
        command.args(extras);
        command
            .env("GIT_AUTHOR_NAME", "fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid");
        let out = command.current_dir(cwd).output().expect("git runs");
        assert!(
            out.status.success(),
            "git {args:?} {extras:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}
