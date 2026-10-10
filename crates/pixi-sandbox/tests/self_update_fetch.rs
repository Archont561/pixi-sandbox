//! Tests for `self_update::fetch_release` — the bootstrap download the generated workflows
//! run before any pixi-sandbox binary of their own exists on the runner (TASK-76).
//!
//! The contract that separates it from `self_update::run`: there is no up-to-date shortcut.
//! The destination is not the running binary, so "the running binary already has this
//! version" says nothing about what the destination holds — every call resolves, downloads,
//! verifies and installs (decision-4 / D16: the bootstrap is the pinned, checksum-verified
//! binary, always).

use anyhow::Result;
use pixi_sandbox::release::{Asset, ReleaseSource};
use pixi_sandbox::self_update::replace::ReplaceStrategy;
use pixi_sandbox::self_update::{self, Request, SUMS_ASSET};
use rstest::rstest;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const LINUX_ASSET: &str = "pixi-sandbox-x86_64-unknown-linux-musl";
const PINNED_BINARY: &[u8] = b"the pinned 0.6.0 standalone binary";

/// A fake release, in memory. No socket is opened anywhere in this file.
struct FakeRelease {
    latest: Option<String>,
    assets: BTreeMap<(String, String), Vec<u8>>,
    requested: RefCell<Vec<String>>,
}

impl FakeRelease {
    fn new() -> Self {
        Self {
            latest: Some("v0.6.0".to_string()),
            assets: BTreeMap::new(),
            requested: RefCell::new(Vec::new()),
        }
    }

    fn with_asset(mut self, tag: &str, name: &str, bytes: &[u8]) -> Self {
        self.assets
            .insert((tag.to_string(), name.to_string()), bytes.to_vec());
        self
    }

    fn with_sums(self, tag: &str, entries: &[(&str, &[u8])]) -> Self {
        let body: String = entries
            .iter()
            .map(|(name, bytes)| {
                format!(
                    "{}  {name}\n",
                    pixi_sandbox_core::shard::sha256_bytes(bytes)
                )
            })
            .collect();
        self.with_asset(tag, SUMS_ASSET, body.as_bytes())
    }

    /// The complete, healthy v0.6.0 release.
    fn healthy() -> Self {
        Self::new()
            .with_asset("v0.6.0", LINUX_ASSET, PINNED_BINARY)
            .with_sums("v0.6.0", &[(LINUX_ASSET, PINNED_BINARY)])
    }

    fn downloads(&self) -> Vec<String> {
        self.requested.borrow().clone()
    }
}

impl ReleaseSource for FakeRelease {
    fn latest_tag(&self, repo: &str) -> Result<String> {
        self.latest
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no releases for {repo}"))
    }
    fn asset(&self, _repo: &str, tag: &str, name: &str) -> Result<Asset> {
        self.requested.borrow_mut().push(format!("{tag}/{name}"));
        let bytes = self
            .assets
            .get(&(tag.to_string(), name.to_string()))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("404: no asset {name} in {tag}"))?;
        Ok(Asset { bytes })
    }
    fn published_checksums(
        &self,
        _repo: &str,
        _tag: &str,
    ) -> Result<Option<BTreeMap<String, String>>> {
        Ok(None)
    }
}

/// A bootstrap destination: a fresh path that does not exist yet, the shape the generated
/// workflow's `fetch-release` step installs into.
struct Scratch {
    _dir: tempfile::TempDir,
    destination: PathBuf,
}

fn scratch() -> Scratch {
    let dir = tempfile::tempdir().expect("tempdir");
    Scratch {
        destination: dir.path().join("pixi-sandbox"),
        _dir: dir,
    }
}

fn request<'a>(
    destination: &'a Path,
    requested_version: Option<&'a str>,
    current_version: &'a str,
) -> Request<'a> {
    Request {
        repo: self_update::DEFAULT_REPO,
        requested_version,
        destination,
        current_version,
        host_os: "linux",
        host_arch: "x86_64",
        strategy: ReplaceStrategy::Unix,
    }
}

#[rstest]
fn fetch_release_installs_the_verified_asset_even_when_the_running_version_matches() {
    // The bootstrap case: the running binary already IS the pinned version, and the
    // destination holds nothing. `self_update::run` would call this "up to date" and write
    // nothing; fetch-release must download, verify and install regardless.
    let scratch = scratch();
    let source = FakeRelease::healthy();
    let request = request(&scratch.destination, Some("0.6.0"), "0.6.0");

    let applied = self_update::fetch_release(&source, &request).expect("fetched");

    assert_eq!(
        std::fs::read(&scratch.destination).expect("the destination holds the asset"),
        PINNED_BINARY
    );
    assert_eq!(
        applied.digest,
        pixi_sandbox_core::shard::sha256_bytes(PINNED_BINARY)
    );
    // SHA256SUMS is fetched before the binary: there is no point downloading bytes we have
    // no way to judge.
    assert_eq!(
        source.downloads(),
        vec![
            format!("v0.6.0/{SUMS_ASSET}"),
            format!("v0.6.0/{LINUX_ASSET}")
        ]
    );
}

#[rstest]
fn fetch_release_accepts_the_v_prefixed_spelling_of_the_pin() {
    let scratch = scratch();
    let source = FakeRelease::healthy();
    let request = request(&scratch.destination, Some("v0.6.0"), "0.6.0");

    let applied = self_update::fetch_release(&source, &request).expect("fetched");

    assert_eq!(applied.plan.resolved.tag, "v0.6.0");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        PINNED_BINARY
    );
}

#[rstest]
fn fetch_release_refuses_a_pixi_managed_destination_before_downloading_anything() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join(".pixi/envs/default");
    std::fs::create_dir_all(prefix.join("conda-meta")).expect("mkdir");
    let destination = prefix.join("bin/pixi-sandbox");
    std::fs::create_dir_all(destination.parent().expect("parent")).expect("mkdir");
    std::fs::write(&destination, b"pixi-managed").expect("write");

    let source = FakeRelease::healthy();
    let request = request(&destination, Some("0.6.0"), "0.6.0");

    let err = self_update::fetch_release(&source, &request).unwrap_err();
    let text = format!("{err:?}");
    assert!(
        text.contains("owned by pixi") || text.contains("conda"),
        "{text}"
    );
    assert!(
        source.downloads().is_empty(),
        "a refused destination costs no bytes: {:?}",
        source.downloads()
    );
    assert_eq!(
        std::fs::read(&destination).expect("read"),
        b"pixi-managed",
        "the refused destination is untouched"
    );
}

#[rstest]
fn fetch_release_refuses_a_checksum_mismatch_and_leaves_no_partial_file() {
    let scratch = scratch();
    // The release lists the digest of the real asset but serves different bytes.
    let source = FakeRelease::new()
        .with_asset("v0.6.0", LINUX_ASSET, b"tampered bytes")
        .with_sums("v0.6.0", &[(LINUX_ASSET, PINNED_BINARY)]);
    let request = request(&scratch.destination, Some("0.6.0"), "0.6.0");

    let err = self_update::fetch_release(&source, &request).unwrap_err();
    assert!(format!("{err:?}").contains("checksum mismatch"));
    assert!(
        !scratch.destination.exists(),
        "verify before write (invariant 1): a mismatch leaves no file at the destination"
    );
}

#[rstest]
fn fetch_release_refuses_a_release_whose_sums_omit_this_host() {
    let scratch = scratch();
    let source = FakeRelease::new()
        .with_asset("v0.6.0", LINUX_ASSET, PINNED_BINARY)
        .with_sums("v0.6.0", &[("pixi-sandbox-aarch64-apple-darwin", b"other")]);
    let request = request(&scratch.destination, Some("0.6.0"), "0.6.0");

    let err = self_update::fetch_release(&source, &request).unwrap_err();
    assert!(format!("{err:?}").contains("no entry for"));
    assert!(!scratch.destination.exists());
}

#[rstest]
fn fetch_release_reports_what_it_installed() {
    let scratch = scratch();
    let source = FakeRelease::healthy();
    let request = request(&scratch.destination, Some("0.6.0"), "0.6.0");

    let applied = self_update::fetch_release(&source, &request).expect("fetched");
    let lines = applied.fetch_report();

    let joined = lines.join("\n");
    assert!(joined.contains("fetch-release"), "{joined}");
    assert!(
        joined.contains(&scratch.destination.display().to_string()),
        "{joined}"
    );
    assert!(joined.contains("0.6.0"), "{joined}");
    assert!(joined.contains(LINUX_ASSET), "{joined}");
    assert!(
        joined.contains(&pixi_sandbox_core::shard::sha256_bytes(PINNED_BINARY)),
        "{joined}"
    );
    // The report must not claim an update happened: nothing was updated, the destination
    // was provisioned.
    assert!(!joined.contains("updated"), "{joined}");
}
