//! The manifest is a wire format: these tests freeze its shape and its validation rules,
//! and they check the repository's *own* pin file for completeness.

use pixi_sandbox_core::host_requirements::{HostCapability, HostRequirementSet};
use pixi_sandbox_core::manifest::{Manifest, SCHEMA_VERSION};
use pixi_sandbox_core::tools_lock::ToolsLock;
use proptest::prelude::*;
use rstest::rstest;

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
      ],
      "files": {{
        "blob": {{ "path": "envs/dev/files.json", "size": 512,
                  "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" }},
        "entries": 12
      }}
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
    assert_eq!(
        round_trip.envs["dev"].files.as_ref().unwrap().entries,
        12,
        "the per-file oracle must survive a round trip"
    );
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Any valid wire document still validates after JSON serialisation and parsing. Varying
    /// independently-owned scalar fields catches accidental omissions from serde derives and
    /// validation paths that only work for the hand-written fixture.
    #[test]
    fn a_valid_manifest_json_round_trips(version in 0u16..10_000, commit in "[0-9a-f]{1,16}") {
        let text = valid_manifest()
            .replace("\"version\": \"0.1.0\"", &format!("\"version\": \"{version}.0\""))
            .replace("\"commit\": \"deadbeef\"", &format!("\"commit\": \"{commit}\""));
        let manifest = parse(&text);
        prop_assert!(manifest.validate().is_ok());

        let encoded = serde_json::to_value(&manifest).expect("manifest serialises");
        let decoded: Manifest = serde_json::from_value(encoded.clone()).expect("manifest parses");
        prop_assert!(decoded.validate().is_ok());
        prop_assert_eq!(serde_json::to_value(decoded).unwrap(), encoded);
    }
}

/// Issue #109 / TASK-75 AC#2: the host requirements a bundle declared travel in the transport,
/// additively — a manifest without the section stays valid and byte-identical, and a reader
/// that predates the field ignores it rather than refusing the branch.
#[test]
fn host_requirements_are_carried_additively_and_absent_by_default() {
    let plain = parse(&valid_manifest());
    assert_eq!(plain.host_requirements, None);
    let encoded = serde_json::to_value(&plain).unwrap();
    assert!(
        encoded.get("host_requirements").is_none(),
        "a project that declares nothing must not gain a key: {encoded}"
    );

    // A manifest that carries the section parses, validates and round-trips unchanged.
    let text = valid_manifest().replace(
        "\"envs\": {",
        "\"host_requirements\": {\n    \"libc\": \">=2.34\",\n    \"packages\": [\"fontconfig\", \"xvfb\"],\n    \"services\": [\"dbus\"],\n    \"capabilities\": [\"display\"],\n    \"headless\": [\"xvfb-run\"]\n  },\n  \"envs\": {",
    );
    let manifest = parse(&text);
    manifest
        .validate()
        .expect("the section must not break validation");
    let host = manifest.host_requirements.as_ref().unwrap();
    assert_eq!(host.libc.as_deref(), Some(">=2.34"));
    assert_eq!(host.packages, ["fontconfig", "xvfb"].map(String::from));
    assert_eq!(host.services, ["dbus"].map(String::from));
    assert_eq!(host.capabilities, [HostCapability::Display]);
    assert_eq!(host.headless, ["xvfb-run"].map(String::from));

    let round_trip: Manifest =
        serde_json::from_str(&serde_json::to_string(&manifest).unwrap()).unwrap();
    assert_eq!(round_trip.host_requirements, manifest.host_requirements);

    // The summary both human surfaces render.
    assert_eq!(
        host.summary(),
        "libc >=2.34 · packages fontconfig, xvfb · services dbus · capabilities display · \
         headless xvfb-run"
    );
}

/// The backward-compatibility half stated the other way round: an *older* reader meets the key
/// as unknown data and ignores it, because the manifest is deliberately not
/// `deny_unknown_fields` — a published branch outlives the binary that wrote it.
#[test]
fn an_unknown_manifest_key_is_ignored_rather_than_refused() {
    let text = valid_manifest().replace(
        "\"envs\": {",
        "\"host_requirements_from_the_future\": { \"whatever\": true },\n  \"envs\": {",
    );
    let manifest = parse(&text);
    manifest
        .validate()
        .expect("an unknown key must not break a restore");
}

#[test]
fn the_host_summary_omits_kinds_that_are_not_declared() {
    let set: HostRequirementSet =
        serde_json::from_str(r#"{ "packages": ["fontconfig"] }"#).unwrap();
    assert_eq!(set.summary(), "packages fontconfig");
    assert_eq!(HostRequirementSet::default().summary(), "");
}

#[test]
fn unknown_schema_is_refused() {
    let text = valid_manifest().replace(&format!("\"schema\": {SCHEMA_VERSION}"), "\"schema\": 99");
    let err = parse(&text).validate().expect_err("must refuse");
    assert!(err.to_string().contains("schema 99"), "got: {err}");
}

/// A published branch outlives the binary that packed it: schema 1 transports (everything
/// published before the per-file oracle existed) must stay restorable, with the oracle simply
/// absent. Only a schema newer than this build is refused.
#[test]
fn an_older_schema_still_validates_without_the_file_oracle() {
    let text = valid_manifest()
        .replace(&format!("\"schema\": {SCHEMA_VERSION}"), "\"schema\": 1")
        .replace(
            // operates on the rendered JSON (single braces), not on the format! source
            r#",
      "files": {
        "blob": { "path": "envs/dev/files.json", "size": 512,
                  "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" },
        "entries": 12
      }"#,
            "",
        );
    let manifest = parse(&text);
    manifest.validate().expect("schema 1 must stay loadable");
    assert_eq!(manifest.schema, 1);
    assert!(
        manifest.envs["dev"].files.is_none(),
        "a schema-1 env carries no oracle; verify_restored must say so, not guess"
    );
}

#[test]
fn a_files_manifest_with_a_bad_path_entries_or_digest_is_refused() {
    // escaping list path
    let escaping = valid_manifest().replace("envs/dev/files.json", "../outside/files.json");
    assert!(
        parse(&escaping)
            .validate()
            .expect_err("an escaping files path must be refused")
            .to_string()
            .contains("escapes")
    );

    // zero entries: an oracle that lists nothing checks nothing
    let empty = valid_manifest().replace("\"entries\": 12", "\"entries\": 0");
    assert!(
        parse(&empty)
            .validate()
            .expect_err("zero entries must be refused")
            .to_string()
            .contains("no entries")
    );

    // a list digest that is not a sha256
    let bad_digest = valid_manifest().replace(
        r#""blob": { "path": "envs/dev/files.json", "size": 512,
                  "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" },"#,
        r#""blob": { "path": "envs/dev/files.json", "size": 512, "sha256": "nope" },"#,
    );
    assert!(
        parse(&bad_digest)
            .validate()
            .expect_err("a non-sha256 list digest must be refused")
            .to_string()
            .contains("sha256")
    );
}

#[test]
fn a_windows_drive_prefix_is_refused_wherever_a_path_appears() {
    // A colon alone is a legal POSIX byte (issue #95); a Windows drive prefix is what the
    // relative-path guard exists to catch, in every position it guards.
    let cases = [
        // (label, needle, drive-prefixed replacement)
        (
            "blob path",
            "\"path\": \"envs/dev/pack/channel/noarch/a.conda\"",
            "\"path\": \"C:/envs/dev/pack/channel/noarch/a.conda\"",
        ),
        (
            "pack_path",
            ".pixi-sandbox/envs/dev/pack",
            "E:.pixi-sandbox/envs/dev/pack",
        ),
        (
            "tool path",
            "\"path\": \"tools/linux-64/pixi-unpack\"",
            "\"path\": \"C:/tools/linux-64/pixi-unpack\"",
        ),
    ];
    for (label, needle, replacement) in cases {
        let text = valid_manifest().replace(needle, replacement);
        let err = match parse(&text).validate() {
            Ok(()) => panic!("{label}: a drive prefix must be refused"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("must be relative"),
            "{label}: got {err}"
        );
    }

    // part path: a well-formed parts array (summing to the blob size) whose one part
    // carries the drive prefix — the sum check passes, the part path must not.
    let part = valid_manifest().replace(
        r#""size": 4,
           "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08""#,
        r#""size": 4,
           "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
           "parts": [ { "path": "C:\\envs\\dev\\part000", "size": 4,
                        "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08" } ]"#,
    );
    let err = parse(&part)
        .validate()
        .expect_err("a drive-prefixed part path must be refused");
    assert!(
        err.to_string().contains("must be relative"),
        "part path: got {err}"
    );

    // absolute paths stay refused by the same guard, colon or not
    let absolute = valid_manifest().replace(
        "\"path\": \"envs/dev/pack/channel/noarch/a.conda\"",
        "\"path\": \"/etc/passwd\"",
    );
    let err = parse(&absolute)
        .validate()
        .expect_err("an absolute blob path must be refused");
    assert!(
        err.to_string().contains("must be relative"),
        "absolute: got {err}"
    );
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
#[rstest]
#[case("pixi-pack", "linux-64")]
#[case("pixi-pack", "linux-aarch64")]
#[case("pixi-pack", "osx-64")]
#[case("pixi-pack", "osx-arm64")]
#[case("pixi-pack", "win-64")]
#[case("pixi-unpack", "linux-64")]
#[case("pixi-unpack", "linux-aarch64")]
#[case("pixi-unpack", "osx-64")]
#[case("pixi-unpack", "osx-arm64")]
#[case("pixi-unpack", "win-64")]
#[case("pixi", "linux-64")]
#[case("pixi", "linux-aarch64")]
#[case("pixi", "osx-64")]
#[case("pixi", "osx-arm64")]
#[case("pixi", "win-64")]
fn the_embedded_tool_pins_are_valid_and_complete(#[case] tool: &str, #[case] platform: &str) {
    let lock = ToolsLock::embedded().expect("embedded tools lock must parse");
    let pin = lock
        .pin(tool, platform)
        .unwrap_or_else(|| panic!("{tool} must be pinned for {platform}"));
    assert_eq!(pin.sha256.len(), 64, "{tool}/{platform}: sha256");
    let url = lock.url(tool, platform).expect("url");
    assert!(url.starts_with("https://github.com/"), "got {url}");
    assert!(!url.contains('{'), "unsubstituted template: {url}");

    // The linux-64 unpacker is the static musl asset (decisions D4) — the whole airlock story
    // rests on that one asset.
    if (tool, platform) == ("pixi-unpack", "linux-64") {
        assert_eq!(pin.linkage, "static");
        assert_eq!(pin.target, "x86_64-unknown-linux-musl");
        assert_eq!(
            pin.sha256,
            "8191f586b734e634e2f1644e553dcb8e07718d0bafbfefb65c5e6e22b4d484b7"
        );
    }
}
