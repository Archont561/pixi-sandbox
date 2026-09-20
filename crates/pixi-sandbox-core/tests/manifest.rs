//! The manifest is a wire format: these tests freeze its shape and its validation rules,
//! and they check the repository's *own* pin file for completeness.

use pixi_sandbox_core::manifest::{Manifest, SCHEMA_VERSION};
use pixi_sandbox_core::tools_lock::ToolsLock;

/// A manifest that validates, written the way the packer writes it.
fn valid_manifest() -> String {
    format!(
        r#"{{
  "schema": {SCHEMA_VERSION},
  "tool": {{ "name": "pixi-sandbox", "version": "0.1.0" }},
  "created_at": "2026-09-20T12:00:00Z",
  "platform": "linux-64",
  "shard_limit_bytes": 99614720,
  "source": {{ "commit": "deadbeef", "lock_sha256": "aa" }},
  "tools": {{
    "pixi-unpack": {{
      "version": "0.7.11",
      "url": "https://example.invalid/unpack",
      "pinned_sha256": "8191f586b734e634e2f1644e553dcb8e07718d0bafbfefb65c5e6e22b4d484b7",
      "linkage": "static",
      "size_bytes": 4,
      "path": "tools/linux-64/pixi-unpack"
    }}
  }},
  "envs": {{
    "dev": {{
      "platform": "linux-64",
      "pack_path": ".pixi-sandbox/envs/dev/pack",
      "packed_size_bytes": 1000,
      "unpacked_size_bytes": 1000,
      "pixi_environment_fingerprint": "0123456789abcdef",
      "blobs": [
        {{ "path": "envs/dev/pack/channel/noarch/a.conda", "size": 4,
           "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" }}
      ]
    }}
  }},
  "vendor": {{ "mode": "loose", "crates": 33, "size_bytes": 2048, "cargo_lock_sha256": "bb",
    "blobs": [
      {{ "path": "vendor/serde-1.0.0/Cargo.toml", "size": 2048,
         "sha256": "7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069" }}
    ] }}
}}"#
    )
}

fn parse(text: &str) -> Manifest {
    serde_json::from_str(text).expect("fixture parses")
}

#[test]
fn a_well_formed_manifest_validates_and_summarises() {
    let manifest = parse(&valid_manifest());
    manifest.validate().expect("fixture must be valid");
    assert_eq!(manifest.schema, SCHEMA_VERSION);
    assert!(manifest.envs.contains_key("dev"));
    assert_eq!(manifest.payload_bytes(), 1000 + 4 + 2048);
    assert_eq!(manifest.payload_split(), (1000, 4, 2048));
    // vendor files are declared blobs too, so `doctor --verify` covers the crate tree
    let blobs = manifest.blobs(None);
    assert_eq!(blobs.len(), 2, "one env blob + one vendor blob");
    assert!(
        blobs
            .iter()
            .any(|(owner, blob)| *owner == "vendor" && blob.path.starts_with("vendor/")),
        "the vendor blob must be part of the verified set"
    );
    // ... and they are validated like any other reference to a file
    let broken = valid_manifest().replace("vendor/serde-1.0.0/Cargo.toml", "../escape");
    assert!(
        parse(&broken)
            .validate()
            .expect_err("an escaping vendor path must be refused")
            .to_string()
            .contains("escapes")
    );
    // the writer must produce the same field names again
    let round_trip: Manifest =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    assert_eq!(round_trip.envs.len(), 1);
}

#[test]
fn unknown_schema_is_refused() {
    let text = valid_manifest().replace(&format!("\"schema\": {SCHEMA_VERSION}"), "\"schema\": 99");
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("schema 99"), "got: {err}");
}

#[test]
fn a_blob_path_that_escapes_the_transport_is_refused() {
    let text = valid_manifest().replace(
        "\"path\": \"envs/dev/pack/channel/noarch/a.conda\"",
        "\"path\": \"../../etc/passwd\"",
    );
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("escapes"), "got: {err}");
}

#[test]
fn a_tool_path_that_escapes_the_transport_is_refused() {
    let text = valid_manifest().replace(
        "\"path\": \"tools/linux-64/pixi-unpack\"",
        "\"path\": \"../outside/pixi-unpack\"",
    );
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("escapes"), "got: {err}");
}

#[test]
fn a_digest_that_is_not_a_sha256_is_refused() {
    let text = valid_manifest().replace(
        "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
        "nope",
    );
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("sha256"), "got: {err}");
}

#[test]
fn split_parts_must_sum_to_the_blob_size() {
    let text = valid_manifest().replace(
        r#""size": 4,
           "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08""#,
        r#""size": 400,
           "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
           "parts": [ { "path": "envs/dev/pack/channel/noarch/a.conda.part000", "size": 4,
                        "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" } ]"#,
    );
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("parts sum"), "got: {err}");
}

/// The embedded catalogue is what installed binaries use. These are the tools CI downloads and
/// the airlock runs, so a missing platform is a broken release, not a warning.
#[test]
fn the_embedded_tool_pins_are_valid_and_complete() {
    let lock = ToolsLock::embedded().expect("embedded tools lock must parse");

    for (tool, platforms) in [
        (
            "pixi-pack",
            vec!["linux-64", "linux-aarch64", "osx-64", "osx-arm64", "win-64"],
        ),
        (
            "pixi-unpack",
            vec!["linux-64", "linux-aarch64", "osx-64", "osx-arm64", "win-64"],
        ),
        (
            "pixi",
            vec!["linux-64", "linux-aarch64", "osx-64", "osx-arm64", "win-64"],
        ),
    ] {
        for platform in platforms {
            let pin = lock
                .pin(tool, platform)
                .unwrap_or_else(|| panic!("{tool} must be pinned for {platform}"));
            assert_eq!(pin.sha256.len(), 64, "{tool}/{platform}: sha256");
            let url = lock.url(tool, platform).expect("url");
            assert!(url.starts_with("https://github.com/"), "got {url}");
            assert!(!url.contains('{'), "unsubstituted template: {url}");
        }
    }

    // the linux-64 unpacker is the static musl asset (decisions D4) — the whole airlock
    // story rests on that one asset
    let pin = lock.pin("pixi-unpack", "linux-64").unwrap();
    assert_eq!(pin.linkage, "static");
    assert_eq!(pin.target, "x86_64-unknown-linux-musl");
    assert_eq!(
        pin.sha256,
        "8191f586b734e634e2f1644e553dcb8e07718d0bafbfefb65c5e6e22b4d484b7"
    );
}
