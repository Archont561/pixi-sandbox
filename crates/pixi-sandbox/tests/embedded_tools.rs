//! Explicit executable embedding and standalone version checks.
mod support;
use pixi_sandbox::embedded_tools::{ToolSource, embed_tool};
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
    // Independently calculated with sha256sum: an explicit local tool still needs
    // byte-integrity verification, even though no upstream catalogue supplied its hash.
    assert_eq!(
        entry.pinned_sha256.as_deref(),
        Some("c2ada0229a549c814119ca32e80eec8520bfcca24991e63f4268d94d4c4f1407")
    );
    assert!(isolated_home.path().join("tools/win-64/pixi.exe").is_file());
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
        pixi_sandbox::embedded_tools::reported_version(&path).unwrap(),
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
    let error = pixi_sandbox::embedded_tools::reported_version(&path).unwrap_err();
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
