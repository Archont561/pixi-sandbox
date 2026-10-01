//! Repo-level consistency lints: the claims this repository makes about itself that no
//! ordinary cargo test is allowed to check.
//!
//! Why an xtask and not a `#[test]`: D10 — tests target fixtures, never this repository, and
//! `tests/fixtures.rs::no_test_targets_the_repository_root` enforces it. Every checker here
//! therefore takes an explicit root; only `main.rs` ever passes the real checkout, and the
//! unit tests below drive each policy against synthetic repositories in tempdirs.
//!
//! The checks, in order (numbering preserved from the retired shell lint):
//!  1. task-6 — no reference under `crates/` to the deleted pre-Rust implementation.
//!  2. task-7 — the README Platforms badge, `pixi.toml` and `.pixi-sandbox.toml` tell one
//!     platform story, and the Windows gap is documented rather than advertised.
//!  3. task-2 — version references cannot drift (`release_refs::scan`), and the conda package
//!     manifest (the one file whose format demands a restated literal) equals Cargo.toml.
//!  4. every third-party `uses:` is a full commit SHA with a trailing release label, so a
//!     moved tag cannot change what CI runs.
//!  5. the `install.sh` release asset renders from `templates/install.sh` at the tag, and no
//!     committed copy exists to drift (task-2).
//!  6. no workflow pins a literal `vX.Y.Z` release tag, so no proof silently keeps running
//!     against the previous release after a cut.
//!  7. task-12 — every published action path is a self-contained composite action and the two
//!     generated copies still match the root they are rendered from.
//!  8. the surviving shell scripts stay on the Bash 3.2 surface: the darwin runners execute
//!     them with macOS's /bin/bash 3.2, and v0.3.6's release died 127 on both darwin legs
//!     over one Bash-4 builtin (run 36865921206) — the regression that moved everything else
//!     into this xtask.

use crate::util::{lines_without_opt_out, version_tag_re};
use anyhow::{Result, bail};
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub struct Failure {
    pub headline: String,
    pub details: Vec<String>,
    pub hint: Option<String>,
}

impl Failure {
    fn new(headline: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            details: Vec::new(),
            hint: None,
        }
    }
    fn with(headline: impl Into<String>, details: Vec<String>, hint: impl Into<String>) -> Self {
        Self {
            headline: headline.into(),
            details,
            hint: Some(hint.into()),
        }
    }
}

/// Run every check, returning all failures in one pass so a red run names every problem.
pub fn check_repository(root: &Path) -> Result<Vec<Failure>> {
    let mut failures = Vec::new();
    stale_implementation_references(root, &mut failures); // 1
    platform_claims(root, &mut failures)?; // 2
    version_references(root, &mut failures)?; // 3
    action_pins(root, &mut failures)?; // 4
    install_template(root, &mut failures)?; // 5
    workflow_literal_tags(root, &mut failures)?; // 6
    published_actions(root, &mut failures)?; // 7
    bash32_surface(root, &mut failures)?; // 8
    Ok(failures)
}

/// `xtask check-repository`: GitHub-annotated adapter over [`check_repository`].
pub fn run(root: &Path) -> Result<()> {
    let failures = check_repository(root)?;
    if !failures.is_empty() {
        for failure in &failures {
            eprintln!("::error::{}", failure.headline);
            for detail in &failure.details {
                eprintln!("  {detail}");
            }
            if let Some(hint) = &failure.hint {
                eprintln!("  {hint}");
            }
        }
        bail!("repo consistency: {} check(s) failed", failures.len());
    }
    eprintln!(
        "repo consistency: crates/ is free of prototype references; platform and version claims agree; action pins are immutable; the install one-liner is rendered from templates/; no workflow pins a literal release tag; every published action path is self-contained and matches action.yml; the surviving shell scripts stay on the Bash 3.2 surface of the macOS runners" // stale-ref-allowed
    );
    Ok(())
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

// ---------------------------------------------------------------- 1. stale implementation references

// stale-ref-allowed — the check must spell the forbidden words out to forbid them.
const STALE_WORDS: [&str; 3] = ["python", "prototype", "knowledge/research"];

/// task-6: no reference under `crates/` to the implementation deleted in 0.2.0. It lived
/// under `.knowledge/`'s research directory; an airlocked reader has no network to discover
/// that it is gone. A line that must name the thing to forbid it opts out with the marker on
/// (or above) it.
///
/// Generated trees are pruned rather than left to gitignore: `crates/pixi-sandbox/.pixi/bld`
/// holds a vendored third-party registry the moment anyone runs `pixi run package` locally,
/// and this scan has to be as blind to it as git is.
fn stale_implementation_references(root: &Path, failures: &mut Vec<Failure>) {
    let mut hits = Vec::new();
    for file in walk_pruned(&root.join("crates"), &["rs", "md"]) {
        let Ok(text) = crate::util::read(&file) else {
            continue;
        };
        for (number, line) in lines_without_opt_out(&text) {
            let lower = line.to_lowercase();
            if STALE_WORDS.iter().any(|w| lower.contains(w)) {
                hits.push(format!("{}:{number}:{line}", rel(root, &file)));
            }
        }
    }
    if !hits.is_empty() {
        failures.push(Failure::with(
            "crates/ still references the implementation deleted in 0.2.0 (task-6):",
            hits,
            "point the text at .knowledge/design.md instead, or mark the line stale-ref-allowed",
        ));
    }
}

fn walk_pruned(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    const PRUNED: [&str; 4] = [".pixi", "target", "node_modules", "vendor"];
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_entry(|e| {
            !(e.file_type().is_dir() && PRUNED.contains(&e.file_name().to_string_lossy().as_ref()))
        })
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| extensions.iter().any(|want| x == *want))
        })
        .collect();
    files.sort();
    files
}

// ---------------------------------------------------------------- 2. platform claims

/// task-7: the README Platforms badge says exactly what `pixi.toml` declares, every platform
/// `.pixi-sandbox.toml` publishes is one of them, and the Windows gap (no conda-forge `bun`,
/// D11) is documented rather than advertised by the badge.
fn platform_claims(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let declared = workspace_platforms(root)?;
    let readme = crate::util::read(&root.join("README.md"))?;

    match badge_platforms(&readme) {
        None => failures.push(Failure::new(
            "README.md has no parsable Platforms badge (task-7)",
        )),
        Some(badge) => {
            if badge != declared {
                failures.push(Failure::new(format!(
                    "the Platforms badge advertises '{}' but pixi.toml declares '{}' (task-7)",
                    join(&badge),
                    join(&declared)
                )));
            }
            if badge.contains("win-64") {
                failures.push(Failure::new(
                    "the Platforms badge claims win-64, which is neither declared nor proven (task-7)",
                ));
            }
        }
    }

    for platform in published_platforms(root)? {
        if !declared.contains(&platform) {
            failures.push(Failure::new(format!(
                ".pixi-sandbox.toml publishes {platform}, which pixi.toml does not declare (task-7)"
            )));
        }
    }

    // Windows is not dropped in silence: one line has to carry the whole explanation, or the
    // reader has to assemble it — the platform, the blocker (`bun` as a word; `bundle` does
    // not count) and the decision that gates it.
    let gap =
        Regex::new(r"win-64.*[^a-z]bun([^a-z]|$).*[^A-Za-z]D11([^0-9]|$)").expect("static regex");
    if !readme.lines().any(|l| gap.is_match(l)) {
        failures.push(Failure::new(
            "README.md does not document the Windows gap on one line: name win-64, the conda-forge bun blocker and D11 (task-7)",
        ));
    }
    Ok(())
}

fn join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}

fn workspace_platforms(root: &Path) -> Result<BTreeSet<String>> {
    let manifest: toml::Table = crate::util::read(&root.join("pixi.toml"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("parsing pixi.toml: {e}"))?;
    Ok(string_array(
        manifest.get("workspace").and_then(|w| w.get("platforms")),
    ))
}

/// Every platform any bundle in the publish plan targets, parsed as TOML rather than scraped
/// by line shape (the retired pipeline silently read only one bundle).
fn published_platforms(root: &Path) -> Result<BTreeSet<String>> {
    let plan: toml::Table = crate::util::read(&root.join(".pixi-sandbox.toml"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("parsing .pixi-sandbox.toml: {e}"))?;
    let mut platforms = BTreeSet::new();
    if let Some(bundles) = plan.get("bundle").and_then(|b| b.as_array()) {
        for bundle in bundles {
            platforms.extend(string_array(bundle.get("platforms")));
        }
    }
    Ok(platforms)
}

fn string_array(value: Option<&toml::Value>) -> BTreeSet<String> {
    value
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The badge encodes platforms as `Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg`:
/// `%20%7C%20` separates entries and shields.io doubles a literal hyphen.
fn badge_platforms(readme: &str) -> Option<BTreeSet<String>> {
    let badge =
        Regex::new(r"img\.shields\.io/badge/Platforms-(.*)-[A-Za-z]*\.svg").expect("static regex");
    let encoded = badge.captures(readme)?.get(1)?.as_str();
    let decoded = encoded
        .replace("%20%7C%20", " ")
        .replace("%20", " ")
        .replace("%7C", " ")
        .replace("--", "-");
    Some(decoded.split_whitespace().map(str::to_string).collect())
}

// ---------------------------------------------------------------- 3. version references

/// task-2 follow-up: two regimes, one predicate (`release_refs`, shared with the fix in
/// `prepare-release`, so the reporter and the fixer cannot disagree). Plus 3b: the conda
/// package manifest is the one file whose format demands a restated literal version, so it
/// must equal Cargo.toml's — it sat at 0.2.0 for two releases while the workspace said 0.3.2,
/// and a `pixi publish` would have shipped a wrongly-versioned .conda with no red build.
fn version_references(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let workspace_version = crate::version::workspace_version(root)?;
    match crate::release_refs::scan(root) {
        Err(error) => failures.push(Failure::new(format!("the release-reference scan failed: {error:#}"))),
        Ok(findings) if !findings.is_empty() => failures.push(Failure::with(
            "documentation pins a version the manifests do not declare (task-2):",
            findings,
            format!(
                "README-family drift: run 'pixi run prepare-release v{workspace_version}' (or mark the line stale-ref-allowed); docs/ literals: replace the tag with v__VERSION__ — the site derives the version at build time"
            ),
        )),
        Ok(_) => {}
    }

    let conda_manifest = root.join("crates/pixi-sandbox/pixi.toml");
    let conda_version = crate::util::read(&conda_manifest)?.lines().find_map(|l| {
        l.strip_prefix("version = \"")
            .and_then(|rest| rest.split('"').next())
            .map(str::to_string)
    });
    match conda_version {
        None => failures.push(Failure::new(
            "crates/pixi-sandbox/pixi.toml has no parsable [package] version — 'pixi publish' would refuse or guess",
        )),
        Some(conda_version) if conda_version != workspace_version => failures.push(Failure::with(
            format!(
                "crates/pixi-sandbox/pixi.toml declares {conda_version} but Cargo.toml declares {workspace_version} — the published .conda would carry the wrong version"
            ),
            Vec::new(),
            format!(
                "prepare-release stamps this file; for a manual fix set version = \"{workspace_version}\" in crates/pixi-sandbox/pixi.toml"
            ),
        )),
        Some(_) => {}
    }
    Ok(())
}

// ---------------------------------------------------------------- 4. action pins

/// Every third-party `uses:` is a full commit SHA plus a trailing release label, so a
/// compromised or force-moved tag cannot change what CI runs. actionlint validates workflow
/// syntax and inputs but says nothing about what a `uses:` resolves to, so this is the only
/// thing standing between a mutable tag and the release pipeline.
///
/// Same-repository actions (`./setup`, `../action.yml`) are not third-party and carry no ref
/// to pin. A deliberate exception opts out with the marker on the line.
fn action_pins(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let label = Regex::new(r"#[\t ]*v[0-9]").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if line.trim_start().starts_with('#') {
                continue;
            }
            let Some(uses_at) = line.find("uses:") else {
                continue;
            };
            let spec = line[uses_at + "uses:".len()..]
                .trim_start()
                .split(|c: char| c.is_whitespace() || c == '#')
                .next()
                .unwrap_or("");
            if spec.starts_with("./") || spec.starts_with("../") || spec.starts_with("docker://") {
                continue;
            }
            let Some((_, reference)) = spec.split_once('@') else {
                continue;
            };
            let after_spec = &line[line.find(spec).map_or(line.len(), |at| at + spec.len())..];
            let why = if reference.len() != 40
                || !reference
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            {
                Some(format!(
                    "pins {reference}, a mutable ref, not a 40-character commit SHA"
                ))
            } else if !label.is_match(after_spec) {
                Some("pins a SHA with no trailing release label, so the next person cannot tell what it is".to_string())
            } else {
                None
            };
            if let Some(why) = why {
                findings.push(format!("{}:{number}:{why}", rel(root, &file)));
            }
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a third-party action is not pinned to a full commit SHA (see .knowledge/publish-automation.md):",
            findings,
            "resolve the tag with 'gh api repos/OWNER/REPO/git/ref/tags/TAG', dereference it if it is an annotated tag, and pin it as owner/repo@<sha> # vX.Y.Z",
        ));
    }
    Ok(())
}

fn workflow_files(root: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = walkdir::WalkDir::new(root.join(".github/workflows"))
        .into_iter()
        .flatten()
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| p.extension().is_some_and(|x| x == "yml" || x == "yaml"))
        .collect();
    files.sort();
    files
}

// ---------------------------------------------------------------- 5. install one-liner

/// The `install.sh` release asset is RENDERED from templates/install.sh at the tag, not
/// copied from a committed script (task-2 — the v0.3.0 asset shipped a v0.2.0 default). The
/// render itself is a tested pure function now, so what is left to check here is the
/// repository's half of the bargain: the template still renders for the declared version,
/// and no committed, ready-to-run copy exists anywhere to drift.
fn install_template(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let template_path = root.join("templates/install.sh");
    if !template_path.is_file() {
        failures.push(Failure::new(
            "templates/install.sh is missing — release.yml renders the install.sh asset from it",
        ));
    } else {
        let template = crate::util::read(&template_path)?;
        let declared = format!("v{}", crate::version::workspace_version(root)?);
        if let Err(error) = crate::install_template::render(&template, &declared) {
            failures.push(Failure::new(format!(
                "templates/install.sh does not render for the declared release ({declared}): {error:#}"
            )));
        }
    }
    if root.join("scripts/install.sh").exists() || root.join("init.sh").exists() {
        failures.push(Failure::new(
            "a committed install.sh exists outside templates/ — it would ship a hardcoded VERSION default that lags the release (task-2)",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- 6. no hardcoded release tags in CI

/// A workflow that names a specific `vX.Y.Z` of ours is stale the moment a release lands,
/// and it fails silently: the job keeps running against old binaries and still reports green.
/// Scoped to a `version:` value, so an example in a description is not a false positive.
fn workflow_literal_tags(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let version_key = Regex::new(r"(^|[\t ])version:[\t ]").expect("static regex");
    let mut findings = Vec::new();
    for file in workflow_files(root) {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if !version_key.is_match(line) || line.contains("description:") {
                continue;
            }
            for tag in version_tag_re().find_iter(line) {
                findings.push(format!(
                    "{}:{number}: pins the literal {}",
                    rel(root, &file),
                    tag.as_str()
                ));
            }
        }
    }
    if !findings.is_empty() {
        failures.push(Failure::with(
            "a workflow pins a literal release tag, so it will silently prove a stale release after the next cut:",
            findings,
            "read the tag from the checked-out Cargo.toml, or take it from a repository variable",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------- 7. self-contained actions

/// task-12 / issue #37: v0.3.1 published two action paths that could not run, because a
/// relative `uses:` inside a *remote* composite action does not resolve against the
/// repository that defines it. Each published path must be a complete composite action, and
/// the two generated copies must still re-render byte-identically from the root.
fn published_actions(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    for action in ["action.yml", "setup/action.yml", "publish/action.yml"] {
        let path = root.join(action);
        if !path.is_file() {
            failures.push(Failure::new(format!(
                "{action} is missing — it is a published action path (owner/repo[/path]@ref) (task-12)"
            )));
            continue;
        }
        let text = crate::util::read(&path)?;
        if let Some(relative) = crate::action_shims::relative_uses(&text) {
            failures.push(Failure::with(
                format!("{action} uses a relative action reference, which cannot resolve from a remote ref (task-12):"),
                vec![relative],
                "a published action path must be self-contained; edit action.yml and run 'pixi run render-action-shims'",
            ));
        }
        if !text.lines().any(|l| l == "  using: composite") {
            failures.push(Failure::new(format!(
                "{action} is not a composite action — a published action path must run on its own (task-12)"
            )));
        }
    }

    let root_action_path = root.join("action.yml");
    if root_action_path.is_file() {
        let root_action = crate::util::read(&root_action_path)?;
        for variant in [
            crate::action_shims::Variant::Setup,
            crate::action_shims::Variant::Publish,
        ] {
            let generated_path = root.join(format!("{variant}/action.yml"));
            if !generated_path.is_file() {
                continue; // already reported above
            }
            match crate::action_shims::render(&root_action, variant) {
                Err(error) => failures.push(Failure::new(format!(
                    "cannot render {variant}/action.yml from action.yml (task-12): {error:#}"
                ))),
                Ok(rendered) => {
                    let committed = crate::util::read(&generated_path)?;
                    if committed != rendered {
                        let first_diff = committed
                            .lines()
                            .zip(rendered.lines())
                            .position(|(a, b)| a != b)
                            .map(|at| at + 1)
                            .unwrap_or_else(|| {
                                committed.lines().count().min(rendered.lines().count()) + 1
                            });
                        failures.push(Failure::with(
                            format!("{variant}/action.yml has drifted from action.yml (task-12):"),
                            vec![format!("first difference at line {first_diff}")],
                            "regenerate with 'pixi run render-action-shims'",
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------- 8. Bash 3.2 for the macOS runners

/// The scripts that remain shell (`restore.sh` bootstraps hosts that have nothing but sh and
/// git; `airlock-gate.sh` runs where no toolchain may be assumed) are invoked with whatever
/// `bash` the host has, and on the GitHub macOS runners that is Apple's /bin/bash 3.2 — Bash
/// 4 never shipped (GPLv3). v0.3.6's release run 36865921206 exited 127 on both darwin legs
/// over one Bash-4 builtin; Linux (bash 5) and Windows (Git bash 5) passed, so macOS is the
/// only leg that can ever see this failure — the blind spot this check closes. actionlint
/// cannot help: the construct sits one invocation layer down, inside the script a step calls.
fn bash32_surface(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    // The forbidden set is the Bash-4 surface people actually reach for: the array-read
    // builtins and coprocesses, associative arrays, case conversion, and `&>>`.
    let bash4 = Regex::new(
        r"(^|[^[:alnum:]_])(mapfile|readarray|coproc)([^[:alnum:]_]|$)|(declare|typeset)[\t ]+-[A-Za-z]*A([\t ]|$)|\$\{[A-Za-z_][A-Za-z0-9_]*(,,|\^\^)|&>>",
    )
    .expect("static regex");
    let mut hits = Vec::new();
    let scripts_dir = root.join("scripts");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&scripts_dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().is_some_and(|x| x == "sh"))
                .collect()
        })
        .unwrap_or_default();
    scripts.sort();
    for file in scripts {
        let text = crate::util::read(&file)?;
        for (number, line) in lines_without_opt_out(&text) {
            if bash4.is_match(line) {
                hits.push(format!("{}:{number}:{line}", rel(root, &file)));
            }
        }
    }
    if !hits.is_empty() {
        failures.push(Failure::with(
            "a script uses a Bash-4-only construct (array-read builtins, declare -A, case conversion, &>>), which macOS's /bin/bash 3.2 cannot run (v0.3.6 run 36865921206):", // stale-ref-allowed
            hits,
            "rewrite it with 3.2-compatible builtins (e.g. a while-read process-substitution loop instead of the array builtin) — or better, move the logic into this xtask",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check_repository;
    use std::fs;
    use std::path::Path;

    /// A minimal repository that passes every check — each test then breaks exactly one
    /// policy and asserts that policy alone fires. Built in a tempdir: D10 forbids these
    /// tests from ever looking at the real checkout.
    fn valid_fixture() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        let write = |rel: &str, text: &str| {
            let path = root.join(rel);
            fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
            fs::write(path, text).expect("file");
        };

        write("Cargo.toml", "[workspace.package]\nversion = \"1.0.0\"\n");
        write(
            "pixi.toml",
            "[workspace]\nname = \"fixture\"\nplatforms = [\"linux-64\", \"osx-arm64\"]\n",
        );
        write(
            ".pixi-sandbox.toml",
            "schema = 1\n\n[[bundle]]\nname = \"developer\"\nplatforms = [\"linux-64\"]\n",
        );
        write(
            "README.md",
            "<img src=\"https://img.shields.io/badge/Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg\" alt=\"Platforms\">\n\nwin-64 is unsupported: conda-forge ships no bun build (D11).\n",
        );
        write(
            "crates/pixi-sandbox/pixi.toml",
            "[package]\nname = \"fixture\"\nversion = \"1.0.0\"\n",
        );
        write("crates/pixi-sandbox/src/lib.rs", "// clean\n");
        write(
            "templates/install.sh",
            "#!/bin/sh\nVERSION=${PIXI_SANDBOX_VERSION:-__VERSION__}\necho \"$VERSION\"\n",
        );
        write(
            ".github/workflows/ci.yml",
            "jobs:\n  ci:\n    steps:\n      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1\n",
        );
        write(
            "scripts/restore.sh",
            "#!/usr/bin/env bash\nset -euo pipefail\necho restore\n",
        );

        // A root action plus generated copies rendered by the same renderer the check uses.
        let root_action = "name: fixture\ndescription: root\n\ninputs:\n  subpath:\n    required: false\n    default: setup\n  repository:\n    description: where\n    default: Example/widget\nruns:\n  using: composite\n  steps:\n    - shell: bash\n      run: echo hi\n";
        write("action.yml", root_action);
        for variant in [
            crate::action_shims::Variant::Setup,
            crate::action_shims::Variant::Publish,
        ] {
            let rendered = crate::action_shims::render(root_action, variant).expect("render");
            write(&format!("{variant}/action.yml"), &rendered);
        }
        dir
    }

    fn headlines(root: &Path) -> Vec<String> {
        check_repository(root)
            .expect("checks run")
            .into_iter()
            .map(|f| f.headline)
            .collect()
    }

    #[test]
    fn the_valid_fixture_passes_every_check() {
        let dir = valid_fixture();
        assert_eq!(headlines(dir.path()), Vec::<String>::new());
    }

    #[test]
    fn a_stale_reference_under_crates_fires_check_1_and_the_marker_silences_it() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("crates/pixi-sandbox/src/old.rs"),
            // stale-ref-allowed — fixture content must name a forbidden word to test the check.
            "// see the Python prototype\n",
        )
        .expect("file");
        let found = headlines(dir.path());
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].contains("task-6"), "{found:?}");

        fs::write(
            dir.path().join("crates/pixi-sandbox/src/old.rs"),
            "// stale-ref-allowed — documents history\n// see the Python prototype\n",
        )
        .expect("file");
        assert_eq!(headlines(dir.path()), Vec::<String>::new());
    }

    #[test]
    fn a_badge_that_disagrees_with_the_workspace_fires_check_2() {
        let dir = valid_fixture();
        let readme = fs::read_to_string(dir.path().join("README.md")).expect("readme");
        fs::write(
            dir.path().join("README.md"),
            readme.replace("osx--arm64", "win--64"),
        )
        .expect("readme");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("Platforms badge advertises")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|h| h.contains("claims win-64")),
            "{found:?}"
        );
    }

    #[test]
    fn a_published_platform_the_workspace_lacks_fires_check_2() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join(".pixi-sandbox.toml"),
            "schema = 1\n\n[[bundle]]\nname = \"developer\"\nplatforms = [\"linux-aarch64\"]\n",
        )
        .expect("plan");
        let found = headlines(dir.path());
        assert!(
            found.iter().any(|h| h.contains("publishes linux-aarch64")),
            "{found:?}"
        );
    }

    #[test]
    fn a_conda_manifest_version_mismatch_fires_check_3b() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("crates/pixi-sandbox/pixi.toml"),
            "[package]\nname = \"fixture\"\nversion = \"0.9.0\"\n",
        )
        .expect("manifest");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("declares 0.9.0 but Cargo.toml declares 1.0.0")),
            "{found:?}"
        );
    }

    #[test]
    fn a_stale_documented_release_reference_fires_check_3() {
        let dir = valid_fixture();
        let readme = fs::read_to_string(dir.path().join("README.md")).expect("readme");
        fs::write(
            dir.path().join("README.md"),
            format!("{readme}\nuses: Example/widget/setup@v0.1.0\n"),
        )
        .expect("readme");
        let found = headlines(dir.path());
        assert!(found.iter().any(|h| h.contains("task-2")), "{found:?}");
    }

    #[test]
    fn a_mutable_tag_or_an_unlabelled_sha_fires_check_4() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join(".github/workflows/bad.yml"),
            "jobs:\n  x:\n    steps:\n      - uses: actions/checkout@v7\n      - uses: actions/cache@3d3c42e5aac5ba805825da76410c181273ba90b1\n",
        )
        .expect("workflow");
        let failures = check_repository(dir.path()).expect("checks");
        let pins = failures
            .iter()
            .find(|f| f.headline.contains("not pinned to a full commit SHA"))
            .expect("pin failure");
        assert_eq!(pins.details.len(), 2, "{:?}", pins.details);
        assert!(pins.details[0].contains("mutable ref"));
        assert!(pins.details[1].contains("no trailing release label"));
    }

    #[test]
    fn a_missing_template_or_committed_installer_fires_check_5() {
        let dir = valid_fixture();
        fs::remove_file(dir.path().join("templates/install.sh")).expect("rm");
        fs::write(dir.path().join("init.sh"), "#!/bin/sh\n").expect("init");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("templates/install.sh is missing")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|h| h.contains("outside templates/")),
            "{found:?}"
        );
    }

    #[test]
    fn a_literal_release_tag_in_a_workflow_fires_check_6() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join(".github/workflows/proof.yml"),
            "jobs:\n  x:\n    steps:\n      - with:\n          version: ${{ vars.RELEASE || 'v0.3.0' }}\n",
        )
        .expect("workflow");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("pins a literal release tag")),
            "{found:?}"
        );
    }

    #[test]
    fn a_drifted_or_relative_generated_action_fires_check_7() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("setup/action.yml"),
            "name: stale copy\nruns:\n  using: composite\n  steps:\n    - uses: ../action.yml\n",
        )
        .expect("action");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("relative action reference")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|h| h.contains("drifted from action.yml")),
            "{found:?}"
        );
    }

    #[test]
    fn a_bash4_builtin_in_a_surviving_script_fires_check_8() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join("scripts/gate.sh"),
            "#!/usr/bin/env bash\nmapfile -t lines < <(ls)\n",
        )
        .expect("script");
        let found = headlines(dir.path());
        assert!(
            found.iter().any(|h| h.contains("Bash-4-only construct")),
            "{found:?}"
        );
    }
}
