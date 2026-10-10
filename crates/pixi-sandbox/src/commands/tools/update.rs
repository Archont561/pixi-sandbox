//! `tools update` — refresh the helper-tool pins from the same upstream the pins name.
//!
//! The catalogue is reviewed data (D4), so this command never *installs* anything and never
//! decides that a newer build is better. It produces a candidate lock for a human to diff.
//! What it does insist on is that every hash it writes is earned:
//!
//! * **Cross-check against the upstream manifest when the project publishes one.** pixi ships a
//!   `sha256.sum` per release, so a corrupted download is caught against a second, independently
//!   published source. Measured: Quantco/pixi-pack publishes **no** checksum manifest at all, so
//!   there the hash is trust-on-first-use and the lock records that fact in `note` rather than
//!   pretending to a guarantee it does not have.
//! * **Verify linkage from the bytes, not from the previous lock.** A pin is allowed to claim
//!   `static`; this command reads the ELF and refuses if the asset is not. That is the check that
//!   catches a `~/.pixi/bin` trampoline (a 766 KiB dynamic shim that works on the build machine
//!   and dies in the airlock) at pin time instead of at restore time.
//!
//! The write is atomic and happens only after every asset in the catalogue has been checked, so a
//! failure anywhere leaves the operator's lock byte-for-byte unchanged (AC#2).

use crate::cli::ToolsUpdateArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::release::{GitHubReleaseSource, PIXI_CHECKSUM_MANIFEST, ReleaseSource};
use pixi_sandbox_core::tools_lock::{PlatformPin, Tool, ToolsLock};
use pixi_sandbox_core::verify::{Linkage, linkage_of_bytes};
use std::collections::BTreeMap;
use std::path::Path;

/// The release asset name: the last path segment of a fully substituted download URL.
pub fn asset_name_of(url: &str) -> Option<&str> {
    let name = url.rsplit('/').next()?;
    if name.is_empty() || name.contains('{') {
        return None;
    }
    Some(name)
}

/// The `owner/repo` a pin downloads from, read out of its own URL template.
///
/// The template is the single source of truth for *where* a build came from (AC#1): the same
/// source the existing pin was compiled from, never a hardcoded repository somewhere in this
/// file that could drift away from the data it is meant to serve.
pub fn github_repo(url_template: &str) -> Option<&str> {
    let rest = url_template.strip_prefix("https://github.com/")?;
    let mut parts = rest.split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    // A GitHub release asset is always at /owner/repo/releases/download/<tag>/<name>. Requiring
    // the literal `releases` segment keeps a repo-less path from being read as a repository
    // called "releases" — which would resolve "latest" against the wrong project entirely.
    if parts.next()? != "releases" {
        return None;
    }
    if owner.is_empty() || repo.is_empty() || owner.contains('{') || repo.contains('{') {
        return None;
    }
    Some(&rest[..owner.len() + 1 + repo.len()])
}

/// `generated_at` and the `note` on every pin we touch are the audit trail: the next reader must
/// be able to tell a hash that was cross-checked from one that was merely observed.
pub fn run(args: ToolsUpdateArgs) -> Result<()> {
    let target = match &args.tools_lock {
        Some(path) => {
            let path = support::absolute(path)?;
            let lock = ToolsLock::load(&path)
                .with_context(|| format!("reading tool pins {}", path.display()))?;
            (lock, Some(path))
        }
        None => (
            ToolsLock::embedded().context("loading embedded helper-tool pins")?,
            None,
        ),
    };
    let (lock, path) = target;

    let source = GitHubReleaseSource::new();
    let (updated, report) = refresh(&lock, &source, &args.tool)?;

    if args.check {
        return report_check(&report);
    }

    let rendered = render(&updated)?;
    match path {
        // No `--tools-lock`: print the candidate so a reviewer can redirect it into the asset
        // file and diff it. The embedded copy is compiled into the binary, so there is nothing
        // beside the executable to write — and nothing to corrupt by accident.
        None => {
            print!("{rendered}");
            eprintln!(
                "\n{} tool(s) checked. Redirect this into crates/pixi-sandbox-core/assets/tools.lock.json and review the diff.",
                report.checked
            );
            Ok(())
        }
        Some(path) => write_atomic(&path, &rendered).context("writing the updated tools lock"),
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Report {
    /// (tool, platform, `old_version`, `new_version`) for every asset that changed.
    changed: Vec<(String, String, String, String)>,
    /// Assets whose version and hash were already current.
    unchanged: usize,
    checked: usize,
    skipped: Vec<String>,
}

impl Report {
    fn is_change(&self) -> bool {
        !self.changed.is_empty()
    }
}

fn report_check(report: &Report) -> Result<()> {
    for (tool, platform, from, to) in &report.changed {
        println!("update {tool}/{platform}: {from} -> {to}");
    }
    for note in &report.skipped {
        println!("skipped {note}");
    }
    println!(
        "{} asset(s) checked, {} unchanged, {} to update",
        report.checked,
        report.unchanged,
        report.changed.len()
    );
    if report.is_change() {
        // Non-zero is the point: this is what a scheduled workflow keys on.
        bail!("tool pins are out of date; run `pixi-sandbox tools update`");
    }
    Ok(())
}

/// Rebuild the catalogue against the newest releases, verifying every asset on the way.
fn refresh(
    lock: &ToolsLock,
    source: &dyn ReleaseSource,
    only: &[String],
) -> Result<(ToolsLock, Report)> {
    let mut updated = lock.clone();
    let mut report = Report::default();

    for name in lock.names() {
        if !only.is_empty() && !only.iter().any(|wanted| wanted == name) {
            report.skipped.push(format!("{name} (not selected)"));
            continue;
        }
        let tool = &lock.tools[name];
        let Some(repo) = github_repo(&tool.url_template) else {
            // An organisation mirror's URL is not a GitHub release path, so there is no
            // "latest" to resolve. Leaving the pin untouched is the safe reading: we cannot
            // know what a mirror's newest build is, and guessing would replace reviewed data.
            report.skipped.push(format!(
                "{name}: url_template is not a GitHub release URL, so the latest version cannot be \
                 resolved; keep this pin or point --tools-lock at a GitHub-hosted mirror"
            ));
            continue;
        };

        let tag = source
            .latest_tag(repo)
            .with_context(|| format!("resolving the latest {name} release from {repo}"))?;
        let version = tag.trim_start_matches('v').to_string();
        if version == tool.version {
            report.unchanged += tool.platforms.len();
            report.checked += tool.platforms.len();
            continue;
        }

        let checksums = source
            .published_checksums(repo, &tag)
            .with_context(|| format!("reading published checksums for {repo} {tag}"))?;
        let mut platforms = BTreeMap::new();

        for platform in tool.platforms.keys() {
            let previous = &tool.platforms[platform];
            let url = tool
                .url_template
                .replace("{version}", &version)
                .replace("{target}", &previous.target);
            // The release asset is the *file name* of the substituted URL, which the template
            // builds as e.g. `pixi-x86_64-unknown-linux-musl`. Deriving it from the template
            // rather than from `target` alone is what keeps the tool prefix in the name; using
            // the target directly 404s against real GitHub.
            let asset_name = asset_name_of(&url).with_context(|| {
                format!("the url_template for {name} has no file name to download: {url}")
            })?;
            let asset = source
                .asset(repo, &tag, asset_name)
                .with_context(|| format!("downloading {name} {version} for {platform}"))?;
            report.checked += 1;

            let actual = asset.sha256();
            let published = checksums.as_ref().and_then(|map| map.get(asset_name));
            let note = match (checksums.as_ref(), published) {
                (Some(_), Some(expected)) if expected != &actual => bail!(
                    "integrity: {name} {version} for {platform} hashes to {actual}, but {repo} \
                     publishes {expected} for {asset_name} in {PIXI_CHECKSUM_MANIFEST}. Refusing \
                     to write the lock; the download or the upstream manifest is not trustworthy."
                ),
                (_, Some(_)) => {
                    format!("cross-checked against {PIXI_CHECKSUM_MANIFEST} in {repo} {tag}")
                }
                // Measured 2026-09-29: pixi publishes `sha256.sum` covering only the archives
                // (`pixi-x86_64-unknown-linux-musl.tar.gz`, `.zip`, `.msi`), while the catalogue
                // pins the *bare* binary that is not listed. Saying "cross-checked" there would be
                // a claim no one verified, so the weaker guarantee is written into the pin.
                (Some(_), None) => format!(
                    "{PIXI_CHECKSUM_MANIFEST} in {repo} {tag} does not list {asset_name} (it \
                     covers archives); hash observed and linkage verified locally, not \
                     cross-checked"
                ),
                (None, _) => format!(
                    "no upstream checksum manifest; hash observed on {tag} and linkage verified \
                     locally, not cross-checked"
                ),
            };

            let linkage = linkage_of_bytes(&asset.bytes);
            if previous.linkage == Linkage::Static.as_str() && linkage != Linkage::Static {
                bail!(
                    "integrity: {name} {version} for {platform} is a {} binary, but the pin \
                     declares it static ({url}). A dynamic helper works on the build machine and \
                     dies in the airlock (D4) — this is the failure the pin exists to prevent. \
                     Refusing to write the lock.",
                    linkage.as_str()
                );
            }

            report.changed.push((
                name.to_string(),
                platform.clone(),
                tool.version.clone(),
                version.clone(),
            ));
            platforms.insert(
                platform.clone(),
                PlatformPin {
                    target: previous.target.clone(),
                    sha256: actual,
                    // Write what was observed. A pin that can carry a stale `linkage` claim is a
                    // pin that will lie to whoever reads it next.
                    linkage: linkage.as_str().to_string(),
                    note: Some(note),
                },
            );
        }

        updated.tools.insert(
            name.to_string(),
            Tool {
                version,
                url_template: tool.url_template.clone(),
                platforms,
            },
        );
    }

    if report.is_change() {
        updated.generated_at = Some(support::now_rfc3339());
    }
    Ok((updated, report))
}

fn render(lock: &ToolsLock) -> Result<String> {
    let mut text = serde_json::to_string_pretty(lock).context("serialising the tools lock")?;
    text.push('\n');
    Ok(text)
}

/// Write via a sibling temp file and rename, so a reader never sees a half-written lock and a
/// failure never truncates the existing one (invariant 1: verify before write).
fn write_atomic(path: &Path, contents: &str) -> Result<()> {
    let temporary = path.with_file_name(format!(
        ".{}.update-{}",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("tools.lock.json"),
        std::process::id()
    ));
    let result = (|| -> Result<()> {
        std::fs::write(&temporary, contents)
            .with_context(|| format!("writing {}", temporary.display()))?;
        std::fs::rename(&temporary, path)
            .with_context(|| format!("moving the new tools lock into place at {}", path.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// A scratch catalogue for the unit tests: one tool, one platform, already current.
///
/// Written as a literal rather than copied from the real asset so a change to the shipped pins
/// cannot silently change what these tests are about.
#[cfg(test)]
fn scratch_lock(version: &str, sha256: &str) -> ToolsLock {
    serde_json::from_str(&format!(
        r#"{{"schema":1,"tools":{{"pixi":{{"version":"{version}",
        "url_template":"https://github.com/prefix-dev/pixi/releases/download/v{{version}}/pixi-{{target}}",
        "platforms":{{"linux-64":{{"target":"x86_64-unknown-linux-musl","sha256":"{sha256}",
        "linkage":"static"}}}}}}}}}}"#
    ))
    .expect("scratch lock must parse")
}

/// A minimal 64-bit `x86_64` ELF: identification header, one program header, nothing else.
///
/// The program header is what the linkage test reads, so a synthetic file has to contain one —
/// a bare 64-byte header has none, and a 64-byte file with `e_phoff = 64` reads as `Unknown`
/// rather than static. A test fixture that quietly produced `Unknown` would have let the
/// dynamic-asset refusal pass for the wrong reason.
#[cfg(test)]
const PHDR_OFFSET: usize = 64;
#[cfg(test)]
const PHDR_SIZE: usize = 56;

#[cfg(test)]
fn elf(program_type: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; PHDR_OFFSET + PHDR_SIZE];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2; // ELFCLASS64
    bytes[5] = 1; // ELFDATA2LSB
    bytes[6] = 1; // EV_CURRENT
    bytes[16..18].copy_from_slice(&2u16.to_le_bytes()); // e_type = ET_EXEC
    bytes[32..40].copy_from_slice(&(PHDR_OFFSET as u64).to_le_bytes()); // e_phoff
    bytes[52..54].copy_from_slice(&(PHDR_OFFSET as u16).to_le_bytes()); // e_ehsize
    bytes[54..56].copy_from_slice(&(PHDR_SIZE as u16).to_le_bytes()); // e_phentsize
    bytes[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
    bytes[PHDR_OFFSET..PHDR_OFFSET + 4].copy_from_slice(&program_type.to_le_bytes());
    bytes
}

/// `PT_LOAD`: no interpreter, so it runs on a bare machine.
#[cfg(test)]
fn static_elf() -> Vec<u8> {
    elf(1)
}

/// The same file with a `PT_INTERP` header — the `~/.pixi/bin` trampoline shape (D4).
#[cfg(test)]
fn dynamic_elf() -> Vec<u8> {
    elf(3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pixi_sandbox::release::{Asset, parse_sha256_manifest};

    struct FakeSource {
        latest: BTreeMap<String, String>,
        assets: BTreeMap<(String, String, String), Vec<u8>>,
        checksums: BTreeMap<(String, String), Option<BTreeMap<String, String>>>,
        requested: std::cell::RefCell<Vec<String>>,
    }

    impl FakeSource {
        fn new() -> Self {
            Self {
                latest: BTreeMap::new(),
                assets: BTreeMap::new(),
                checksums: BTreeMap::new(),
                requested: std::cell::RefCell::new(Vec::new()),
            }
        }

        fn with_latest(mut self, repo: &str, tag: &str) -> Self {
            self.latest.insert(repo.to_string(), tag.to_string());
            self
        }

        fn with_asset(mut self, repo: &str, tag: &str, name: &str, bytes: Vec<u8>) -> Self {
            self.assets
                .insert((repo.to_string(), tag.to_string(), name.to_string()), bytes);
            self
        }

        fn with_checksums(mut self, repo: &str, tag: &str, map: BTreeMap<String, String>) -> Self {
            self.checksums
                .insert((repo.to_string(), tag.to_string()), Some(map));
            self
        }
    }

    impl ReleaseSource for FakeSource {
        fn latest_tag(&self, repo: &str) -> Result<String> {
            self.latest
                .get(repo)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("no release for {repo}"))
        }

        fn asset(&self, repo: &str, tag: &str, name: &str) -> Result<Asset> {
            self.requested
                .borrow_mut()
                .push(format!("{repo}/{tag}/{name}"));
            let bytes = self
                .assets
                .get(&(repo.to_string(), tag.to_string(), name.to_string()))
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("no asset {name} in {repo} {tag}"))?;
            Ok(Asset { bytes })
        }

        fn published_checksums(
            &self,
            repo: &str,
            tag: &str,
        ) -> Result<Option<BTreeMap<String, String>>> {
            Ok(self
                .checksums
                .get(&(repo.to_string(), tag.to_string()))
                .cloned()
                .flatten())
        }
    }

    const REPO: &str = "prefix-dev/pixi";
    const OLD: &str = "0000000000000000000000000000000000000000000000000000000000000000";
    const NEW: &str = "1111111111111111111111111111111111111111111111111111111111111111";

    fn sha_of(bytes: &[u8]) -> String {
        pixi_sandbox_core::shard::sha256_bytes(bytes)
    }

    /// AC#1: the version comes from the release the template already names, and every asset's
    /// hash is replaced rather than kept.
    #[test]
    fn a_new_release_rewrites_the_version_and_every_hash() {
        let source = FakeSource::new().with_latest(REPO, "v0.82.0").with_asset(
            REPO,
            "v0.82.0",
            "pixi-x86_64-unknown-linux-musl",
            static_elf(),
        );
        let (updated, report) =
            refresh(&scratch_lock("0.81.0", OLD), &source, &[]).expect("refresh");

        let pin = updated.pin("pixi", "linux-64").expect("pin");
        assert_eq!(updated.tools["pixi"].version, "0.82.0");
        assert_eq!(pin.sha256, sha_of(&static_elf()));
        assert_ne!(pin.sha256, OLD, "a stale hash must not survive an update");
        assert_eq!(pin.linkage, "static");
        assert!(updated.generated_at.is_some(), "an update must be dated");
        assert_eq!(report.changed.len(), 1);
        assert_eq!(report.changed[0].2, "0.81.0");
        assert_eq!(report.changed[0].3, "0.82.0");
    }

    /// AC#2: a hash that disagrees with the project's own published manifest aborts the run.
    #[test]
    fn a_disagreement_with_the_upstream_manifest_aborts() {
        let mut map = BTreeMap::new();
        map.insert(
            "pixi-x86_64-unknown-linux-musl".to_string(),
            NEW.to_string(),
        );
        let source = FakeSource::new()
            .with_latest(REPO, "v0.82.0")
            .with_asset(
                REPO,
                "v0.82.0",
                "pixi-x86_64-unknown-linux-musl",
                static_elf(),
            )
            .with_checksums(REPO, "v0.82.0", map);

        let error = refresh(&scratch_lock("0.81.0", OLD), &source, &[])
            .expect_err("a manifest disagreement must not be written")
            .to_string();
        assert!(error.contains("Refusing to write the lock"), "got: {error}");
        assert!(error.contains("sha256.sum"), "got: {error}");
    }

    /// AC#2 and D4: the pin says static, the bytes say dynamic. This is the failure the whole
    /// catalogue exists to prevent, and it must be caught at pin time.
    #[test]
    fn a_dynamic_asset_under_a_static_pin_aborts() {
        let source = FakeSource::new().with_latest(REPO, "v0.82.0").with_asset(
            REPO,
            "v0.82.0",
            "pixi-x86_64-unknown-linux-musl",
            dynamic_elf(),
        );

        let error = refresh(&scratch_lock("0.81.0", OLD), &source, &[])
            .expect_err("a dynamic binary must not become a static pin")
            .to_string();
        assert!(error.contains("dynamic"), "got: {error}");
        assert!(error.contains("airlock"), "got: {error}");
    }

    /// The weaker guarantee has to be visible in the file. pixi-pack publishes no manifest, so
    /// claiming a cross-check there would be a lie written into reviewed data.
    #[test]
    fn a_tool_without_an_upstream_manifest_says_so_in_the_pin() {
        let source = FakeSource::new().with_latest(REPO, "v0.82.0").with_asset(
            REPO,
            "v0.82.0",
            "pixi-x86_64-unknown-linux-musl",
            static_elf(),
        );
        let (updated, _) = refresh(&scratch_lock("0.81.0", OLD), &source, &[]).expect("refresh");

        let note = updated
            .pin("pixi", "linux-64")
            .unwrap()
            .note
            .clone()
            .unwrap();
        assert!(
            note.contains("no upstream checksum manifest"),
            "got: {note}"
        );
    }

    /// This is the branch real pixi takes. Measured 2026-09-29 against v0.81.0: `sha256.sum`
    /// lists ten entries, all archives (`.tar.gz`, `.zip`, `.msi`, `install.sh`) plus
    /// `source.tar.gz` — the bare binary this catalogue pins is not among them. Recording
    /// "cross-checked" there would be an unearned claim in reviewed data, so the pin has to say
    /// the manifest exists and does not cover this asset.
    #[test]
    fn a_manifest_that_does_not_cover_the_pinned_asset_says_so() {
        let mut map = BTreeMap::new();
        map.insert(
            "pixi-x86_64-unknown-linux-musl.tar.gz".to_string(),
            NEW.to_string(),
        );
        let source = FakeSource::new()
            .with_latest(REPO, "v0.82.0")
            .with_asset(
                REPO,
                "v0.82.0",
                "pixi-x86_64-unknown-linux-musl",
                static_elf(),
            )
            .with_checksums(REPO, "v0.82.0", map);
        let (updated, _) = refresh(&scratch_lock("0.81.0", OLD), &source, &[]).expect("refresh");

        let pin = updated.pin("pixi", "linux-64").unwrap();
        let note = pin.note.clone().unwrap();
        assert!(note.contains("does not list"), "got: {note}");
        assert!(note.contains("not cross-checked"), "got: {note}");
        assert!(!note.contains("cross-checked against"), "got: {note}");
        // The hash is still recorded — it just is not advertised as verified.
        assert_eq!(pin.sha256, sha_of(&static_elf()));
    }

    #[test]
    fn a_matching_upstream_manifest_is_recorded_as_a_cross_check() {
        let mut map = BTreeMap::new();
        map.insert(
            "pixi-x86_64-unknown-linux-musl".to_string(),
            sha_of(&static_elf()),
        );
        let source = FakeSource::new()
            .with_latest(REPO, "v0.82.0")
            .with_asset(
                REPO,
                "v0.82.0",
                "pixi-x86_64-unknown-linux-musl",
                static_elf(),
            )
            .with_checksums(REPO, "v0.82.0", map);
        let (updated, _) = refresh(&scratch_lock("0.81.0", OLD), &source, &[]).expect("refresh");

        let note = updated
            .pin("pixi", "linux-64")
            .unwrap()
            .note
            .clone()
            .unwrap();
        assert!(
            note.contains("cross-checked against sha256.sum"),
            "got: {note}"
        );
    }

    /// Idempotence is what makes `--check` usable in a schedule: an already-current catalogue
    /// must produce no change, not a rewritten file with a new timestamp.
    #[test]
    fn a_current_catalogue_is_left_alone() {
        let current = sha_of(&static_elf());
        let source = FakeSource::new().with_latest(REPO, "v0.81.0");
        let (updated, report) =
            refresh(&scratch_lock("0.81.0", &current), &source, &[]).expect("refresh");

        assert!(!report.is_change(), "nothing newer: {report:?}");
        assert_eq!(report.unchanged, 1);
        assert!(
            source.requested.borrow().is_empty(),
            "no download should happen"
        );
        assert!(
            updated.generated_at.is_none(),
            "no change means no new date"
        );
        assert_eq!(updated.pin("pixi", "linux-64").unwrap().sha256, current);
    }

    /// AC#4: a mirror's URL is not a GitHub release path, so the pin is kept rather than being
    /// replaced with a guess about what the mirror's newest build might be.
    #[test]
    fn a_mirror_url_is_kept_and_reported_rather_than_guessed_at() {
        let mut lock = scratch_lock("0.81.0", OLD);
        lock.tools.get_mut("pixi").unwrap().url_template =
            "https://mirror.example.internal/pixi/v{version}/pixi-{target}".to_string();
        let source = FakeSource::new();
        let (updated, report) = refresh(&lock, &source, &[]).expect("refresh");

        assert_eq!(updated.tools["pixi"].version, "0.81.0");
        assert_eq!(updated.pin("pixi", "linux-64").unwrap().sha256, OLD);
        assert_eq!(report.skipped.len(), 1);
        assert!(
            report.skipped[0].contains("--tools-lock"),
            "got: {:?}",
            report.skipped
        );
    }

    /// Invariant 1 applies to the lock itself: a failure must not leave a truncated or emptied
    /// file where a working catalogue used to be.
    #[test]
    fn a_failed_write_leaves_the_previous_lock_byte_for_byte() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tools.lock.json");
        let original = render(&scratch_lock("0.81.0", OLD)).unwrap();
        std::fs::write(&path, &original).unwrap();

        // A directory cannot be renamed over, so the rename fails after the temp file is written.
        let blocker = dir.path().join("blocker");
        std::fs::create_dir(&blocker).unwrap();
        let error =
            write_atomic(&blocker, "new contents").expect_err("rename onto a directory fails");
        assert!(
            error.to_string().contains("moving the new tools lock"),
            "got: {error}"
        );

        assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .filter(|name| name.contains("update-"))
            .collect();
        assert!(leftovers.is_empty(), "temp file left behind: {leftovers:?}");
    }

    #[test]
    fn a_check_run_fails_only_when_something_is_newer() {
        let up_to_date = Report {
            unchanged: 1,
            checked: 1,
            ..Report::default()
        };
        assert!(report_check(&up_to_date).is_ok());
        assert!(up_to_date.skipped.is_empty());

        let stale = Report {
            changed: vec![(
                "pixi".into(),
                "linux-64".into(),
                "0.81.0".into(),
                "0.82.0".into(),
            )],
            checked: 1,
            ..Report::default()
        };
        let error = report_check(&stale)
            .expect_err("a stale catalogue must fail a check")
            .to_string();
        assert!(error.contains("out of date"), "got: {error}");
    }

    /// The release asset is the file name the template builds, not the bare target triple.
    /// Getting this wrong 404s against real GitHub, which is how it was found: the fake sources
    /// in these tests are keyed by the real asset name, so a regression fails here first.
    #[test]
    fn the_asset_name_keeps_the_prefix_the_template_builds() {
        assert_eq!(
            asset_name_of(
                "https://github.com/prefix-dev/pixi/releases/download/v0.82.0/pixi-x86_64-unknown-linux-musl"
            ),
            Some("pixi-x86_64-unknown-linux-musl")
        );
        assert_eq!(
            asset_name_of(
                "https://github.com/Quantco/pixi-pack/releases/download/v0.7.11/pixi-unpack-x86_64-pc-windows-msvc.exe"
            ),
            Some("pixi-unpack-x86_64-pc-windows-msvc.exe")
        );
        for unusable in [
            "https://github.com/prefix-dev/pixi/releases/download/v0.82.0/",
            "https://github.com/prefix-dev/pixi/releases/download/v0.82.0/pixi-{target}",
        ] {
            assert_eq!(
                asset_name_of(unusable),
                None,
                "{unusable} is not a file name"
            );
        }
    }

    #[test]
    fn the_repo_is_read_out_of_the_url_template() {
        assert_eq!(
            github_repo(
                "https://github.com/prefix-dev/pixi/releases/download/v{version}/pixi-{target}"
            ),
            Some("prefix-dev/pixi")
        );
        assert_eq!(
            github_repo("https://github.com/Quantco/pixi-pack/releases/download/v{version}/x"),
            Some("Quantco/pixi-pack")
        );
        for not_github in [
            "https://mirror.example.internal/pixi/v{version}/pixi-{target}",
            "https://github.com/prefix-dev/releases/download/v{version}/pixi-{target}",
            "https://github.com/prefix-dev/{repo}/releases/download/v{version}/x",
        ] {
            assert_eq!(
                github_repo(not_github),
                None,
                "{not_github} must not resolve"
            );
        }
    }

    #[test]
    fn both_checksum_manifest_spellings_parse() {
        let body = "\
1111111111111111111111111111111111111111111111111111111111111111  pixi-x86_64-unknown-linux-musl
2222222222222222222222222222222222222222222222222222222222222222  install.sh
3333333333333333333333333333333333333333333333333333333333333333 other-aarch64-apple-darwin
not a checksum line
";
        let map = parse_sha256_manifest(body);
        assert_eq!(map.len(), 3, "{map:?}");
        assert_eq!(
            map["pixi-x86_64-unknown-linux-musl"],
            "1111111111111111111111111111111111111111111111111111111111111111"
        );
        assert_eq!(
            map["other-aarch64-apple-darwin"],
            "3333333333333333333333333333333333333333333333333333333333333333"
        );

        let swapped = parse_sha256_manifest(
            "pixi-x86_64-apple-darwin 4444444444444444444444444444444444444444444444444444444444444444\n",
        );
        assert_eq!(swapped.len(), 1);
        assert_eq!(
            swapped["pixi-x86_64-apple-darwin"],
            "4444444444444444444444444444444444444444444444444444444444444444"
        );
    }
}
