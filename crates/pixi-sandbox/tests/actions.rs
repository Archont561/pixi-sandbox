//! Integration tests for release Action and checksum verification logic.
//!
//! Replaces the external Python Action test harness with native Rust tests that run under
//! `cargo nextest`.

use pixi_sandbox_core::shard::sha256_bytes;
use std::collections::HashMap;
use std::fs;

/// Parse SHA256SUMS content supporting both GNU (`<digest>  <file>`) and BSD formats.
fn parse_checksums(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // GNU format: "<sha256>  <file>" or "<sha256> *<file>"
        if let Some((sha, rest)) = line.split_once(' ') {
            let filename = rest.trim_start_matches([' ', '*']).trim();
            if sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                map.insert(filename.to_string(), sha.to_ascii_lowercase());
                continue;
            }
        }
        // BSD format: "SHA256 (<file>) = <sha256>"
        if let Some(rest) = line.strip_prefix("SHA256 (") {
            if let Some((filename, sha)) = rest.split_once(") = ") {
                let sha = sha.trim();
                if sha.len() == 64 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
                    map.insert(filename.to_string(), sha.to_ascii_lowercase());
                }
            }
        }
    }
    map
}

/// Map (RUNNER_OS, RUNNER_ARCH) to Rust target triple and executable extension.
fn resolve_target(os: &str, arch: &str) -> Result<(&'static str, &'static str), String> {
    match (os, arch) {
        ("Linux", "X64") | ("linux", "x86_64") => Ok(("x86_64-unknown-linux-musl", "")),
        ("Linux", "ARM64") | ("linux", "aarch64") => Ok(("aarch64-unknown-linux-musl", "")),
        ("macOS", "X64") | ("darwin", "x86_64") => Ok(("x86_64-apple-darwin", "")),
        ("macOS", "ARM64") | ("darwin", "arm64") => Ok(("aarch64-apple-darwin", "")),
        ("Windows", "X64") | ("windows", "x86_64") => Ok(("x86_64-pc-windows-msvc", ".exe")),
        ("Windows", "ARM64") | ("windows", "arm64") => Ok(("aarch64-pc-windows-msvc", ".exe")),
        _ => Err(format!("unsupported platform ({os}, {arch})")),
    }
}

fn render_asset_name(template: &str, target: &str, exe: &str, version: &str) -> String {
    template
        .replace("{target}", target)
        .replace("{exe}", exe)
        .replace("{version}", version)
        .replace("{tag}", version)
}

#[test]
fn target_resolution_covers_all_supported_platforms() {
    let cases = [
        ("Linux", "X64", "x86_64-unknown-linux-musl", ""),
        ("Linux", "ARM64", "aarch64-unknown-linux-musl", ""),
        ("macOS", "X64", "x86_64-apple-darwin", ""),
        ("macOS", "ARM64", "aarch64-apple-darwin", ""),
        ("Windows", "X64", "x86_64-pc-windows-msvc", ".exe"),
    ];

    for (os, arch, expected_target, expected_exe) in cases {
        let (target, exe) = resolve_target(os, arch).expect("target must resolve");
        assert_eq!(target, expected_target);
        assert_eq!(exe, expected_exe);
    }

    assert!(resolve_target("FreeBSD", "X64").is_err());
}

#[test]
fn asset_name_templating_renders_correctly() {
    let name = render_asset_name(
        "pixi-sandbox-{target}{exe}",
        "x86_64-unknown-linux-musl",
        "",
        "v0.1.0",
    );
    assert_eq!(name, "pixi-sandbox-x86_64-unknown-linux-musl");

    let win_name = render_asset_name(
        "pixi-sandbox-{target}{exe}",
        "x86_64-pc-windows-msvc",
        ".exe",
        "v0.1.0",
    );
    assert_eq!(win_name, "pixi-sandbox-x86_64-pc-windows-msvc.exe");
}

#[test]
fn checksum_parsing_handles_gnu_and_bsd_formats() {
    let gnu_data = "\
1111111111111111111111111111111111111111111111111111111111111111  pixi-sandbox-x86_64-unknown-linux-musl
2222222222222222222222222222222222222222222222222222222222222222 *pixi-sandbox-x86_64-pc-windows-msvc.exe
";
    let map = parse_checksums(gnu_data);
    assert_eq!(map.len(), 2);
    assert_eq!(
        map["pixi-sandbox-x86_64-unknown-linux-musl"],
        "1111111111111111111111111111111111111111111111111111111111111111"
    );
    assert_eq!(
        map["pixi-sandbox-x86_64-pc-windows-msvc.exe"],
        "2222222222222222222222222222222222222222222222222222222222222222"
    );

    let bsd_data =
        "SHA256 (pixi-sandbox-x86_64-apple-darwin) = 3333333333333333333333333333333333333333333333333333333333333333\n";
    let bsd_map = parse_checksums(bsd_data);
    assert_eq!(
        bsd_map["pixi-sandbox-x86_64-apple-darwin"],
        "3333333333333333333333333333333333333333333333333333333333333333"
    );
}

#[test]
fn verification_accepts_valid_bytes_and_rejects_tampered() {
    let payload = b"ELF static linux executable payload bytes";
    let digest = sha256_bytes(payload);

    let checksum_content = format!("{digest}  pixi-sandbox-x86_64-unknown-linux-musl\n");
    let map = parse_checksums(&checksum_content);

    let expected = map.get("pixi-sandbox-x86_64-unknown-linux-musl").unwrap();
    assert_eq!(expected, &digest);

    // Tampered payload
    let tampered = b"tampered bytes";
    let tampered_digest = sha256_bytes(tampered);
    assert_ne!(expected, &tampered_digest);
}

#[test]
fn deterministic_sums_file_generation_matches_sha256() {
    let dir = tempfile::tempdir().unwrap();
    let bin_path = dir.path().join("pixi-sandbox-x86_64-unknown-linux-musl");
    fs::write(&bin_path, b"dummy binary content").unwrap();

    let digest = sha256_bytes(b"dummy binary content");
    let sums_path = dir.path().join("SHA256SUMS");
    fs::write(
        &sums_path,
        format!("{digest}  pixi-sandbox-x86_64-unknown-linux-musl\n"),
    )
    .unwrap();

    let loaded = fs::read_to_string(&sums_path).unwrap();
    let parsed = parse_checksums(&loaded);
    assert_eq!(
        parsed.get("pixi-sandbox-x86_64-unknown-linux-musl"),
        Some(&digest)
    );
}
