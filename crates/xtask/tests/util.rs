//! `util.rs` — the shared primitives (moved from the module's inline `#[cfg(test)]`, TASK-83:
//! names kept, bodies verbatim).

use std::fs;
use xtask::util::{append_line, is_strict_semver, lines_without_opt_out, version_tag_re};

#[test]
fn workflow_file_appends_land_as_lines_and_create_the_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let output = dir.path().join("nested").join("out");
    fs::create_dir_all(output.parent().expect("parent")).expect("mkdir");

    append_line(&output, "version=v0.3.7");
    append_line(&output, "later=again");

    assert_eq!(
        fs::read_to_string(&output).expect("output file"),
        "version=v0.3.7\nlater=again\n",
        "one line per call, created on first use"
    );
}

#[test]
fn opt_out_skips_the_line_and_the_line_below_the_marker() {
    let text = "one\nstale-ref-allowed\ntwo\nthree";
    let kept: Vec<_> = lines_without_opt_out(text).collect();
    assert_eq!(kept, vec![(1, "one"), (4, "three")]);
}

#[test]
fn strict_semver_rejects_prefixes_suffixes_and_missing_parts() {
    assert!(is_strict_semver("0.3.6"));
    assert!(is_strict_semver("10.20.30"));
    for bad in ["v0.3.6", "0.3", "0.3.6.1", "0.3.6-rc1", "", "a.b.c"] {
        assert!(!is_strict_semver(bad), "{bad} must be rejected");
    }
}

#[test]
fn version_tags_are_found_mid_line() {
    let caps: Vec<_> = version_tag_re()
        .find_iter("uses x@v1.2.3 and v10.0.1")
        .map(|m| m.as_str())
        .collect();
    assert_eq!(caps, vec!["v1.2.3", "v10.0.1"]);
}
