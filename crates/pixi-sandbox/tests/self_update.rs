//! Unit tests for the self-update orchestration (`src/self_update/mod.rs`).
//!
//! The whole trust boundary — resolve, classify, download, verify, replace — against an
//! in-memory fake release. No socket is opened and no real binary is written (D10).

use anyhow::Result;
use pixi_sandbox::release::{Asset, ReleaseSource};
use pixi_sandbox::self_update::replace::ReplaceStrategy;
use pixi_sandbox::self_update::*;
use rstest::{fixture, rstest};
use std::path::{Path, PathBuf};

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::Write as _;

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
        let mut body = String::new();
        for (name, bytes) in entries {
            let _ = writeln!(
                body,
                "{}  {name}",
                pixi_sandbox_core::shard::sha256_bytes(bytes)
            );
        }
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

/// A standalone destination holding `body`. Defaults to the pre-update 0.4.4 binary, the shape
/// almost every test in this file needs; tests that start from a different installed binary
/// override it with `#[with(...)]`.
#[fixture]
fn scratch(#[default(&b"the 0.4.4 binary"[..])] body: &[u8]) -> Scratch {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("pixi-sandbox");
    std::fs::write(&destination, body).expect("write");
    Scratch {
        _dir: dir,
        destination,
    }
}

/// A destination pixi itself manages (a `conda-meta` prefix with the binary inside `bin/`),
/// shared by the two tests that assert self-update refuses to touch pixi's own installs.
#[fixture]
fn pixi_managed_destination() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join(".pixi/envs/default");
    std::fs::create_dir_all(prefix.join("conda-meta")).expect("mkdir");
    let destination = prefix.join("bin/pixi-sandbox");
    std::fs::create_dir_all(destination.parent().expect("parent")).expect("mkdir");
    std::fs::write(&destination, b"pixi-managed").expect("write");
    (dir, destination)
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

#[rstest]
fn latest_by_default_resolves_downloads_verifies_and_replaces(scratch: Scratch) {
    let source = FakeRelease::healthy();
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

#[rstest]
fn an_exact_version_installs_that_release_even_when_it_is_older_than_latest(scratch: Scratch) {
    // Rollback: decision-4's "manual rollback selects an exact older one".
    let old = b"the 0.3.7 binary";
    let source = FakeRelease::healthy()
        .with_asset("v0.3.7", LINUX_ASSET, old)
        .with_sums("v0.3.7", &[(LINUX_ASSET, old)]);
    let request = request(&scratch.destination, Some("0.3.7"));

    let plan = plan(&source, &request).expect("planned");
    assert_eq!(plan.resolved.tag, "v0.3.7");
    apply(&source, &request, plan).expect("applied");
    assert_eq!(std::fs::read(&scratch.destination).expect("read"), old);
}

#[rstest]
fn a_checksum_mismatch_refuses_and_leaves_the_destination_byte_for_byte_unchanged(
    scratch: Scratch,
) {
    // The release lists a digest for bytes other than the ones it serves.
    let source = FakeRelease::new()
        .with_asset("v0.5.0", LINUX_ASSET, b"tampered bytes")
        .with_sums("v0.5.0", &[(LINUX_ASSET, NEW_BINARY)]);
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

#[rstest]
fn a_release_whose_sums_omit_this_host_is_refused_before_anything_is_written(scratch: Scratch) {
    let source = FakeRelease::new()
        .with_asset("v0.5.0", LINUX_ASSET, NEW_BINARY)
        .with_sums("v0.5.0", &[("pixi-sandbox-aarch64-apple-darwin", b"other")]);
    let request = request(&scratch.destination, None);

    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("no entry for"), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[rstest]
fn a_release_with_no_sums_asset_at_all_is_refused(scratch: Scratch) {
    let source = FakeRelease::new().with_asset("v0.5.0", LINUX_ASSET, NEW_BINARY);
    let request = request(&scratch.destination, None);
    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains(SUMS_ASSET), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[rstest]
fn a_missing_binary_asset_is_an_http_failure_that_writes_nothing(scratch: Scratch) {
    let source = FakeRelease::new().with_sums("v0.5.0", &[(LINUX_ASSET, NEW_BINARY)]);
    let request = request(&scratch.destination, None);
    let plan = plan(&source, &request).expect("planned");
    let err = format!("{:?}", apply(&source, &request, plan).unwrap_err());
    assert!(err.contains("404"), "{err}");
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
}

#[rstest]
fn a_pixi_managed_destination_is_refused_without_downloading_a_single_byte(
    pixi_managed_destination: (tempfile::TempDir, PathBuf),
) {
    let (_dir, destination) = pixi_managed_destination;
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

#[rstest]
fn an_unsupported_host_fails_at_plan_time_before_any_release_lookup(
    #[with(b"binary")] scratch: Scratch,
) {
    let mut request = request(&scratch.destination, None);
    request.host_arch = "riscv64";
    let source = FakeRelease::healthy();
    let err = format!("{:?}", plan(&source, &request).unwrap_err());
    assert!(err.contains("linux/riscv64"), "{err}");
}

#[rstest]
fn planning_reports_an_already_current_destination_without_writing(
    #[with(b"the 0.5.0 binary")] scratch: Scratch,
) {
    let source = FakeRelease::healthy();
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

#[rstest]
fn the_windows_strategy_completes_the_same_verified_update(scratch: Scratch) {
    let source = FakeRelease::healthy();
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

// --- the printed report -----------------------------------------------------------------
//
// Formatting lives in the library so it can be asserted on structurally. `commands/` only
// loops over these lines, and a black-box test of a spawned binary can check substrings but
// earns no coverage and cannot tell "absent" from "misspelled".

#[rstest]
fn the_plan_header_names_the_destination_version_selection_and_asset(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let plan = plan(&source, &request(&scratch.destination, None)).expect("planned");
    let report = plan.report();
    assert_eq!(report.len(), 4);
    assert!(report[0].contains("pixi-sandbox"), "{report:?}");
    assert!(report[1].contains("0.4.4"), "{report:?}");
    assert!(
        report[2].contains("0.5.0") && report[2].contains("latest"),
        "the target line states which selection produced it: {report:?}"
    );
    assert!(report[3].contains(LINUX_ASSET), "{report:?}");
}

#[rstest]
fn a_pinned_plan_header_says_pinned_rather_than_latest(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let plan = plan(&source, &request(&scratch.destination, Some("0.3.7"))).expect("planned");
    assert!(plan.report()[2].contains("pinned"), "{:?}", plan.report());
}

#[rstest]
fn check_reports_an_available_update_and_says_it_wrote_nothing(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let plan = plan(&source, &request(&scratch.destination, None)).expect("planned");
    let verdict = plan.check_verdict();
    assert!(verdict[0].contains("0.4.4 -> 0.5.0"), "{verdict:?}");
    assert!(verdict[1].contains("wrote nothing"), "{verdict:?}");
}

#[rstest]
fn check_reports_an_up_to_date_destination(#[with(b"the 0.5.0 binary")] scratch: Scratch) {
    let source = FakeRelease::healthy();
    let mut request = request(&scratch.destination, None);
    request.current_version = "0.5.0";
    let plan = plan(&source, &request).expect("planned");
    assert!(
        plan.check_verdict()[0].contains("up to date"),
        "{:?}",
        plan.check_verdict()
    );
    assert!(plan.up_to_date_verdict().contains("already 0.5.0"));
}

#[rstest]
fn a_completed_update_reports_the_digest_and_the_transition(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let request = request(&scratch.destination, None);
    let plan = plan(&source, &request).expect("planned");
    let applied = apply(&source, &request, plan).expect("applied");
    let report = applied.report();
    assert!(
        report[0].contains(&pixi_sandbox_core::shard::sha256_bytes(NEW_BINARY)),
        "{report:?}"
    );
    assert!(
        report.last().expect("a verdict").contains("0.4.4 -> 0.5.0"),
        "{report:?}"
    );
    // Unix keeps nothing aside, so no "still mapped" note is printed.
    assert!(
        !report.iter().any(|line| line.contains("still mapped")),
        "{report:?}"
    );
}

#[test]
fn a_completed_update_reports_each_swept_leftover() {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("pixi-sandbox");
    std::fs::write(&destination, b"the 0.4.4 binary").expect("write");
    std::fs::write(
        dir.path().join("pixi-sandbox.pixi-sandbox-old-0.0.1"),
        b"corpse",
    )
    .expect("write");

    let source = FakeRelease::healthy();
    let request = request(&destination, None);
    let plan = plan(&source, &request).expect("planned");
    let applied = apply(&source, &request, plan).expect("applied");
    assert!(
        applied.report().iter().any(|line| line.contains("swept")),
        "{:?}",
        applied.report()
    );
}

#[test]
fn a_destination_that_is_somehow_not_standalone_is_refused_with_a_fallback_message() {
    // Defensive branch: a verdict with no remedy must still refuse rather than proceed.
    let dir = tempfile::tempdir().expect("tempdir");
    let prefix = dir.path().join("env");
    std::fs::create_dir_all(prefix.join("conda-meta")).expect("mkdir");
    let destination = prefix.join("bin/pixi-sandbox");
    std::fs::create_dir_all(destination.parent().expect("parent")).expect("mkdir");
    std::fs::write(&destination, b"managed").expect("write");

    let source = FakeRelease::healthy();
    let request = request(&destination, None);
    let plan = plan(&source, &request).expect("planned");
    assert!(apply(&source, &request, plan).is_err());
}

// --- the command flow, end to end ---------------------------------------------------------

#[rstest]
fn run_with_check_reports_the_transition_and_writes_nothing(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let lines = run(&source, &request(&scratch.destination, None), true).expect("checked");
    assert!(
        lines.iter().any(|l| l.contains("0.4.4 -> 0.5.0")),
        "{lines:?}"
    );
    assert!(
        lines.iter().any(|l| l.contains("wrote nothing")),
        "{lines:?}"
    );
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        b"the 0.4.4 binary"
    );
    assert!(
        source.downloads().is_empty(),
        "--check must not download the binary: {:?}",
        source.downloads()
    );
}

#[rstest]
fn run_without_check_performs_the_verified_replacement(scratch: Scratch) {
    let source = FakeRelease::healthy();
    let lines = run(&source, &request(&scratch.destination, None), false).expect("updated");
    assert!(
        lines.last().expect("verdict").contains("0.4.4 -> 0.5.0"),
        "{lines:?}"
    );
    assert_eq!(
        std::fs::read(&scratch.destination).expect("read"),
        NEW_BINARY
    );
}

#[rstest]
fn run_stops_at_an_up_to_date_destination_without_downloading(
    #[with(b"the 0.5.0 binary")] scratch: Scratch,
) {
    let source = FakeRelease::healthy();
    let mut request = request(&scratch.destination, None);
    request.current_version = "0.5.0";
    let lines = run(&source, &request, false).expect("nothing to do");
    assert!(
        lines.last().expect("verdict").contains("already 0.5.0"),
        "{lines:?}"
    );
    assert!(source.downloads().is_empty());
}

#[rstest]
fn run_refuses_a_managed_destination_in_both_modes_and_shows_the_header(
    pixi_managed_destination: (tempfile::TempDir, PathBuf),
) {
    let (_dir, destination) = pixi_managed_destination;
    for check in [true, false] {
        let source = FakeRelease::healthy();
        let err = run(&source, &request(&destination, None), check).unwrap_err();
        let text = format!("{err:?}");
        assert!(
            text.contains("pixi update pixi-sandbox"),
            "check={check}: {text}"
        );
        assert!(
            text.contains("self-update"),
            "the header must precede the refusal so the destination is visible: {text}"
        );
        assert!(source.downloads().is_empty(), "check={check}");
    }
    assert_eq!(std::fs::read(&destination).expect("read"), b"pixi-managed");
}

#[rstest]
fn run_propagates_an_unsupported_host_before_touching_the_release(
    #[with(b"binary")] scratch: Scratch,
) {
    let mut request = request(&scratch.destination, None);
    request.host_arch = "riscv64";
    let err = run(&FakeRelease::healthy(), &request, true).unwrap_err();
    assert!(format!("{err:?}").contains("linux/riscv64"), "{err:?}");
}
