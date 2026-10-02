//! Unit tests for `SHA256SUMS` parsing and verification (`src/self_update/checksums.rs`).

use pixi_sandbox::self_update::checksums::*;

fn sums_for(name: &str, bytes: &[u8]) -> String {
    format!(
        "{}  {name}\n",
        pixi_sandbox_core::shard::sha256_bytes(bytes)
    )
}

#[test]
fn a_matching_digest_verifies_and_returns_it() {
    let body = sums_for("pixi-sandbox-x86_64-unknown-linux-musl", b"new binary");
    let sums = Checksums::parse(&body);
    let digest = sums
        .verify("pixi-sandbox-x86_64-unknown-linux-musl", b"new binary")
        .expect("digest matches");
    assert_eq!(
        digest,
        pixi_sandbox_core::shard::sha256_bytes(b"new binary")
    );
}

#[test]
fn a_missing_entry_is_refused_and_lists_what_the_release_did_cover() {
    let body = sums_for("pixi-sandbox-aarch64-apple-darwin", b"other host");
    let sums = Checksums::parse(&body);
    let err = sums
        .verify("pixi-sandbox-x86_64-unknown-linux-musl", b"anything")
        .unwrap_err()
        .to_string();
    assert!(err.contains("no entry for"), "{err}");
    assert!(err.contains("pixi-sandbox-aarch64-apple-darwin"), "{err}");
    assert!(err.contains("unverified"), "{err}");
}

#[test]
fn a_tampered_download_is_refused_and_both_digests_are_shown() {
    let body = sums_for("asset", b"published bytes");
    let sums = Checksums::parse(&body);
    let err = sums
        .verify("asset", b"tampered bytes")
        .unwrap_err()
        .to_string();
    assert!(err.contains("checksum mismatch"), "{err}");
    assert!(err.contains("nothing was written"), "{err}");
    assert!(
        err.contains(&pixi_sandbox_core::shard::sha256_bytes(b"published bytes")),
        "the expected digest must appear: {err}"
    );
}

#[test]
fn an_empty_or_unparsable_file_is_refused_rather_than_treated_as_no_constraint() {
    let sums = Checksums::parse("404: Not Found\n");
    assert!(sums.is_empty());
    let err = sums.verify("asset", b"bytes").unwrap_err().to_string();
    assert!(err.contains("empty or unparsable"), "{err}");
}

#[test]
fn the_real_release_checksums_shape_parses_every_standalone_binary() {
    // Exactly the shape `xtask release-checksums` writes: `<digest>  <name>`, two spaces.
    let mut body = String::new();
    for (_, _, name) in pixi_sandbox::self_update::assets::SUPPORTED_HOSTS {
        body.push_str(&sums_for(name, name.as_bytes()));
    }
    let sums = Checksums::parse(&body);
    assert_eq!(sums.names().len(), 5);
    for (_, _, name) in pixi_sandbox::self_update::assets::SUPPORTED_HOSTS {
        sums.verify(name, name.as_bytes())
            .expect("each row verifies");
    }
}
