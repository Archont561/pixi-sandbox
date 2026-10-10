//! `files.json` path rules, frozen from issue #95.
//!
//! A colon is a legal byte in a POSIX file name — perl's module man pages are
//! `man/man3/App::Cpan.3` — so a colon alone must not make a path non-relative. Only a
//! Windows drive prefix does (`C:/x`, `C:\x`, and the drive-relative `C:x`), because that
//! is what the guard was ever meant to catch. The incident: an environment that resolves
//! perl (the conda gtk/webkit stack) could never pack, because building the per-file
//! oracle scanned the verification-unpacked environment and rejected the man pages.

use pixi_sandbox_core::files_manifest::{
    self, ALLOWED_EXTRAS, FileEntry, FilesDoc, RESTORE_MARKERS, SENTINEL, canonical_sha256,
    canonicalise, collapse_nul_runs, list_rel_path, replace_all, scan_prefix,
};
use proptest::prelude::*;
use std::fs;
use std::path::Path;

/// The incident's tree, scanned the way pack scans it: a prefix whose perl man pages
/// carry `Package::Name.3` names.
fn scanned_perl_prefix() -> (tempfile::TempDir, FilesDoc) {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("env");
    fs::create_dir_all(prefix.join("man/man3")).unwrap();
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::write(prefix.join("man/man3/App::Cpan.3"), "doc stub\n").unwrap();
    fs::write(prefix.join("bin/tool"), "#!/bin/sh\necho hi\n").unwrap();
    let staged = prefix.to_string_lossy().into_owned().into_bytes();
    let (doc, _) = files_manifest::scan_prefix(&prefix, std::slice::from_ref(&staged)).unwrap();
    (temp, doc)
}

#[test]
fn a_posix_colon_file_name_scans_into_the_oracle_and_parses_back() {
    // pack time — where the incident fired: the scan must record the man page, not refuse it
    let (_temp, doc) = scanned_perl_prefix();
    let entry = doc
        .files
        .iter()
        .find(|e| e.p == "man/man3/App::Cpan.3")
        .expect("the colon-named man page must be recorded");
    assert!(entry.h.is_some(), "a plain file carries a content digest");

    // restore time — the same list read back from the transport must parse and validate
    let parsed = FilesDoc::parse(&doc.to_bytes().unwrap()).unwrap();
    assert!(
        parsed.files.iter().any(|e| e.p == "man/man3/App::Cpan.3"),
        "the man page must survive the round-trip"
    );
}

#[test]
fn only_a_windows_drive_prefix_makes_a_files_manifest_entry_non_relative() {
    let (_temp, doc) = scanned_perl_prefix();
    let text = String::from_utf8(doc.to_bytes().unwrap()).unwrap();

    // (the path as it must appear in the JSON text, why it is refused)
    let refused = [
        ("C:/x", "drive prefix with a slash"),
        ("C:\\\\x", "drive prefix with a backslash"),
        ("C:x", "drive-relative path"),
        ("/x", "absolute path"),
        ("\\\\x", "rooted backslash"),
    ];
    for (bad, because) in refused {
        let tampered = text.replace("man/man3/App::Cpan.3", bad);
        let err = match FilesDoc::parse(tampered.as_bytes()) {
            Ok(_) => panic!("{because}: {bad} must be refused"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("must be relative"),
            "{because}: got {err}"
        );
    }
}

/// Issue #95's diagnosis note: at pack time the rejection fires while *scanning* the
/// verification-unpacked environment, before any manifest exists — the error must name
/// the file on disk, not the document it was going to be written into.
#[test]
#[cfg(unix)]
fn a_scan_time_rejection_names_the_scanned_file_not_the_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("env");
    fs::create_dir_all(&prefix).unwrap();
    // `C:x` is a creatable POSIX file name and a refused transport path: exactly the
    // shape a pack of a misbehaving tree would hit.
    fs::write(prefix.join("C:x"), "a file literally named C:x\n").unwrap();
    let staged = prefix.to_string_lossy().into_owned().into_bytes();
    let err = match files_manifest::scan_prefix(&prefix, std::slice::from_ref(&staged)) {
        Ok(_) => panic!("a drive-named file must be refused"),
        Err(err) => err,
    };
    let msg = err.to_string();
    assert!(msg.contains("scanned environment file"), "got: {msg}");
    assert!(msg.contains("must be relative"), "got: {msg}");
}

// Moved from the module's inline `#[cfg(test)]` (TASK-83): names kept, bodies verbatim.

#[test]
fn nul_runs_collapse_to_a_single_nul() {
    assert_eq!(collapse_nul_runs(b"ab"), b"ab");
    assert_eq!(collapse_nul_runs(b"a\0\0\0\0b"), b"a\0b");
    assert_eq!(collapse_nul_runs(b"\0\0"), b"\0");
    assert_eq!(collapse_nul_runs(b"\0a\0\0b\0"), b"\0a\0b\0");
    // A single NUL is a single NUL: integers in binaries keep their meaning.
    assert_eq!(collapse_nul_runs(b"a\0b"), b"a\0b");
}

#[test]
fn replacements_cover_text_and_padded_binary_fields() {
    let stage = b"/scratch/stage/env".to_vec();
    let final_path = b"/home/user/project/.pixi/envs/env".to_vec();

    // A text file names its prefix once.
    let text_stage = format!("prefix={}\n", String::from_utf8_lossy(&stage));
    let text_final = format!("prefix={}\n", String::from_utf8_lossy(&final_path));
    assert_eq!(
        canonicalise(text_stage.as_bytes(), std::slice::from_ref(&stage)),
        canonicalise(text_final.as_bytes(), std::slice::from_ref(&final_path)),
        "text: both sides must hash the same canonical bytes"
    );

    // A fixed-width binary field: the prefix is NUL-padded to the field width, so the
    // padding length follows the path length. Collapsing NULs makes both sides equal.
    let field_stage = [stage.as_slice(), b"\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0"].concat();
    let field_final = [final_path.as_slice(), b"\0"].concat();
    assert_eq!(
        canonicalise(&field_stage, std::slice::from_ref(&stage)),
        canonicalise(&field_final, std::slice::from_ref(&final_path)),
        "binary: padding must not leak the path length"
    );

    // The sentinel itself is what both canonical forms contain, in place of the path.
    let canonical = canonicalise(text_stage.as_bytes(), std::slice::from_ref(&stage));
    assert_eq!(canonical, [b"prefix=", SENTINEL, b"\n"].concat());
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Pack and restore write different-length prefixes into fixed-width NUL-padded fields.
    /// The scanned files manifest must hash either spelling identically, for arbitrary
    /// surrounding binary bytes and arbitrary padding widths.
    #[test]
    fn file_manifest_canonicalisation_is_stable_across_prefixes(
        suffix in proptest::collection::vec(any::<u8>(), 0..128),
        stage_padding in 1usize..32,
        final_padding in 1usize..32,
    ) {
        let stage = b"/tmp/pixi-sandbox-stage/env".to_vec();
        let final_prefix = b"/home/project/.pixi/envs/env".to_vec();
        let stage_bytes = [
            b"prefix=".as_slice(),
            stage.as_slice(),
            &vec![0; stage_padding],
            suffix.as_slice(),
        ].concat();
        let final_bytes = [
            b"prefix=".as_slice(),
            final_prefix.as_slice(),
            &vec![0; final_padding],
            suffix.as_slice(),
        ].concat();

        prop_assert_eq!(
            canonicalise(&stage_bytes, std::slice::from_ref(&stage)),
            canonicalise(&final_bytes, std::slice::from_ref(&final_prefix)),
        );
    }
}

#[test]
fn multiple_candidates_and_repeated_occurrences_all_neutralise() {
    let stage = b"/s/env".to_vec();
    let pack = b"/s/pack".to_vec();
    let text = b"/s/env/bin:/s/env/lib:/s/pack/cache/x:/other";
    let canonical = replace_all(&replace_all(text, &stage, SENTINEL), &pack, SENTINEL);
    assert_eq!(
        canonical,
        [
            SENTINEL,
            b"/bin:",
            SENTINEL,
            b"/lib:",
            SENTINEL,
            b"/cache/x:/other"
        ]
        .concat()
    );
}

#[test]
fn allowed_extras_are_only_restore_or_pixi_bookkeeping() {
    // The allowlist is the attack surface of the oracle: everything in it escapes content
    // verification, so it must stay exactly the files pixi and restore write themselves.
    for rel in ALLOWED_EXTRAS {
        assert!(
            rel.starts_with("conda-meta/")
                || rel.starts_with("etc/conda/activate.d/pixi-sandbox-cargo-home."),
            "{rel}: the allowlist must not reach outside pixi/restore-owned bookkeeping"
        );
    }
}

#[test]
fn scanning_a_staged_prefix_produces_a_verifiable_list() {
    let temp = tempfile::tempdir().unwrap();
    let prefix = temp.path().join("env");
    fs::create_dir_all(prefix.join("bin")).unwrap();
    fs::create_dir_all(prefix.join("conda-meta")).unwrap();

    let staged = prefix.to_string_lossy().into_owned().into_bytes();
    fs::create_dir_all(prefix.join("lib")).unwrap();
    fs::write(prefix.join("bin/tool"), b"#!/bin/sh\necho hi\n").unwrap();
    fs::write(
        prefix.join("lib/pkgconfig.pc"),
        format!("prefix={}\n", String::from_utf8_lossy(&staged)),
    )
    .unwrap();
    fs::write(
        prefix.join("conda-meta/records.json"),
        format!(
            "{{\"url\": \"file://{}/channel/x.conda\"}}",
            String::from_utf8_lossy(&staged)
        ),
    )
    .unwrap();
    fs::write(prefix.join("conda-meta/history"), "// log\n").unwrap();
    // restore's own markers: excluded, never listed
    fs::write(prefix.join("conda-meta/pixi_env_prefix"), "irrelevant").unwrap();
    fs::write(
        prefix.join("conda-meta/.pixi-environment-fingerprint"),
        "0123456789abcdef",
    )
    .unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(prefix.join("bin/tool"), fs::Permissions::from_mode(0o755)).unwrap();
    }

    let (doc, bytes) = scan_prefix(&prefix, std::slice::from_ref(&staged)).unwrap();
    let by_path: std::collections::BTreeMap<&str, &FileEntry> = doc
        .files
        .iter()
        .map(|entry| (entry.p.as_str(), entry))
        .collect();
    assert_eq!(
        doc.excluded,
        RESTORE_MARKERS
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
    );
    assert_eq!(by_path.len(), 4, "markers excluded: {:?}", by_path.keys());
    assert!(by_path["bin/tool"].x);
    assert!(by_path["bin/tool"].h.is_some());
    // the .pc file's digest is over the sentinel form
    let pc = fs::read(prefix.join("lib/pkgconfig.pc")).unwrap();
    assert_eq!(
        by_path["lib/pkgconfig.pc"].h.as_deref(),
        Some(canonical_sha256(&pc, &[staged]).as_str()),
    );
    // records and history are presence-only
    assert!(by_path["conda-meta/records.json"].h.is_none());
    assert!(by_path["conda-meta/history"].h.is_none());
    assert!(bytes > 0);
    doc.validate().unwrap();
}

#[test]
fn parsing_rejects_a_forged_or_malformed_list() {
    let bad = [
        r#"{"schema": 2, "excluded": [], "files": [{"p": "a"}]}"#, // future schema
        r#"{"schema": 1, "excluded": [], "files": []}"#,           // nothing listed
        r#"{"schema": 1, "excluded": [], "files": [{"p": "../escape"}]}"#,
        r#"{"schema": 1, "excluded": [], "files": [{"p": "a"}, {"p": "a"}]}"#,
        r#"{"schema": 1, "excluded": [], "files": [{"p": "a", "h": "nope"}]}"#,
        r#"{"schema": 1, "excluded": [], "files": [{"p": "a", "h": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "l": "../x"}]}"#,
    ];
    for text in bad {
        assert!(
            FilesDoc::parse(text.as_bytes()).is_err(),
            "must be refused: {text}"
        );
    }
    let good = r#"{"schema": 1, "excluded": ["conda-meta/pixi_env_prefix"], "files": [{"p": "bin/tool", "h": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08", "x": true}]}"#;
    FilesDoc::parse(good.as_bytes()).unwrap();
}

#[test]
fn the_list_path_is_inside_the_environment_directory() {
    assert_eq!(list_rel_path("demo"), "envs/demo/files.json");
    assert!(!Path::new(&list_rel_path("demo")).is_absolute());
}
