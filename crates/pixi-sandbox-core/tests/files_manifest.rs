//! `files.json` path rules, frozen from issue #95.
//!
//! A colon is a legal byte in a POSIX file name — perl's module man pages are
//! `man/man3/App::Cpan.3` — so a colon alone must not make a path non-relative. Only a
//! Windows drive prefix does (`C:/x`, `C:\x`, and the drive-relative `C:x`), because that
//! is what the guard was ever meant to catch. The incident: an environment that resolves
//! perl (the conda gtk/webkit stack) could never pack, because building the per-file
//! oracle scanned the verification-unpacked environment and rejected the man pages.

use pixi_sandbox_core::files_manifest::{self, FilesDoc};
use std::fs;

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
