//! Resolve the next release version and stamp it into the repository, without touching git.
//!
//! What it does (and deliberately does NOT do):
//!   * resolves the target version with convco (respecting .versionrc / preMajor),
//!   * rewrites the single top-level `version = "…"` in Cargo.toml, pixi.toml and the conda
//!     package manifest (crates/pixi-sandbox/pixi.toml — `pixi publish` needs a literal
//!     there, and `check-repository` asserts it agrees with Cargo.toml),
//!   * repins the internal path-dependency versions so Cargo can still resolve the bumped
//!     members, and refreshes Cargo.lock,
//!   * repins every documented reference to our own release in the README family (the
//!     `release_refs` module — the same predicate the lint checks with, so the reporter and
//!     the fixer cannot disagree),
//!   * regenerates CHANGELOG.md from the conventional-commit history.
//!
//! It never commits, tags, or pushes — that is the release workflow's job, so a human or a
//! dry-run can review the diff first. The only value on stdout is `vX.Y.Z`; every log line
//! goes to stderr so callers capture the version with a plain command substitution.
//!
//! The list of files it changed is written to the path in `$RELEASE_TOUCHED_FILE` (default
//! `.release-touched`), one per line, because the release workflow has to `git add` exactly
//! those. A hand-kept list in the workflow is a second place to forget: when the repin was
//! added it would have staged the manifests and silently dropped the ~90 documentation fixes
//! it had just made.
//!
//! Requires convco and git on PATH (both are in the `default` pixi environment).

use anyhow::{Context, Result, bail};
use pixi_sandbox_git::ShellGit;
use regex::Regex;
use std::path::Path;
use std::process::Command;

/// Run the whole preparation; returns the `vX.Y.Z` tag for the caller to print.
pub fn run(root: &Path, selector: &str) -> Result<String> {
    let semver = resolve_version(root, selector)?;
    // The version flows into git tags and file contents; keep it a strict semver core.
    if !crate::util::is_strict_semver(&semver) {
        bail!("resolved version '{semver}' is not X.Y.Z");
    }
    let tag = format!("v{semver}");

    let current = crate::version::workspace_version(root).unwrap_or_default();
    eprintln!(
        "→ current version : {}",
        if current.is_empty() {
            "<none>"
        } else {
            &current
        }
    );
    eprintln!("→ target  version : {semver}  (selector: {selector})");
    if semver == current {
        eprintln!(
            "  note: target equals the current version; nothing new to bump but continuing so the changelog regenerates"
        );
    }

    // Only the top-level `version = "…"` in each manifest starts at column 0; dependency pins
    // are indented or written `package = { version = … }`, so the anchored substitution is
    // unambiguous. The conda package manifest is its own standalone pixi workspace
    // (pixi-build), so it cannot inherit the version and `pixi publish` needs a literal: it
    // shipped stale at 0.2.0 while the workspace was at 0.3.2 — stamping it here plus the
    // equality check in `check-repository` turns that drift into a red build instead of a
    // wrongly-versioned .conda.
    for manifest in ["Cargo.toml", "pixi.toml", "crates/pixi-sandbox/pixi.toml"] {
        stamp_version(&root.join(manifest), &semver)
            .with_context(|| format!("stamping {manifest}"))?;
        eprintln!("  stamped {manifest}");
    }

    // Workspace packages pin internal path dependencies with an exact version so cargo-deny
    // does not treat them as wildcards. They must move with the workspace version or Cargo
    // refuses to resolve the bumped members. Only internal-dep lines are rewritten; external
    // requirements are release-independent.
    for member in ["crates/pixi-sandbox/Cargo.toml", "crates/xtask/Cargo.toml"] {
        let path = root.join(member);
        if path.is_file() && stamp_internal_pins(&path, &semver)? {
            eprintln!("  stamped {member} (internal dependency pins)");
        }
    }

    // The committed relock workflow stamps the CLI version. Render it only after Cargo.toml
    // carries the target version, and read that manifest at runtime: this xtask process was
    // compiled before the bump, so env!("CARGO_PKG_VERSION") still names the old release.
    // The v0.5.3 release exposed that skew by committing a 0.5.2 workflow stamp.
    refresh_generated_files(root)?;

    // Keep Cargo.lock's workspace-member versions in step so a later `cargo build --locked`
    // does not trip over a stale lock. `--workspace` touches only the members, not external
    // pins; prefer offline (the vendor/cache is enough because no new dependency appears).
    if root.join("Cargo.lock").is_file() {
        eprintln!("→ syncing Cargo.lock (cargo update --workspace)");
        let quiet = |offline: bool| {
            let mut cmd = Command::new("cargo");
            cmd.current_dir(root).args(["update", "--workspace"]);
            if offline {
                cmd.arg("--offline");
            }
            cmd.stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
        };
        if !quiet(true) && !quiet(false) {
            eprintln!(
                "  warning: could not refresh Cargo.lock automatically — check it before releasing"
            );
        }
    }

    // Move every reference to OUR release in the published docs to the new tag. Without this
    // the release commit bumps the manifests and lands a `main` that contradicts itself:
    // `check-repository` fails on ~90 references still naming the previous release, and a
    // reader following the install one-liner gets the previous binaries.
    eprintln!("→ repinning the documented release references to {tag}");
    crate::release_refs::rewrite(root, &tag)?;

    // convco owns the changelog format (.versionrc). The changelog is generated *before* the
    // tag exists, so name the pending section after the version being cut instead of leaving
    // today's commits under a generic "Unreleased".
    eprintln!("→ regenerating CHANGELOG.md (convco changelog --unreleased {semver})");
    let changelog = convco(root, &["changelog", "--unreleased", &semver])?;
    crate::util::write_atomic(&root.join("CHANGELOG.md"), &changelog)?;

    let touched_file =
        std::env::var("RELEASE_TOUCHED_FILE").unwrap_or_else(|_| ".release-touched".into());
    write_touched_report(root, &root.join(touched_file))?;
    eprintln!("→ prepared release {tag}");
    Ok(tag)
}

/// convco always prints the bare semver here; the leading `v` is added once, by the caller.
fn resolve_version(root: &Path, selector: &str) -> Result<String> {
    let bumped = |extra: &[&str]| -> Result<String> {
        let mut args = vec!["version", "--bump"];
        args.extend_from_slice(extra);
        Ok(convco(root, &args)?.trim().to_string())
    };
    match selector {
        "auto" => bumped(&[]),
        "major" => bumped(&["--major"]),
        "minor" => bumped(&["--minor"]),
        "patch" => bumped(&["--patch"]),
        explicit if explicit.starts_with('v') => Ok(explicit[1..].trim().to_string()),
        explicit if explicit.starts_with(|c: char| c.is_ascii_digit()) => {
            Ok(explicit.trim().to_string())
        }
        other => {
            bail!("unrecognised version selector: '{other}' (use auto|major|minor|patch|vX.Y.Z)")
        }
    }
}

#[doc(hidden)] // test boundary (tests/prepare_release.rs)
pub fn refresh_generated_files(root: &Path) -> Result<()> {
    eprintln!("→ refreshing version-stamped generated files");
    crate::workflow::render_relock(root).context("rendering the release-version relock workflow")
}

fn convco(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("convco")
        .current_dir(root)
        .args(args)
        .output()
        .context("convco not found on PATH — run through pixi (e.g. 'pixi run prepare-release')")?;
    if !output.status.success() {
        bail!(
            "convco {} failed:\n{}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Rewrite every column-0 `version = "…"` line, keeping whatever follows the closing quote.
#[doc(hidden)] // test boundary (tests/prepare_release.rs)
pub fn stamp_version(path: &Path, semver: &str) -> Result<()> {
    let text = crate::util::read(path)?;
    let mut stamped_any = false;
    let rewritten: String = text
        .split_inclusive('\n')
        .map(|line| match reversion_line(line, semver) {
            Some(replaced) => {
                stamped_any = true;
                replaced
            }
            None => line.to_string(),
        })
        .collect();
    if !stamped_any {
        bail!("no top-level 'version = \"…\"' found in {}", path.display());
    }
    crate::util::write_atomic(path, &rewritten)
}

#[doc(hidden)] // test boundary (tests/prepare_release.rs)
pub fn reversion_line(line: &str, semver: &str) -> Option<String> {
    let rest = line.strip_prefix("version = \"")?;
    let close = rest.find('"')?;
    Some(format!("version = \"{semver}\"{}", &rest[close + 1..]))
}

/// On lines pinning an internal `pixi-sandbox*` path dependency, move the exact version
/// requirement to the new workspace version. Returns whether anything changed.
#[doc(hidden)] // test boundary (tests/prepare_release.rs)
pub fn stamp_internal_pins(path: &Path, semver: &str) -> Result<bool> {
    let internal = Regex::new(r"pixi-sandbox(-core|-git)?.*path *=").expect("static regex");
    let version_req = Regex::new(r#"version = "[^"]*""#).expect("static regex");
    let text = crate::util::read(path)?;
    let rewritten: String = text
        .split_inclusive('\n')
        .map(|line| {
            if internal.is_match(line) {
                version_req
                    .replace_all(line, format!("version = \"{semver}\"").as_str())
                    .into_owned()
            } else {
                line.to_string()
            }
        })
        .collect();
    let changed = rewritten != text;
    if changed {
        crate::util::write_atomic(path, &rewritten)?;
    }
    Ok(changed)
}

/// Report exactly what this run changed, derived from the working tree rather than from a
/// hard-coded manifest of "the files this is supposed to touch" — that manifest was correct
/// right up until the repin added eleven more files to the set, at which point `git add`
/// would have staged the manifests and quietly dropped every documentation fix. The query
/// goes through `pixi-sandbox-git` (D9); the output path stays explicit so the release
/// workflow's environment lookup is exercised once at the entrypoint and this function is
/// tempdir-fixture testable.
#[doc(hidden)] // test boundary (tests/prepare_release.rs)
pub fn write_touched_report(root: &Path, touched_file: &Path) -> Result<()> {
    let mut files = ShellGit::new()
        .worktree_status_files(root)
        .context("running git status for the touched-file report")?;
    files.sort();
    let report = files.iter().map(|f| format!("{f}\n")).collect::<String>();
    crate::util::write_atomic(touched_file, &report)?;
    eprintln!(
        "  {} file(s) recorded in {}",
        files.len(),
        touched_file.display()
    );
    Ok(())
}
