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

    write_touched_report(root)?;
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
fn stamp_version(path: &Path, semver: &str) -> Result<()> {
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

fn reversion_line(line: &str, semver: &str) -> Option<String> {
    let rest = line.strip_prefix("version = \"")?;
    let close = rest.find('"')?;
    Some(format!("version = \"{semver}\"{}", &rest[close + 1..]))
}

/// On lines pinning an internal `pixi-sandbox*` path dependency, move the exact version
/// requirement to the new workspace version. Returns whether anything changed.
fn stamp_internal_pins(path: &Path, semver: &str) -> Result<bool> {
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
/// would have staged the manifests and quietly dropped every documentation fix.
fn write_touched_report(root: &Path) -> Result<()> {
    let mut files = ShellGit::new()
        .worktree_status_files(root)
        .context("running git status for the touched-file report")?;
    files.sort();
    let report = files.iter().map(|f| format!("{f}\n")).collect::<String>();
    let touched_file =
        std::env::var("RELEASE_TOUCHED_FILE").unwrap_or_else(|_| ".release-touched".into());
    crate::util::write_atomic(&root.join(&touched_file), &report)?;
    eprintln!("  {} file(s) recorded in {touched_file}", files.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{reversion_line, stamp_internal_pins, stamp_version};
    use std::fs;

    #[test]
    fn only_the_column_zero_version_line_is_stamped() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("pixi.toml");
        fs::write(
            &path,
            "version = \"1.0.0\"\n[deps]\n  version = \"9.9.9\"\npkg = { version = \"3.3.3\" }\n",
        )
        .expect("manifest");
        stamp_version(&path, "2.0.0").expect("stamp");
        let text = fs::read_to_string(&path).expect("readback");
        assert_eq!(
            text,
            "version = \"2.0.0\"\n[deps]\n  version = \"9.9.9\"\npkg = { version = \"3.3.3\" }\n"
        );
    }

    #[test]
    fn a_manifest_without_a_top_level_version_is_refused() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("x.toml");
        fs::write(&path, "[package]\nname = \"x\"\n").expect("manifest");
        assert!(stamp_version(&path, "2.0.0").is_err());
    }

    #[test]
    fn the_suffix_after_the_closing_quote_survives() {
        assert_eq!(
            reversion_line("version = \"1.0.0\" # keep me\n", "2.0.0"),
            Some("version = \"2.0.0\" # keep me\n".to_string())
        );
        assert_eq!(reversion_line("  version = \"1.0.0\"\n", "2.0.0"), None);
    }

    #[test]
    fn internal_path_pins_move_and_external_requirements_do_not() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("Cargo.toml");
        fs::write(
            &path,
            "[dependencies]\nanyhow = { version = \"1\" }\npixi-sandbox = { path = \"../pixi-sandbox\", version = \"1.0.0\" }\npixi-sandbox-core = { version = \"1.0.0\", path = \"../core\" }\n",
        )
        .expect("manifest");
        assert!(stamp_internal_pins(&path, "2.0.0").expect("stamp"));
        let text = fs::read_to_string(&path).expect("readback");
        assert!(text.contains("anyhow = { version = \"1\" }"), "{text}");
        assert!(
            text.contains("pixi-sandbox = { path = \"../pixi-sandbox\", version = \"2.0.0\" }"),
            "{text}"
        );
        assert!(
            text.contains("pixi-sandbox-core = { version = \"2.0.0\", path = \"../core\" }"),
            "{text}"
        );
        // Idempotent: a second pass reports no change.
        assert!(!stamp_internal_pins(&path, "2.0.0").expect("stamp again"));
    }
}
