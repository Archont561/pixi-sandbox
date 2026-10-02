//! Unit tests for the self-update orchestration (`src/self_update/mod.rs`).
//!
//! The whole trust boundary — resolve, classify, download, verify, replace — against an
//! in-memory fake release. No socket is opened and no real binary is written (D10).

use anyhow::Result;
use pixi_sandbox::release::{Asset, ReleaseSource};
use pixi_sandbox::self_update::replace::ReplaceStrategy;
use pixi_sandbox::self_update::*;
use std::path::{Path, PathBuf};

use std::cell::RefCell;
use std::collections::BTreeMap;

const LINUX_ASSET: &str = "pixi-sandbox-x86_64-unknown-linux-musl";
const NEW_BINARY: &[u8] = b"the 0.5.0 binary";

/// A fake release, in memory. No socket is opened anywhere in this file.
struct FakeRelease {
    latest: Option<String>,
    assets: BTreeMap<(String, String), Vec<u8>>,
    requested: RefCell<Vec<String>>,
}

impl FakeRelease {
    fn new() -> Self {
        Self {
            latest: Some("v0.5.0".to_string()),
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

    /// The complete, healthy v0.5.0 release.
    fn healthy() -> Self {
        Self::new()
            .with_asset("v0.5.0", LINUX_ASSET, NEW_BINARY)
            .with_sums("v0.5.0", &[(LINUX_ASSET, NEW_BINARY)])
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

struct Scratch {
    _dir: tempfile::TempDir,
    destination: PathBuf,
}

fn scratch(body: &[u8]) -> Scratch {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("pixi-sandbox");
    std::fs::write(&destination, body).expect("write");
    Scratch {
        _dir: dir,
        destination,
    }
}

fn request<'a>(destination: &'a Path, requested_version: Option<&'a str>) -> Request<'a> {
    Request {
        repo: DEFAULT_REPO,
        requested_version,
        destination,
        current_version: "0.4.4",
        host_os: "linux",
        host_arch: "x86_64",
        strategy: ReplaceStrategy::Unix,
    }
}

#[test]
fn latest_by_default_resolves_downloads_verifies_and_replaces() {
    let source = FakeRelease::healthy();
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, None);

    let plan = plan(&source, &request).expect("planned");
    assert_eq!(plan.resolved.version, "0.5.0");
    assert_eq!(plan.asset, LINUX_ASSET);
    assert!(plan.ownership.is_standalone());
    assert!(!plan.is_up_to_date());

    let applied = apply(&source, &request, plan).expect("applied");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        NEW_BINARY
    );
    assert_eq!(
        applied.digest,
        pixi_sandbox_core::shard::sha256_bytes(NEW_BINARY)
    );
    // SHA256SUMS is fetched before the binary: there is no point downloading bytes we have
    // no way to judge.
    assert_eq!(
        source.downloads(),
        vec![
            format!("v0.5.0/{SUMS_ASSET}"),
            format!("v0.5.0/{LINUX_ASSET}")
        ]
    );
}

#[test]
fn an_exact_version_installs_that_release_even_when_it_is_older_than_latest() {
    // Rollback: decision-4's "manual rollback selects an exact older one".
    let old = b"the 0.3.7 binary";
    let source = FakeRelease::healthy()
        .with_asset("v0.3.7", LINUX_ASSET, old)
        .with_sums("v0.3.7", &[(LINUX_ASSET, old)]);
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, Some("0.3.7"));

    let plan = plan(&source, &request).expect("planned");
    assert_eq!(plan.resolved.tag, "v0.3.7");
    apply(&source, &request, plan).expect("applied");
    assert_eq!(std::fs::read(&scratch.destination).expect("read"), old);
}

#[test]
fn a_checksum_mismatch_refuses_and_leaves_the_destination_byte_for_byte_unchanged() {
    // The release lists a digest for bytes other than the ones it serves.
    let source = FakeRelease::new()
        .with_asset("v0.5.0", LINUX_ASSET, b"tampered bytes")
        .with_sums("v0.5.0", &[(LINUX_ASSET, NEW_BINARY)]);
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, None);

    let plan = plan(&source, &request).expect("planned");
    let err = apply(&source, &request, plan).unwrap_err();
    let text = format!("{err:?}");
    assert!(text.contains("checksum mismatch"), "{text}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[test]
fn a_release_whose_sums_omit_this_host_is_refused_before_anything_is_written() {
    let source = FakeRelease::new()
        .with_asset("v0.5.0", LINUX_ASSET, NEW_BINARY)
        .with_sums("v0.5.0", &[("pixi-sandbox-aarch64-apple-darwin", b"other")]);
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, None);

    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("no entry for"), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[test]
fn a_release_with_no_sums_asset_at_all_is_refused() {
    let source = FakeRelease::new().with_asset("v0.5.0", LINUX_ASSET, NEW_BINARY);
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, None);
    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains(SUMS_ASSET), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[test]
fn a_missing_binary_asset_is_an_http_failure_that_writes_nothing() {
    let source = FakeRelease::new().with_sums("v0.5.0", &[(LINUX_ASSET, NEW_BINARY)]);
    let scratch = scratch(b"the 0.4.4 binary");
    let request = request(&scratch.destination, None);
    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("404"), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[test]
fn a_pixi_managed_destination_is_refused_without_downloading_a_single_byte() {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join(".pixi/envs/default");
    std::fs::create_dir_all(prefix.join("conda-meta")).expect("mkdir");
    let destination = prefix.join("bin/pixi-sandbox");
    std::fs::create_dir_all(destination.parent().expect("parent")).expect("mkdir");
    std::fs::write(&destination, b"pixi-managed").expect("write");

    let source = FakeRelease::healthy();
    let request = request(&destination, None);
    let plan = plan(&source, &request).expect("planned");
    assert!(!plan.ownership.is_standalone());

    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("pixi update pixi-sandbox"), "{err}");
    assert!(
        source.downloads().is_empty(),
        "a refused destination must cost no downloads: {:?}",
        source.downloads()
    );
    assert_eq!(std::fs::read(&destination).expect("read"), b"pixi-managed");
}

#[test]
fn a_global_trampoline_destination_is_refused_with_the_pixi_remedy() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin = dir.path().join("pixi-sandbox");
    std::fs::write(&bin, b"trampoline").expect("write");
    std::fs::create_dir_all(dir.path().join("trampoline_configuration")).expect("mkdir");
    std::fs::write(
        dir.path()
            .join("trampoline_configuration/pixi-sandbox.json"),
        "{}",
    )
    .expect("write");

    let source = FakeRelease::healthy();
    let request = request(&bin, None);
    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("pixi global update"), "{err}");
    assert!(source.downloads().is_empty());
}

#[test]
fn an_explicit_ci_managed_destination_in_runner_scratch_is_supported() {
    // The shape the generated upgrade lane uses: a disposable standalone binary under
    // $RUNNER_TEMP, not the machine's own tool.
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("runner-temp/pixi-sandbox");
    let source = FakeRelease::healthy();
    let request = request(&destination, Some("0.5.0"));
    let plan = plan(&source, &request).expect("planned");
    assert!(plan.ownership.is_standalone());
    apply(&source, &request, plan).expect("applied");
    assert_eq!(std::fs::read(&destination).expect("read"), NEW_BINARY);
}

#[test]
fn an_unsupported_host_fails_at_plan_time_before_any_release_lookup() {
    let scratch = scratch(b"binary");
    let mut request = request(&scratch.destination, None);
    request.host_arch = "riscv64";
    let source = FakeRelease::healthy();
    let err = format!("{:?}", plan(&source, &request).unwrap_err());
    assert!(err.contains("linux/riscv64"), "{err}");
}

#[test]
fn planning_reports_an_already_current_destination_without_writing() {
    let source = FakeRelease::healthy();
    let scratch = scratch(b"the 0.5.0 binary");
    let mut request = request(&scratch.destination, None);
    request.current_version = "0.5.0";
    let plan = plan(&source, &request).expect("planned");
    assert!(plan.is_up_to_date());
    // `plan` is the whole of `--check`: nothing downloaded, nothing written.
    assert!(source.downloads().is_empty());
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.5.0 binary"
    );
}

#[test]
fn the_windows_strategy_completes_the_same_verified_update() {
    let source = FakeRelease::healthy();
    let scratch = scratch(b"the 0.4.4 binary");
    let mut request = request(&scratch.destination, None);
    request.strategy = ReplaceStrategy::Windows;
    let plan = plan(&source, &request).expect("planned");
    let applied = apply(&source, &request, plan).expect("applied");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        NEW_BINARY
    );
    let displaced = applied.replacement.displaced.expect("kept aside");
    assert!(displaced.to_string_lossy().contains("0.4.4"));
}
