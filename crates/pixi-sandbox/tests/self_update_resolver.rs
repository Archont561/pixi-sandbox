//! Unit tests for release resolution (`src/self_update/resolver.rs`).

use anyhow::Result;
use pixi_sandbox::release::{Asset, ReleaseSource};
use pixi_sandbox::self_update::resolver::*;

use rstest::rstest;
use std::collections::BTreeMap;

struct OneRelease {
    latest: Option<String>,
}

impl ReleaseSource for OneRelease {
    fn latest_tag(&self, repo: &str) -> Result<String> {
        self.latest
            .clone()
            .ok_or_else(|| anyhow::anyhow!("no releases published for {repo}"))
    }
    fn asset(&self, _repo: &str, _tag: &str, _name: &str) -> Result<Asset> {
        unreachable!("the resolver downloads nothing")
    }
    fn published_checksums(
        &self,
        _repo: &str,
        _tag: &str,
    ) -> Result<Option<BTreeMap<String, String>>> {
        Ok(None)
    }
}

fn source(latest: Option<&str>) -> OneRelease {
    OneRelease {
        latest: latest.map(str::to_string),
    }
}

#[test]
fn no_requested_version_resolves_latest_and_says_so() {
    let resolved = resolve(&source(Some("v0.4.4")), "o/r", None).expect("resolved");
    assert_eq!(resolved.tag, "v0.4.4");
    assert_eq!(resolved.version, "0.4.4");
    assert_eq!(resolved.selection, Selection::Latest);
    assert_eq!(resolved.selection.describe(), "latest");
}

#[test]
fn an_exact_version_never_asks_the_source_for_latest() {
    // `source(None)` errors on latest_tag, so a pinned resolve that touches it would fail.
    let resolved = resolve(&source(None), "o/r", Some("0.3.7")).expect("resolved");
    assert_eq!(resolved.tag, "v0.3.7");
    assert_eq!(resolved.version, "0.3.7");
    assert_eq!(resolved.selection, Selection::Pinned);
    assert_eq!(resolved.selection.describe(), "pinned");
}

#[test]
fn a_v_prefixed_request_is_accepted_and_not_double_prefixed() {
    let resolved = resolve(&source(None), "o/r", Some("v1.2.3")).expect("resolved");
    assert_eq!(resolved.tag, "v1.2.3");
    assert_eq!(resolved.version, "1.2.3");
}

#[test]
fn an_http_failure_resolving_latest_is_reported_with_the_repository() {
    let err = resolve(&source(None), "owner/name", None).unwrap_err();
    let text = format!("{err:?}");
    assert!(text.contains("owner/name"), "{text}");
    assert!(text.contains("latest"), "{text}");
}

#[test]
fn a_latest_tag_that_is_not_a_release_version_is_an_error_not_a_download() {
    let err = resolve(&source(Some("nightly")), "owner/name", None).unwrap_err();
    let text = format!("{err:?}");
    assert!(text.contains("nightly"), "{text}");
}

#[rstest]
#[case("0.4.4")]
#[case("v0.4.4")]
#[case("1.0.0")]
#[case("10.20.30")]
#[case("0.0.0")]
fn exact_versions_are_accepted(#[case] input: &str) {
    normalise_version(input).expect("accepted");
}

#[rstest]
#[case("0.4")]
#[case("0.4.4.1")]
#[case("0.4.x")]
#[case("latest")]
#[case("")]
#[case("v")]
#[case("0.04.4")]
#[case("0.4.4-rc1")]
#[case(" 0.4.4")]
fn anything_that_is_not_an_exact_release_version_is_refused(#[case] input: &str) {
    let err = normalise_version(input).unwrap_err().to_string();
    assert!(err.contains("exact version"), "{input:?}: {err}");
}
