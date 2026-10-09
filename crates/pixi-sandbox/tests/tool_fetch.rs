//! Pinned helper fetching, cache verification and transport embedding.

mod support;

use pixi_sandbox::tool_fetch::{ToolSource, embed_tool};
use rstest::rstest;
use support::isolated_home;
use tempfile::TempDir;

#[rstest]
fn windows_transport_tools_keep_the_exe_suffix(isolated_home: TempDir) {
    let source = isolated_home.path().join("source.exe");
    // The short MZ-shaped input is the same filename example used before extraction.
    std::fs::write(&source, b"MZ test binary").unwrap();
    let entry = embed_tool(
        isolated_home.path(),
        "win-64",
        "pixi",
        &ToolSource {
            path: source,
            version: "test".to_string(),
            url: None,
            pinned_sha256: None,
        },
        u64::MAX,
    )
    .unwrap();
    assert_eq!(entry.path.as_deref(), Some("tools/win-64/pixi.exe"));
    assert!(isolated_home.path().join("tools/win-64/pixi.exe").is_file());
}

#[rstest]
fn an_explicit_tools_lock_override_that_does_not_exist_is_refused(isolated_home: TempDir) {
    let error = pixi_sandbox::tool_fetch::resolve_lock(
        isolated_home.path(),
        Some(std::path::Path::new("missing-tools-lock.toml")),
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("reading tool-pin override"),
        "{error:#}"
    );
}

#[cfg(unix)]
mod downloads {
    use super::*;
    use pixi_sandbox::tool_fetch::{ToolDownload, fetch_tool_with};
    use pixi_sandbox_core::tools_lock::{PlatformPin, Tool, ToolsLock};
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::io::{Cursor, Read};

    const TOOL_BYTES: &[u8] = b"#!/bin/sh\necho 'pixi 1.2.3'\n";
    // Independently calculated with sha256sum, not the verifier under test.
    const TOOL_SHA256: &str = "9345888d9173e409ff905372c162819f4a77cd51ca15b4cd9cf5a7529e6ebbe4";

    fn lock(version: &str, platform: &str) -> ToolsLock {
        let target = if platform == "win-64" {
            "x86_64-pc-windows-msvc.exe"
        } else {
            "x86_64-unknown-linux-musl"
        };
        ToolsLock {
            schema: 1,
            generated_at: None,
            tools: BTreeMap::from([(
                "pixi".into(),
                Tool {
                    version: version.into(),
                    url_template: "https://tools.invalid/v{version}/{target}".into(),
                    platforms: BTreeMap::from([(
                        platform.into(),
                        PlatformPin {
                            target: target.into(),
                            sha256: TOOL_SHA256.into(),
                            linkage: "static".into(),
                            note: None,
                        },
                    )]),
                },
            )]),
        }
    }

    struct FakeDownload {
        body: Vec<u8>,
        requests: RefCell<Vec<String>>,
        fail: bool,
    }

    impl FakeDownload {
        fn new(body: &[u8]) -> Self {
            Self {
                body: body.to_vec(),
                requests: RefCell::default(),
                fail: false,
            }
        }
    }

    impl ToolDownload for FakeDownload {
        fn open(&self, url: &str) -> anyhow::Result<Box<dyn Read>> {
            self.requests.borrow_mut().push(url.into());
            if self.fail {
                anyhow::bail!("fixture download unavailable");
            }
            Ok(Box::new(Cursor::new(self.body.clone())))
        }
    }

    #[rstest]
    fn only_verified_bytes_are_cached_and_their_pin_provenance_is_preserved(
        isolated_home: TempDir,
    ) {
        let cache = isolated_home.path().join("cache");
        let download = FakeDownload::new(TOOL_BYTES);
        let result = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap();
        assert_eq!(std::fs::read(&result.path).unwrap(), TOOL_BYTES);
        assert_eq!(result.version, "1.2.3");
        assert_eq!(result.sha256, TOOL_SHA256);
        assert_eq!(
            result.url,
            "https://tools.invalid/v1.2.3/x86_64-unknown-linux-musl"
        );
        assert_eq!(result.path, cache.join("pixi-1.2.3-linux-64"));
        assert_eq!(*download.requests.borrow(), [result.url]);
    }

    #[rstest]
    fn a_verified_cache_hit_never_opens_a_download(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let cached = cache.join("pixi-1.2.3-linux-64");
        std::fs::write(&cached, TOOL_BYTES).unwrap();
        let mut download = FakeDownload::new(b"must not be used");
        download.fail = true;
        let result = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap();
        assert_eq!(result.path, cached);
        assert!(download.requests.borrow().is_empty());
    }

    #[rstest]
    fn a_corrupt_cache_is_replaced_only_after_the_download_verifies(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let cached = cache.join("pixi-1.2.3-linux-64");
        std::fs::write(&cached, b"corrupt cached bytes").unwrap();
        let download = FakeDownload::new(TOOL_BYTES);
        fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap();
        assert_eq!(std::fs::read(cached).unwrap(), TOOL_BYTES);
        assert_eq!(download.requests.borrow().len(), 1);
        assert_eq!(
            std::fs::read_dir(cache).unwrap().count(),
            1,
            "no download scratch survives"
        );
    }

    #[rstest]
    fn an_unverified_download_never_replaces_the_existing_cache(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        std::fs::create_dir_all(&cache).unwrap();
        let cached = cache.join("pixi-1.2.3-linux-64");
        std::fs::write(&cached, b"original corrupt cache").unwrap();
        let download = FakeDownload::new(b"download does not match the pin");
        let error = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains("integrity:"), "{error:#}");
        assert_eq!(std::fs::read(cached).unwrap(), b"original corrupt cache");
        assert_eq!(
            std::fs::read_dir(cache).unwrap().count(),
            1,
            "failed download removed"
        );
    }

    #[rstest]
    fn a_download_failure_names_the_tool_and_url_without_leaving_a_partial_file(
        isolated_home: TempDir,
    ) {
        let cache = isolated_home.path().join("cache");
        let mut download = FakeDownload::new(TOOL_BYTES);
        download.fail = true;
        let error = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains(
                "downloading pixi from https://tools.invalid/v1.2.3/x86_64-unknown-linux-musl"
            ),
            "{message}"
        );
        assert!(
            message.contains("fixture download unavailable"),
            "{message}"
        );
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 0);
    }

    struct InterruptedDownload;
    struct InterruptedBody(bool);
    impl Read for InterruptedBody {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            if self.0 {
                return Err(std::io::Error::other("fixture stream interrupted"));
            }
            self.0 = true;
            let count = 3.min(buffer.len());
            buffer[..count].copy_from_slice(&b"bad"[..count]);
            Ok(count)
        }
    }
    impl ToolDownload for InterruptedDownload {
        fn open(&self, _: &str) -> anyhow::Result<Box<dyn Read>> {
            Ok(Box::new(InterruptedBody(false)))
        }
    }

    #[rstest]
    fn an_interrupted_body_is_removed_and_reports_the_write_failure(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        let error = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &InterruptedDownload,
        )
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(message.contains("writing download for pixi"), "{message}");
        assert!(message.contains("fixture stream interrupted"), "{message}");
        assert_eq!(std::fs::read_dir(cache).unwrap().count(), 0);
    }

    #[rstest]
    #[case::missing_tool("unreviewed", "linux-64", "not pinned")]
    #[case::missing_platform("pixi", "osx-arm64", "no pin for platform")]
    fn a_missing_pin_refuses_before_download_or_cache_creation(
        isolated_home: TempDir,
        #[case] name: &str,
        #[case] platform: &str,
        #[case] refusal: &str,
    ) {
        let cache = isolated_home.path().join("cache");
        let download = FakeDownload::new(TOOL_BYTES);
        let error = fetch_tool_with(
            &lock("1.2.3", "linux-64"),
            name,
            platform,
            &cache,
            &download,
        )
        .unwrap_err();
        assert!(format!("{error:#}").contains(refusal), "{error:#}");
        assert!(download.requests.borrow().is_empty());
        assert!(!cache.exists());
    }

    #[rstest]
    fn a_verified_binary_with_the_wrong_reported_version_is_refused(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        let download = FakeDownload::new(TOOL_BYTES);
        let error = fetch_tool_with(
            &lock("2.0.0", "linux-64"),
            "pixi",
            "linux-64",
            &cache,
            &download,
        )
        .unwrap_err();
        let message = format!("{error:#}");
        assert!(
            message.contains("selected tool pins require 2.0.0"),
            "{message}"
        );
        assert!(message.contains("1.2.3"), "{message}");
    }

    #[rstest]
    fn windows_cached_tools_keep_the_exe_suffix(isolated_home: TempDir) {
        let cache = isolated_home.path().join("cache");
        let download = FakeDownload::new(TOOL_BYTES);
        let result = fetch_tool_with(
            &lock("1.2.3", "win-64"),
            "pixi",
            "win-64",
            &cache,
            &download,
        )
        .unwrap();
        assert_eq!(result.path, cache.join("pixi-1.2.3-win-64.exe"));
        assert_eq!(
            result.url,
            "https://tools.invalid/v1.2.3/x86_64-pc-windows-msvc.exe"
        );
    }
}

#[rstest]
#[case::relative(false)]
#[case::absolute(true)]
fn an_override_resolves_relative_to_the_project_or_as_an_absolute_path(
    isolated_home: TempDir,
    #[case] absolute: bool,
) {
    let path = isolated_home.path().join("reviewed.json");
    std::fs::write(&path, r#"{"schema":1,"tools":{"reviewed-helper":{"version":"7.8.9","url_template":"https://mirror.invalid/{version}/{target}","platforms":{}}}}"#).unwrap();
    let argument = if absolute {
        path
    } else {
        std::path::PathBuf::from("reviewed.json")
    };
    let lock =
        pixi_sandbox::tool_fetch::resolve_lock(isolated_home.path(), Some(&argument)).unwrap();
    assert_eq!(lock.names(), ["reviewed-helper"]);
    assert_eq!(lock.tools["reviewed-helper"].version, "7.8.9");
}

#[rstest]
fn no_override_selects_the_complete_embedded_catalogue(isolated_home: TempDir) {
    let lock = pixi_sandbox::tool_fetch::resolve_lock(isolated_home.path(), None).unwrap();
    assert_eq!(lock.names(), ["pixi", "pixi-pack", "pixi-unpack"]);
}

#[rstest]
fn an_explicit_cache_needs_no_home(isolated_home: TempDir) {
    let path = isolated_home.path().join("explicit-cache");
    assert_eq!(
        pixi_sandbox::tool_fetch::tools_cache(Some(&path), None).unwrap(),
        path
    );
    assert!(!path.exists(), "cache selection does not create it");
}

#[rstest]
fn a_default_cache_uses_the_supplied_home(isolated_home: TempDir) {
    let path = pixi_sandbox::tool_fetch::tools_cache(None, Some(isolated_home.path().as_os_str()))
        .unwrap();
    assert_eq!(path, isolated_home.path().join(".cache/pixi-sandbox/tools"));
    assert!(!path.exists());
}

#[test]
fn a_default_cache_without_a_home_explains_the_missing_input() {
    let error = pixi_sandbox::tool_fetch::tools_cache(None, None).unwrap_err();
    assert_eq!(
        error.to_string(),
        "cannot choose a tools cache: HOME is not set"
    );
}

#[rstest]
fn embedding_preserves_verified_bytes_and_pin_metadata(isolated_home: TempDir) {
    let path = isolated_home.path().join("input.exe");
    std::fs::write(&path, b"MZ test binary").unwrap();
    let digest = "c2ada0229a549c814119ca32e80eec8520bfcca24991e63f4268d94d4c4f1407";
    let source = ToolSource {
        path,
        version: "1.2.3".into(),
        url: Some("https://mirror.invalid/reviewed".into()),
        pinned_sha256: Some(digest.into()),
    };
    let payload = isolated_home.path().join("payload");
    let entry = embed_tool(&payload, "win-64", "pixi", &source, 95 * 1024 * 1024).unwrap();
    assert_eq!(
        std::fs::read(payload.join("tools/win-64/pixi.exe")).unwrap(),
        b"MZ test binary"
    );
    assert_eq!(entry.size_bytes, 14);
    assert_eq!(entry.version, "1.2.3");
    assert_eq!(entry.url, source.url);
    assert_eq!(entry.pinned_sha256.as_deref(), Some(digest));
    // This short input predates extraction: the classifier needs 20 bytes, so it is unknown.
    assert_eq!(entry.linkage, "unknown");
}

#[rstest]
#[case::pin_mismatch(
    Some("0000000000000000000000000000000000000000000000000000000000000000"),
    u64::MAX,
    "does not match its pin"
)]
#[case::too_large(None, 13, "tools cannot be split")]
fn an_unverified_or_oversized_embedded_tool_is_refused(
    isolated_home: TempDir,
    #[case] pin: Option<&str>,
    #[case] limit: u64,
    #[case] refusal: &str,
) {
    let source = isolated_home.path().join("input");
    std::fs::write(&source, b"MZ test binary").unwrap();
    let error = embed_tool(
        isolated_home.path(),
        "win-64",
        "pixi",
        &ToolSource {
            path: source,
            version: "test".into(),
            url: None,
            pinned_sha256: pin.map(str::to_owned),
        },
        limit,
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains(refusal), "{error:#}");
}

#[rstest]
fn a_dynamic_tool_names_the_static_asset_remedy(isolated_home: TempDir) {
    let source = support::crate_dir().join("tests/fixtures/tool-fetch/dynamic-elf");
    let error = embed_tool(
        isolated_home.path(),
        "linux-64",
        "pixi",
        &ToolSource {
            path: source,
            version: "test".into(),
            url: None,
            pinned_sha256: None,
        },
        u64::MAX,
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("dynamically linked"),
        "{error:#}"
    );
    assert!(
        format!("{error:#}").contains("static release asset"),
        "{error:#}"
    );
}

#[cfg(unix)]
#[rstest]
#[case::version("#!/bin/sh\necho 'tool 1.2.3'\n", "1.2.3")]
#[case::empty("#!/bin/sh\nexit 0\n", "unknown")]
fn version_reporting_keeps_the_last_token_or_unknown(
    isolated_home: TempDir,
    #[case] script: &str,
    #[case] expected: &str,
) {
    let path = isolated_home.path().join("tool");
    support::write_executable(&path, script);
    assert_eq!(
        pixi_sandbox::tool_fetch::reported_version(&path).unwrap(),
        expected
    );
}

#[cfg(unix)]
#[rstest]
fn failed_version_reporting_keeps_both_streams_and_the_tool_context(isolated_home: TempDir) {
    let path = isolated_home.path().join("tool");
    support::write_executable(
        &path,
        "#!/bin/sh\necho 'version stdout'\necho 'version stderr' >&2\nexit 7\n",
    );
    let error = pixi_sandbox::tool_fetch::reported_version(&path).unwrap_err();
    let message = format!("{error:#}");
    assert!(
        message.contains("asking") && message.contains("for its version"),
        "{message}"
    );
    assert!(
        message.contains("version stdout") && message.contains("version stderr"),
        "{message}"
    );
}
