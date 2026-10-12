//! TASK-78 / ADR-28: the pure rules for wrapping a prebuilt payload as a conda package.
//!
//! Every case here is decided from in-memory inputs. Nothing runs `rattler-build`, reads the
//! network, or touches this checkout: the toolchain seam is a fake that records what it was asked
//! to do, and filesystem cases use a tempdir.

use pixi_sandbox_core::payload::{
    NativeFormat, PAYLOAD_DIR, PROVENANCE_FILE, PayloadEntry, PayloadError, PayloadSpec,
    PayloadToolchain, WrapPlan, WrapRequest, budget_preflight, detect_native, plan_wrap,
    render_provenance, render_recipe, validate_payload, wrap, write_provenance,
};
use pixi_sandbox_core::platform::Platform;
use pixi_sandbox_core::transport_budget::{MIB, TransportBudgets};
use rstest::rstest;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

const ELF: &[u8] = b"\x7fELF\x02\x01\x01\x00";
const MACHO: &[u8] = &[0xCF, 0xFA, 0xED, 0xFE];
const PE: &[u8] = b"MZ\x90\x00";
const DATA: &[u8] = b"plain";
const SHA256: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn file(path: &str, size: u64, executable: bool, head: &[u8]) -> PayloadEntry {
    PayloadEntry::File {
        path: path.to_string(),
        size,
        executable,
        head: head.to_vec(),
    }
}

fn dir(path: &str) -> PayloadEntry {
    PayloadEntry::Directory {
        path: path.to_string(),
    }
}

fn link(path: &str, target: &str) -> PayloadEntry {
    PayloadEntry::Symlink {
        path: path.to_string(),
        target: target.to_string(),
    }
}

fn spec(platform: Platform) -> PayloadSpec {
    PayloadSpec {
        name: "demo-payload".to_string(),
        version: "1.0.0".to_string(),
        platform,
        noarch: false,
        entry_point: None,
    }
}

/// A linux-64 tree with every shape the round trip must keep: an executable, a shared library,
/// a relative link, a dangling relative link, and an empty directory.
fn linux_tree() -> Vec<PayloadEntry> {
    vec![
        dir("empty"),
        file("bin/tool", 1000, true, ELF),
        file("lib/libx.so", 300, false, ELF),
        file("share/data.bin", 500, false, DATA),
        link("bin/alias", "../lib/libx.so"),
        link("lib/dangling", "gone.so"),
    ]
}

fn request(spec: PayloadSpec, entries: Vec<PayloadEntry>) -> WrapRequest {
    WrapRequest {
        spec,
        entries,
        source_url: "https://example.invalid/demo-1.0.0.tar.gz".to_string(),
        source_sha256: SHA256.to_string(),
        budgets: TransportBudgets::default(),
    }
}

// ---- validation -------------------------------------------------------------------------------

#[test]
fn a_linux_payload_with_links_and_empty_directories_validates_and_counts_its_parts() {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree())
        .expect("a well-formed linux payload is accepted");

    assert_eq!(validated.file_count, 3);
    assert_eq!(validated.directory_count, 1);
    assert_eq!(validated.symlink_count, 2);
    assert_eq!(validated.total_bytes, 1800);
}

#[test]
fn a_native_payload_declared_noarch_is_refused_naming_the_file_and_its_format() {
    let mut declared = spec(Platform::Linux64);
    declared.noarch = true;

    let error = validate_payload(&declared, &linux_tree()).expect_err("noarch native is refused");

    assert_eq!(
        error,
        PayloadError::NativeDeclaredNoarch {
            path: "bin/tool".to_string(),
            format: NativeFormat::Elf,
        }
    );
}

#[test]
fn a_data_only_payload_may_be_declared_noarch() {
    let mut declared = spec(Platform::Linux64);
    declared.noarch = true;
    let data_only = vec![file("share/data.bin", 500, false, DATA)];

    validate_payload(&declared, &data_only).expect("no native file, so noarch is honest");
}

#[test]
fn an_executable_of_another_operating_system_is_refused_for_the_platform() {
    let entries = vec![file("bin/tool", 10, true, MACHO)];

    let error = validate_payload(&spec(Platform::Linux64), &entries)
        .expect_err("a Mach-O file does not belong in linux-64");

    assert_eq!(
        error,
        PayloadError::PlatformMismatch {
            path: "bin/tool".to_string(),
            format: NativeFormat::MachO,
            platform: Platform::Linux64,
        }
    );
    validate_payload(&spec(Platform::OsxArm64), &entries)
        .expect("the same Mach-O file is at home in osx-arm64");
}

#[rstest]
#[case(b"\x7fELF", Some(NativeFormat::Elf))]
#[case(&[0xCF, 0xFA, 0xED, 0xFE], Some(NativeFormat::MachO))]
#[case(&[0xFE, 0xED, 0xFA, 0xCF], Some(NativeFormat::MachO))]
#[case(&[0xCE, 0xFA, 0xED, 0xFE], Some(NativeFormat::MachO))]
#[case(&[0xFE, 0xED, 0xFA, 0xCE], Some(NativeFormat::MachO))]
#[case(b"MZ", Some(NativeFormat::Pe))]
// A universal Mach-O shares its magic with Java class files, so it is deliberately not
// detected: a false positive would refuse a legitimate noarch package.
#[case(&[0xCA, 0xFE, 0xBA, 0xBE], None)]
#[case(b"#!/bin/sh", None)]
#[case(b"\x7fEL", None)]
#[case(b"", None)]
fn detect_native_recognises_executables_by_magic_bytes(
    #[case] head: &[u8],
    #[case] expected: Option<NativeFormat>,
) {
    assert_eq!(detect_native(head), expected);
}

#[rstest]
#[case("/etc/passwd")]
#[case("../escape")]
#[case("a/../../b")]
#[case("a//b")]
#[case("a\\b")]
#[case("")]
#[case("./a")]
#[case("a/./b")]
fn an_unsafe_payload_path_is_refused(#[case] path: &str) {
    let entries = vec![file(path, 1, false, DATA)];

    let error = validate_payload(&spec(Platform::Linux64), &entries)
        .expect_err("unsafe paths never reach the recipe");

    assert_eq!(error, PayloadError::UnsafePath(path.to_string()));
}

#[test]
fn a_duplicate_path_is_a_conflict_naming_the_second_entry() {
    let entries = vec![file("a", 1, false, DATA), file("a", 2, false, DATA)];

    let error = validate_payload(&spec(Platform::Linux64), &entries).expect_err("duplicate");

    assert_eq!(error, PayloadError::ConflictingPath("a".to_string()));
}

#[test]
fn a_file_cannot_sit_where_a_directory_of_the_payload_already_is() {
    let entries = vec![file("a/b", 1, false, DATA), file("a", 1, false, DATA)];

    let error = validate_payload(&spec(Platform::Linux64), &entries).expect_err("conflict");

    assert_eq!(error, PayloadError::ConflictingPath("a".to_string()));
}

#[test]
fn a_symlink_used_as_a_parent_directory_is_refused_so_links_cannot_hide_an_escape() {
    // A directory symlink would make the lexical escape check disagree with the real layout.
    let entries = vec![link("s", "."), file("s/x", 1, false, DATA)];

    let error = validate_payload(&spec(Platform::Linux64), &entries).expect_err("conflict");

    assert_eq!(error, PayloadError::ConflictingPath("s/x".to_string()));
}

#[test]
fn a_relative_symlink_that_stays_inside_the_payload_is_accepted() {
    let entries = vec![file("y/file", 1, false, DATA), link("x/link", "../y/file")];

    validate_payload(&spec(Platform::Linux64), &entries).expect("stays inside the root");
}

#[rstest]
#[case("top", "../outside")]
#[case("a/b/link", "../../../x")]
#[case("link", "/usr/lib/libz.so")]
fn a_symlink_that_leaves_the_payload_is_refused(#[case] link_path: &str, #[case] target: &str) {
    let entries = vec![link(link_path, target)];

    let error = validate_payload(&spec(Platform::Linux64), &entries)
        .expect_err("escaping links are refused");

    assert_eq!(
        error,
        PayloadError::EscapingSymlink {
            link: link_path.to_string(),
            target: target.to_string(),
        }
    );
}

#[rstest]
#[case("Bad Name")]
#[case("UPPER")]
#[case("-leading")]
#[case("")]
#[case("name/with/slash")]
fn an_invalid_package_name_is_refused(#[case] name: &str) {
    let mut bad = spec(Platform::Linux64);
    bad.name = name.to_string();

    let error = validate_payload(&bad, &linux_tree()).expect_err("invalid name");

    assert_eq!(error, PayloadError::InvalidName(name.to_string()));
}

#[rstest]
#[case("chromium_browser")]
#[case("playwright-chromium")]
#[case("tool.v2")]
fn package_names_may_use_lowercase_digits_dots_dashes_and_underscores(#[case] name: &str) {
    let mut ok = spec(Platform::Linux64);
    ok.name = name.to_string();

    validate_payload(&ok, &linux_tree()).expect("a valid conda name");
}

#[rstest]
#[case("")]
#[case("1.0-beta")]
#[case("v1 0")]
#[case(".1")]
#[case("1.0!")]
fn an_invalid_version_is_refused(#[case] version: &str) {
    let mut bad = spec(Platform::Linux64);
    bad.version = version.to_string();

    let error = validate_payload(&bad, &linux_tree()).expect_err("invalid version");

    assert_eq!(error, PayloadError::InvalidVersion(version.to_string()));
}

#[rstest]
#[case("1.0.0")]
#[case("2026.10.1")]
#[case("1.0.0+build.2")]
fn calendar_and_build_suffixed_versions_are_accepted(#[case] version: &str) {
    let mut ok = spec(Platform::Linux64);
    ok.version = version.to_string();

    validate_payload(&ok, &linux_tree()).expect("a valid conda version");
}

// ---- entry point and launcher -----------------------------------------------------------------

#[test]
fn an_entry_point_must_name_an_executable_file_inside_the_payload() {
    let mut with_entry = spec(Platform::Linux64);
    with_entry.entry_point = Some("bin/nope".to_string());
    assert_eq!(
        validate_payload(&with_entry, &linux_tree()).expect_err("missing"),
        PayloadError::EntryPointMissing("bin/nope".to_string())
    );

    with_entry.entry_point = Some("share/data.bin".to_string());
    assert_eq!(
        validate_payload(&with_entry, &linux_tree()).expect_err("not executable"),
        PayloadError::EntryPointNotExecutable("share/data.bin".to_string())
    );

    with_entry.entry_point = Some("bin/tool".to_string());
    validate_payload(&with_entry, &linux_tree()).expect("an executable file is a valid entry");
}

#[test]
fn an_entry_point_the_launcher_cannot_quote_is_refused() {
    let entries = vec![file("bin/my tool", 10, true, ELF)];
    let mut with_entry = spec(Platform::Linux64);
    with_entry.entry_point = Some("bin/my tool".to_string());

    let error = validate_payload(&with_entry, &entries).expect_err("space is not allowed");

    assert_eq!(
        error,
        PayloadError::EntryPointUnsafe("bin/my tool".to_string())
    );
}

#[test]
fn an_entry_point_on_win64_is_refused_until_a_windows_launcher_exists() {
    let entries = vec![file("bin/tool.exe", 10, true, PE)];
    let mut with_entry = spec(Platform::Win64);
    with_entry.entry_point = Some("bin/tool.exe".to_string());

    let error = validate_payload(&with_entry, &entries).expect_err("POSIX launcher only");

    assert_eq!(error, PayloadError::LauncherUnsupported(Platform::Win64));
}

#[test]
fn the_recipe_installs_under_libexec_and_writes_a_launcher_that_execs_in_place() {
    let mut with_entry = spec(Platform::Linux64);
    with_entry.entry_point = Some("bin/tool".to_string());
    let validated = validate_payload(&with_entry, &linux_tree()).expect("valid");

    let recipe = render_recipe(&with_entry, &validated);

    let expected = r#"package:
  name: "demo-payload"
  version: "1.0.0"

source:
  - path: payload
  - path: pixi-sandbox-payload.json

build:
  number: 0
  script: |
    set -eu
    mkdir -p "$PREFIX/libexec/demo-payload" "$PREFIX/bin"
    cp -a "$RECIPE_DIR/payload/." "$PREFIX/libexec/demo-payload/"
    cp "$RECIPE_DIR/pixi-sandbox-payload.json" "$PREFIX/libexec/demo-payload/"
    cat > "$PREFIX/bin/tool" <<'PIXI_SANDBOX_LAUNCHER'
    #!/bin/sh
    exec "$(dirname "$0")/../libexec/demo-payload/bin/tool" "$@"
    PIXI_SANDBOX_LAUNCHER
    chmod 755 "$PREFIX/bin/tool"
"#;
    assert_eq!(recipe, expected);
    assert_eq!(PAYLOAD_DIR, "payload");
}

#[test]
fn a_recipe_without_an_entry_point_copies_the_payload_and_writes_no_launcher() {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree()).expect("valid");

    let recipe = render_recipe(&spec(Platform::Linux64), &validated);

    assert!(recipe.contains("cp -a \"$RECIPE_DIR/payload/.\""));
    assert!(!recipe.contains("PIXI_SANDBOX_LAUNCHER"));
    assert!(!recipe.contains("$PREFIX/bin"));
}

// ---- budgets ----------------------------------------------------------------------------------

fn sized(total_mib: u64) -> Vec<PayloadEntry> {
    vec![file("share/big.bin", total_mib * MIB, false, DATA)]
}

#[test]
fn a_payload_within_both_budgets_passes_preflight() {
    let validated = validate_payload(&spec(Platform::Linux64), &sized(10)).expect("valid");

    budget_preflight(&validated, &TransportBudgets::default()).expect("10 MiB is well within");
}

#[test]
fn an_oversized_package_is_refused_naming_the_blob_budget_and_the_measured_size() {
    let validated = validate_payload(&spec(Platform::Linux64), &sized(96)).expect("valid");

    let error = budget_preflight(&validated, &TransportBudgets::default())
        .expect_err("96 MiB exceeds the 95 MiB blob budget");

    let PayloadError::Budget(violations) = error else {
        panic!("expected a budget refusal, got {error:?}");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].field, "budgets.max_blob_mib");
    assert_eq!(violations[0].measured_bytes, 96 * MIB);
    assert_eq!(violations[0].limit_bytes, 95 * MIB);
    assert!(!violations[0].remedy.is_empty());
}

#[test]
fn the_transport_budget_counts_the_payload_twice_once_as_package_and_once_as_prefix_files() {
    // 60 MiB is under the blob budget, but the transport carries it twice: 120 MiB > 100 MiB.
    let budgets = TransportBudgets {
        max_transport_bytes: 100 * MIB,
        ..TransportBudgets::default()
    };
    let validated = validate_payload(&spec(Platform::Linux64), &sized(60)).expect("valid");

    let error = budget_preflight(&validated, &budgets).expect_err("the transport is over");

    let PayloadError::Budget(violations) = error else {
        panic!("expected a budget refusal, got {error:?}");
    };
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].field, "budgets.max_transport_mib");
    assert_eq!(violations[0].measured_bytes, 120 * MIB);
    assert_eq!(violations[0].limit_bytes, 100 * MIB);
}

#[test]
fn both_budgets_can_be_refused_in_one_report_in_a_fixed_order() {
    let budgets = TransportBudgets {
        max_transport_bytes: 100 * MIB,
        ..TransportBudgets::default()
    };
    let validated = validate_payload(&spec(Platform::Linux64), &sized(96)).expect("valid");

    let PayloadError::Budget(violations) =
        budget_preflight(&validated, &budgets).expect_err("both are over")
    else {
        panic!("expected a budget refusal");
    };
    let fields: Vec<&str> = violations.iter().map(|v| v.field).collect();
    assert_eq!(
        fields,
        ["budgets.max_blob_mib", "budgets.max_transport_mib"]
    );
}

// ---- provenance -------------------------------------------------------------------------------

#[test]
fn the_provenance_document_records_the_source_hash_and_payload_totals() {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree()).expect("valid");
    let provenance = pixi_sandbox_core::payload::provenance(
        &spec(Platform::Linux64),
        &validated,
        "https://example.invalid/demo-1.0.0.tar.gz",
        SHA256,
    )
    .expect("a valid source");

    let expected = r#"{
  "schema": 1,
  "package": {
    "name": "demo-payload",
    "version": "1.0.0",
    "platform": "linux-64"
  },
  "source": {
    "url": "https://example.invalid/demo-1.0.0.tar.gz",
    "sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
  },
  "payload": {
    "files": 3,
    "bytes": 1800
  }
}
"#;
    assert_eq!(render_provenance(&provenance), expected);
}

#[rstest]
#[case("http://example.invalid/x")]
#[case("file:///tmp/x")]
#[case("https://example.invalid/has space")]
#[case("")]
fn provenance_refuses_a_source_that_is_not_an_https_url(#[case] url: &str) {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree()).expect("valid");

    let error =
        pixi_sandbox_core::payload::provenance(&spec(Platform::Linux64), &validated, url, SHA256)
            .expect_err("only https sources are recorded");

    assert_eq!(error, PayloadError::InvalidSourceUrl(url.to_string()));
}

#[rstest]
#[case("")]
#[case("0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF")]
#[case("0123456789abcdef")]
#[case("zz23456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")]
fn provenance_refuses_a_digest_that_is_not_64_lowercase_hex(#[case] digest: &str) {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree()).expect("valid");

    let error = pixi_sandbox_core::payload::provenance(
        &spec(Platform::Linux64),
        &validated,
        "https://example.invalid/demo.tar.gz",
        digest,
    )
    .expect_err("a digest the auditor cannot compare is refused");

    assert_eq!(error, PayloadError::InvalidSha256(digest.to_string()));
}

#[test]
fn write_provenance_writes_the_rendered_document_beside_the_recipe() {
    let validated = validate_payload(&spec(Platform::Linux64), &linux_tree()).expect("valid");
    let provenance = pixi_sandbox_core::payload::provenance(
        &spec(Platform::Linux64),
        &validated,
        "https://example.invalid/demo-1.0.0.tar.gz",
        SHA256,
    )
    .expect("valid");
    let work = tempfile::tempdir().expect("tempdir");

    let written = write_provenance(work.path(), &provenance).expect("writes");

    assert_eq!(written, work.path().join(PROVENANCE_FILE));
    assert_eq!(
        std::fs::read_to_string(&written).expect("readable"),
        render_provenance(&provenance)
    );
}

// ---- plan and wrap through the toolchain seam -------------------------------------------------

/// Records every call the orchestrator makes, in order. Reports the build as failed when asked.
#[derive(Default)]
struct FakeToolchain {
    calls: RefCell<Vec<String>>,
    fail_build: bool,
}

impl PayloadToolchain for FakeToolchain {
    fn build(&self, plan: &WrapPlan, channel_dir: &Path) -> Result<PathBuf, PayloadError> {
        self.calls.borrow_mut().push(format!(
            "build {} {} {}",
            plan.spec.name, plan.spec.version, plan.spec.platform
        ));
        if self.fail_build {
            return Err(PayloadError::Toolchain(
                "rattler-build exited 1".to_string(),
            ));
        }
        Ok(channel_dir
            .join(plan.spec.platform.as_str())
            .join(format!("{}-{}-0.conda", plan.spec.name, plan.spec.version)))
    }

    fn index(&self, channel_dir: &Path) -> Result<(), PayloadError> {
        self.calls
            .borrow_mut()
            .push(format!("index {}", channel_dir.display()));
        Ok(())
    }
}

#[test]
fn a_refused_payload_never_reaches_the_toolchain() {
    let fake = FakeToolchain::default();
    let mut declared = spec(Platform::Linux64);
    declared.noarch = true;

    let error = wrap(
        &request(declared, linux_tree()),
        &fake,
        Path::new("channel"),
    )
    .expect_err("noarch native is refused before any build");

    assert!(matches!(error, PayloadError::NativeDeclaredNoarch { .. }));
    assert!(fake.calls.borrow().is_empty(), "no build, no index");
}

#[test]
fn an_over_budget_payload_never_reaches_the_toolchain() {
    let fake = FakeToolchain::default();
    let mut too_big = request(spec(Platform::Linux64), sized(96));
    too_big.budgets.max_blob_bytes = 95 * MIB;

    let error = wrap(&too_big, &fake, Path::new("channel")).expect_err("over budget");

    assert!(matches!(error, PayloadError::Budget(_)));
    assert!(
        fake.calls.borrow().is_empty(),
        "a refused package is never built"
    );
}

#[test]
fn an_accepted_payload_is_built_then_indexed_and_the_package_is_returned() {
    let fake = FakeToolchain::default();

    let package = wrap(
        &request(spec(Platform::Linux64), linux_tree()),
        &fake,
        Path::new("channel"),
    )
    .expect("accepted");

    assert_eq!(
        *fake.calls.borrow(),
        ["build demo-payload 1.0.0 linux-64", "index channel"]
    );
    assert_eq!(
        package,
        Path::new("channel/linux-64/demo-payload-1.0.0-0.conda")
    );
}

#[test]
fn a_failed_build_is_reported_and_the_channel_is_not_indexed() {
    let fake = FakeToolchain {
        fail_build: true,
        ..FakeToolchain::default()
    };

    let error = wrap(
        &request(spec(Platform::Linux64), linux_tree()),
        &fake,
        Path::new("channel"),
    )
    .expect_err("the build failed");

    assert_eq!(
        error,
        PayloadError::Toolchain("rattler-build exited 1".to_string())
    );
    assert_eq!(*fake.calls.borrow(), ["build demo-payload 1.0.0 linux-64"]);
}

#[test]
fn the_plan_carries_the_recipe_and_provenance_for_the_same_payload() {
    let mut with_entry = spec(Platform::Linux64);
    with_entry.entry_point = Some("bin/tool".to_string());

    let plan: WrapPlan = plan_wrap(&request(with_entry.clone(), linux_tree())).expect("accepted");

    let validated = validate_payload(&with_entry, &linux_tree()).expect("valid");
    assert_eq!(plan.recipe, render_recipe(&with_entry, &validated));
    assert_eq!(plan.provenance.source.sha256, SHA256);
    assert_eq!(plan.provenance.payload.files, 3);
    assert_eq!(plan.provenance.payload.bytes, 1800);
}

// ---- generality -------------------------------------------------------------------------------

#[test]
fn the_implementation_names_no_browser_or_vendor_it_is_an_example_of() {
    // AC#9: Firefox and Playwright are worked examples in documentation and tests. The code
    // takes a payload, a name, a version and a platform, and must stay that generic.
    let source =
        std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/payload.rs"))
            .expect("the payload module is readable");
    let lower = source.to_lowercase();

    for word in ["playwright", "chromium", "firefox", "browser"] {
        assert!(
            !lower.contains(word),
            "src/payload.rs names `{word}`; the example belongs in docs and tests"
        );
    }
}
