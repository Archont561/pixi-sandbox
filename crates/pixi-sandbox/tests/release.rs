//! Unit tests for the shared release transport (`src/release.rs`).
//!
//! Tests live under `tests/`, never beside the source they judge (AGENTS.md, Test conventions).

use pixi_sandbox::release::*;

const DIGEST: &str = "1111111111111111111111111111111111111111111111111111111111111111";

#[test]
fn both_manifest_spellings_parse_and_junk_lines_are_ignored() {
    let body = format!("{DIGEST}  coreutils-order\nreversed-order {DIGEST}\nnot a checksum\n");
    let map = parse_sha256_manifest(&body);
    assert_eq!(map.get("coreutils-order"), Some(&DIGEST.to_string()));
    assert_eq!(map.get("reversed-order"), Some(&DIGEST.to_string()));
    assert_eq!(map.len(), 2, "the junk line must not become an entry");
}

#[test]
fn a_sha256_is_exactly_sixty_four_hex_digits() {
    assert!(is_sha256(DIGEST));
    assert!(!is_sha256(&DIGEST[..63]));
    assert!(!is_sha256(&format!("{DIGEST}0")));
    assert!(!is_sha256(&"z".repeat(64)));
}

#[test]
fn an_asset_hashes_its_own_bytes() {
    let asset = Asset {
        bytes: b"payload".to_vec(),
    };
    assert_eq!(
        asset.sha256(),
        pixi_sandbox_core::shard::sha256_bytes(b"payload")
    );
}
