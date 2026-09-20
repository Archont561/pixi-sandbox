//! Verification must catch a corrupted branch *before* anything is written, and must catch
//! the one mistake that only shows up on the airlock: a dynamically linked tool.

use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify::{Kind, Linkage, linkage_of, manifest_path, verify};
use std::fs;
use std::path::Path;

const BLOB_BODY: &[u8] = b"conda-payload";
/// A blob that travelled as two `.partNNN` pieces (the whole file is not on the branch).
const SPLIT_PARTS: [&[u8]; 2] = [b"AAA", b"BBB"];

fn manifest_json(sha: &str, size: usize) -> String {
    let split = sha256_of(&[SPLIT_PARTS[0], SPLIT_PARTS[1]].concat());
    let p0 = sha256_of(SPLIT_PARTS[0]);
    let p1 = sha256_of(SPLIT_PARTS[1]);
    format!(
        r#"{{
  "schema": 1,
  "tool": {{ "name": "pixi-sandbox", "version": "0.1.0" }},
  "created_at": "2026-09-20T12:00:00Z",
  "platform": "linux-64",
  "shard_limit_bytes": 99614720,
  "source": {{ "commit": null, "lock_sha256": null }},
  "tools": {{}},
  "envs": {{ "dev": {{
    "platform": "linux-64",
    "pack_path": ".pixi-sandbox/envs/dev/pack",
    "packed_size_bytes": {size},
    "unpacked_size_bytes": 4096,
    "pixi_environment_fingerprint": "0123456789abcdef",
    "blobs": [
      {{ "path": "envs/dev/pack/channel/noarch/a.conda", "size": {size}, "sha256": "{sha}" }},
      {{ "path": "envs/dev/pack/channel/noarch/big.conda", "size": 6, "sha256": "{split}",
        "parts": [
          {{ "path": "envs/dev/pack/channel/noarch/big.conda.part000", "size": 3, "sha256": "{p0}" }},
          {{ "path": "envs/dev/pack/channel/noarch/big.conda.part001", "size": 3, "sha256": "{p1}" }}
        ] }}
    ]
  }} }}
}}"#
    )
}

/// A transport dir whose single blob matches the given digest.
fn transport(dir: &Path, sha: &str) -> Manifest {
    let pack = dir.join(".pixi-sandbox/envs/dev/pack/channel/noarch");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("a.conda"), BLOB_BODY).unwrap();
    fs::write(pack.join("big.conda.part000"), SPLIT_PARTS[0]).unwrap();
    fs::write(pack.join("big.conda.part001"), SPLIT_PARTS[1]).unwrap();
    let manifest: Manifest = serde_json::from_str(&manifest_json(sha, BLOB_BODY.len())).unwrap();
    fs::write(
        manifest_path(dir),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    manifest
}

fn sha256_of(bytes: &[u8]) -> String {
    pixi_sandbox_core::shard::sha256_bytes(bytes)
}

#[test]
fn manifest_path_points_into_the_pixi_sandbox_directory() {
    let path = manifest_path(Path::new("/some/branch"));
    assert!(
        path.ends_with(".pixi-sandbox/manifest.json"),
        "got {path:?}"
    );
}

#[test]
fn a_good_transport_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = transport(dir.path(), &sha256_of(BLOB_BODY));
    let report = verify(&manifest, dir.path(), None);
    assert!(report.ok(), "unexpected failures: {:?}", report.failures);
    // two logical blobs: one whole file, one that arrived as two parts
    assert_eq!(report.files, 2);
    assert_eq!(report.bytes, BLOB_BODY.len() as u64 + 6);
    assert!(
        !report.failures.iter().any(|f| f.path.contains("big.conda")),
        "a split blob whose parts are present must not be reported as missing"
    );
}

#[test]
fn a_tampered_blob_is_reported_as_an_integrity_failure() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = transport(dir.path(), &sha256_of(BLOB_BODY));
    // same length on purpose: the size check must not mask the digest check
    fs::write(
        dir.path()
            .join(".pixi-sandbox/envs/dev/pack/channel/noarch/a.conda"),
        b"conda-PAYLOAD",
    )
    .unwrap();

    let report = verify(&manifest, dir.path(), None);
    assert!(!report.ok());
    assert_eq!(report.failures[0].kind, Kind::Integrity);
    assert!(report.failures[0].path.ends_with("a.conda"));
}

#[test]
fn a_missing_blob_is_reported_as_missing() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = transport(dir.path(), &sha256_of(BLOB_BODY));
    fs::remove_file(
        dir.path()
            .join(".pixi-sandbox/envs/dev/pack/channel/noarch/a.conda"),
    )
    .unwrap();

    let report = verify(&manifest, dir.path(), None);
    assert!(!report.ok());
    assert_eq!(report.failures[0].kind, Kind::Missing);
}

#[test]
fn a_script_is_not_a_dynamic_binary() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("pixi-sandbox");
    fs::write(&script, b"#!/usr/bin/env python3\nprint('prototype')\n").unwrap();
    assert_eq!(linkage_of(&script), Linkage::Script); // the reference prototype ships this way
}

#[test]
fn elf_without_an_interpreter_is_static_and_with_one_is_dynamic() {
    let dir = tempfile::tempdir().unwrap();

    let dynamic = dir.path().join("dynamic");
    fs::write(&dynamic, elf_with_ph(3 /* PT_INTERP */)).unwrap();
    assert_eq!(linkage_of(&dynamic), Linkage::Dynamic);

    let stat = dir.path().join("static");
    fs::write(&stat, elf_with_ph(1 /* PT_LOAD */)).unwrap();
    assert_eq!(linkage_of(&stat), Linkage::Static);
}

/// Minimal 64-bit little-endian ELF: header + one program header of type `p_type`.
fn elf_with_ph(p_type: u32) -> Vec<u8> {
    let mut elf = vec![0u8; 64 + 56];
    elf[..4].copy_from_slice(b"\x7fELF");
    elf[4] = 2; // 64-bit
    elf[5] = 1; // little-endian
    elf[6] = 1; // version
    elf[16..18].copy_from_slice(&3u16.to_le_bytes()); // ET_DYN
    elf[18..20].copy_from_slice(&0x3eu16.to_le_bytes()); // x86-64
    elf[20..24].copy_from_slice(&1u32.to_le_bytes());
    elf[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
    elf[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
    elf[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
    elf[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
    elf[64..68].copy_from_slice(&p_type.to_le_bytes());
    elf
}
