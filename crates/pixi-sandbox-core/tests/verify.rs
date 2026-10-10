//! Verification must catch a corrupted branch *before* anything is written, and must catch
//! the one mistake that only shows up on the airlock: a dynamically linked tool.

use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify::{
    Kind, Linkage, linkage_of, linkage_of_bytes, manifest_path, verify,
};
use rstest::{fixture, rstest};
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

fn sha256_of(bytes: &[u8]) -> String {
    pixi_sandbox_core::shard::sha256_bytes(bytes)
}

/// A transport dir whose single blob matches the given digest. Defaults to the digest that
/// actually matches `BLOB_BODY`, so the common case (a transport that verifies cleanly) needs
/// no override; tests that want a deliberately wrong manifest digest pass `#[with(...)]`.
#[fixture]
fn transport(#[default(sha256_of(BLOB_BODY))] sha: String) -> (tempfile::TempDir, Manifest) {
    let dir = tempfile::tempdir().unwrap();
    let pack = dir
        .path()
        .join(".pixi-sandbox/envs/dev/pack/channel/noarch");
    fs::create_dir_all(&pack).unwrap();
    fs::write(pack.join("a.conda"), BLOB_BODY).unwrap();
    fs::write(pack.join("big.conda.part000"), SPLIT_PARTS[0]).unwrap();
    fs::write(pack.join("big.conda.part001"), SPLIT_PARTS[1]).unwrap();
    let manifest: Manifest = serde_json::from_str(&manifest_json(&sha, BLOB_BODY.len())).unwrap();
    fs::write(
        manifest_path(dir.path()),
        serde_json::to_string_pretty(&manifest).unwrap(),
    )
    .unwrap();
    (dir, manifest)
}

#[test]
fn manifest_path_points_into_the_pixi_sandbox_directory() {
    let path = manifest_path(Path::new("/some/branch"));
    assert!(
        path.ends_with(".pixi-sandbox/manifest.json"),
        "got {path:?}"
    );
}

#[rstest]
fn a_good_transport_verifies(transport: (tempfile::TempDir, Manifest)) {
    let (dir, manifest) = transport;
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

/// A corruption applied to the transport's `a.conda` blob before verification, paired with the
/// `Kind` of failure it must produce. Tampering and deleting are different mutations but the
/// same shape of test: corrupt one known blob, verify, and check exactly one failure of the
/// expected kind is reported against that blob's path — a textbook `#[case]` table.
#[rstest]
#[case::tampered(
    |path: &Path| {
        // same length on purpose: the size check must not mask the digest check
        fs::write(path, b"conda-PAYLOAD").unwrap();
    },
    Kind::Integrity
)]
#[case::missing(|path: &Path| fs::remove_file(path).unwrap(), Kind::Missing)]
fn a_corrupted_blob_is_reported_with_the_matching_failure_kind(
    transport: (tempfile::TempDir, Manifest),
    #[case] corrupt: fn(&Path),
    #[case] expected: Kind,
) {
    let (dir, manifest) = transport;
    let blob = dir
        .path()
        .join(".pixi-sandbox/envs/dev/pack/channel/noarch/a.conda");
    corrupt(&blob);

    let report = verify(&manifest, dir.path(), None);
    assert!(!report.ok());
    assert_eq!(report.failures[0].kind, expected);
    assert!(report.failures[0].path.ends_with("a.conda"));
}

// ---------------------------------------------------------------- property: any corruption is caught
//
// The example-based tests above pin the exact failure shape (which `Kind`, which path) for one
// hand-picked tamper. This property instead ranges over every blob the fixture carries (the
// whole file and both halves of the split one) and every way a single byte inside it can
// change, and checks the one invariant verify() exists to guarantee: a blob that still matches
// its recorded digest always passes, and one that does not always fails. Bounded to 64 cases —
// enough to range over all three files and the flip/no-flip split without adding meaningful
// runtime to the suite.
mod verify_catches_corruption {
    use super::{BLOB_BODY, SPLIT_PARTS, sha256_of, transport};
    use pixi_sandbox_core::verify::verify;
    use proptest::prelude::*;
    use std::fs;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        /// `file_index` picks which of the transport's three on-disk blobs to touch,
        /// `offset_seed` is reduced into that file's length so it always lands in bounds, and
        /// `flip_mask` is never zero so XOR-ing it in always changes the byte. `flip` is the
        /// property's other half: when false, nothing is touched and verification must still
        /// pass — the proof that this property cannot pass by always tampering.
        #[test]
        fn single_byte_flips_are_always_caught_and_untouched_blobs_never_are(
            file_index in 0usize..3,
            offset_seed in any::<u8>(),
            flip_mask in 1u8..=255u8,
            flip in any::<bool>(),
        ) {
            let (dir, manifest) = transport(sha256_of(BLOB_BODY));
            let root = dir.path().join(".pixi-sandbox/envs/dev/pack/channel/noarch");
            let paths = [
                root.join("a.conda"),
                root.join("big.conda.part000"),
                root.join("big.conda.part001"),
            ];
            // Sanity: the fixture's own sizes, so a future edit to `transport()` cannot shrink
            // a file to zero bytes without this property noticing via a division panic.
            prop_assert_eq!(fs::metadata(&paths[0]).unwrap().len(), BLOB_BODY.len() as u64);
            prop_assert_eq!(fs::metadata(&paths[1]).unwrap().len(), SPLIT_PARTS[0].len() as u64);
            prop_assert_eq!(fs::metadata(&paths[2]).unwrap().len(), SPLIT_PARTS[1].len() as u64);

            if flip {
                let path = &paths[file_index];
                let mut bytes = fs::read(path).unwrap();
                let offset = (offset_seed as usize) % bytes.len();
                bytes[offset] ^= flip_mask;
                fs::write(path, &bytes).unwrap();
            }

            let report = verify(&manifest, dir.path(), None);
            prop_assert_eq!(
                report.ok(),
                !flip,
                "flip={} failures={:?}",
                flip,
                report.failures
            );
        }
    }
}

#[test]
fn a_script_is_not_a_dynamic_binary() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("pixi-sandbox");
    fs::write(&script, b"#!/bin/sh\nexec pixi-sandbox \"$@\"\n").unwrap();
    assert_eq!(linkage_of(&script), Linkage::Script); // a text launcher, not a linked binary
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

// ---------------------------------------------------------------- the restored-tree oracle (D13)
//
// These tests build the whole pipeline in miniature: a staged prefix (what pixi-unpack
// produced), the per-file oracle scanned from it, and a "restored" prefix (what restore
// leaves behind: the staging path rewritten to the final prefix, plus restore's markers).
// Then they attack the result the way task-10 measured a real transport being attacked.

/// Build a staged prefix and the transport carrying its oracle. Returns
/// (transport, project, staged, final) where the project already looks restored.
#[cfg(unix)]
mod restored {
    use pixi_sandbox_core::files_manifest;
    use pixi_sandbox_core::manifest::Manifest;
    use pixi_sandbox_core::shard;
    use pixi_sandbox_core::verify::{Kind, verify, verify_restored};
    use std::fs;
    use std::path::{Path, PathBuf};

    pub struct World {
        pub transport: PathBuf,
        pub project: PathBuf,
        pub staged: PathBuf,
        pub final_prefix: PathBuf,
        pub manifest: Manifest,
    }

    pub fn world(dir: &Path) -> World {
        use std::os::unix::fs::PermissionsExt;

        // The staged prefix sits exactly where restore's default work dir puts it: verify
        // derives its candidate paths from project + env name + work dir alone (never from
        // files restore wrote — that would be a candidate-injection hole), so the world must
        // match that layout for the faithful-restore case to be faithful.
        let project = dir.join("project");
        let staged = project.join(".pixi/.restore-work/stage-dev/dev");
        let final_prefix = project.join(".pixi/envs/dev");
        let transport = dir.join("transport");

        // --- a staged prefix: text that names the staging prefix, a NUL-padded binary field
        // that does the same in fixed width, a conda-meta record, history, a symlink.
        let staged_text = staged.to_string_lossy().into_owned();
        fs::create_dir_all(staged.join("bin")).unwrap();
        fs::create_dir_all(staged.join("lib")).unwrap();
        fs::create_dir_all(staged.join("conda-meta")).unwrap();
        fs::write(
            staged.join("bin/tool"),
            format!("#!/bin/sh\nPREFIX={staged_text}\n"),
        )
        .unwrap();
        fs::write(
            staged.join("lib/thing.pc"),
            format!("prefix={staged_text}\nexec_prefix=${{prefix}}\n"),
        )
        .unwrap();
        let field = [staged_text.as_bytes(), b"\0\0\0\0\0\0\0\0"].concat();
        fs::write(staged.join("lib/binary.ld"), field).unwrap();
        fs::write(
            staged.join("conda-meta/pkg-1.0-0.json"),
            format!("{{\"url\": \"file://{staged_text}/channel/pkg.conda\"}}"),
        )
        .unwrap();
        fs::write(
            staged.join("conda-meta/history"),
            "// not relevant for pixi\n",
        )
        .unwrap();
        // issue #95: perl's module man pages carry `Package::Name.3` names — a legal POSIX
        // byte the oracle must record, not reject (an env resolving perl could never pack).
        fs::create_dir_all(staged.join("man/man3")).unwrap();
        fs::write(staged.join("man/man3/App::Cpan.3"), "doc stub\n").unwrap();
        std::os::unix::fs::symlink("thing.pc", staged.join("lib/link.pc")).unwrap();
        fs::set_permissions(staged.join("bin/tool"), fs::Permissions::from_mode(0o755)).unwrap();

        // --- the oracle, scanned the way pack scans it (candidates = this side's paths)
        let candidates = vec![staged_text.clone().into_bytes()];
        let (doc, _) = files_manifest::scan_prefix(&staged, &candidates).unwrap();

        // --- the transport: the oracle plus one honest env blob, wired into a manifest
        let payload = transport.join(".pixi-sandbox");
        fs::create_dir_all(payload.join("envs/dev/pack/channel/noarch")).unwrap();
        fs::write(
            payload.join("envs/dev/pack/channel/noarch/a.conda"),
            b"conda-payload",
        )
        .unwrap();
        let list_rel = files_manifest::list_rel_path("dev");
        let list_bytes = doc.to_bytes().unwrap();
        fs::write(payload.join(&list_rel), &list_bytes).unwrap();
        let manifest: Manifest = serde_json::from_value(serde_json::json!({
            "schema": 2,
            "tool": { "name": "pixi-sandbox", "version": "0.1.0" },
            "created_at": "2026-09-29T12:00:00Z",
            "platform": "linux-64",
            "shard_limit_bytes": 99_614_720,
            "source": {},
            "tools": {},
            "envs": { "dev": {
                "platform": "linux-64",
                "pack_path": ".pixi-sandbox/envs/dev/pack",
                "packed_size_bytes": 13,
                "unpacked_size_bytes": 4096,
                "pixi_environment_fingerprint": "0123456789abcdef",
                "blobs": [
                    { "path": "envs/dev/pack/channel/noarch/a.conda", "size": 13,
                      "sha256": shard::sha256_bytes(b"conda-payload") }
                ],
                "files": {
                    "blob": { "path": list_rel, "size": list_bytes.len(),
                              "sha256": shard::sha256_bytes(&list_bytes) },
                    "entries": doc.entries()
                }
            }}
        }))
        .unwrap();
        manifest.validate().unwrap();

        // --- the project, the way restore leaves it: the staged prefix moved and its
        // staging path rewritten to the final prefix, plus restore's own markers.
        restore_like(&staged, &final_prefix);

        World {
            transport,
            project,
            staged,
            final_prefix,
            manifest,
        }
    }

    /// The relocation rule restore applies (text only, NUL-preserved binaries untouched),
    /// then `write_markers`' two files.
    pub fn restore_like(staged: &Path, final_prefix: &Path) {
        let old = staged
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        let new = final_prefix.to_string_lossy().into_owned();
        for file in shard::files_under(staged).unwrap() {
            let rel = file.strip_prefix(staged).unwrap();
            let destination = final_prefix.join(rel);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(&file, &destination).unwrap();
            let bytes = fs::read(&destination).unwrap();
            if !bytes.contains(&0) {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    if text.contains(&old) {
                        fs::write(&destination, text.replace(&old, &new)).unwrap();
                    }
                }
            }
        }
        // symlinks (files_under skips them)
        for entry in fs::read_dir(staged.join("lib")).unwrap().flatten() {
            let path = entry.path();
            if path.is_symlink() {
                let target = fs::read_link(&path).unwrap();
                std::os::unix::fs::symlink(
                    &target,
                    final_prefix.join("lib").join(entry.file_name()),
                )
                .unwrap();
            }
        }
        fs::create_dir_all(final_prefix.join("conda-meta")).unwrap();
        fs::write(
            final_prefix.join("conda-meta/pixi_env_prefix"),
            format!(
                "{}/conda-meta",
                final_prefix.canonicalize().unwrap().display()
            ),
        )
        .unwrap();
        fs::write(
            final_prefix.join("conda-meta/.pixi-environment-fingerprint"),
            "0123456789abcdef",
        )
        .unwrap();
    }

    fn kinds(report: &pixi_sandbox_core::verify::RestoredReport) -> Vec<&'static str> {
        report
            .report
            .failures
            .iter()
            .map(|f| f.kind.as_str())
            .collect()
    }

    #[test]
    fn a_faithful_restore_matches_the_oracle() {
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(
            report.ok(),
            "unexpected failures: {:?}",
            report.report.failures
        );
        assert_eq!(report.verified, ["dev"]);
        assert!(report.unverifiable.is_empty());
        // the transport itself verifies too, oracle included
        let transport_report = verify(&world.manifest, &world.transport, None);
        assert!(transport_report.ok(), "{:?}", transport_report.failures);
    }

    #[test]
    fn a_forged_conda_meta_record_is_an_unexpected_file() {
        // The exact attack task-10 measured: a stub prefix carrying one hand-forged record
        // passed every shape check. Here the record is present, but it is not in the list.
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::write(
            world.final_prefix.join("conda-meta/forged-9.9.9-0.json"),
            r#"{"name":"forged","version":"9.9.9","build":"0","files":[]}"#,
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        let failures = &report.report.failures;
        assert_eq!(
            failures.len(),
            1,
            "one forged record, one failure: {failures:?}"
        );
        assert_eq!(failures[0].kind, Kind::Unexpected);
        assert!(failures[0].path.contains("forged"));
    }

    #[test]
    fn every_mismatch_is_collected_not_just_the_first() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::write(
            world.final_prefix.join("lib/thing.pc"),
            "prefix=/somewhere/else\n",
        )
        .unwrap();
        fs::remove_file(world.final_prefix.join("bin/tool")).unwrap();
        fs::write(world.final_prefix.join("lib/extra.txt"), "smuggled\n").unwrap();
        fs::set_permissions(
            world.final_prefix.join("lib/binary.ld"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        let got = kinds(&report);
        assert!(got.contains(&"integrity"), "tampered content: {got:?}");
        assert!(got.contains(&"missing"), "removed file: {got:?}");
        assert!(got.contains(&"unexpected"), "smuggled file: {got:?}");
        // a gained exec bit deviates from the recorded mode just as a lost one does — the
        // oracle records the mode, not a direction
        assert!(
            got.contains(&"mode"),
            "binary.ld gaining the exec bit: {got:?}"
        );
        assert_eq!(got.len(), 4, "exactly the four tamper classes: {got:?}");
    }

    #[test]
    fn a_lost_executable_bit_is_a_mode_failure() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::set_permissions(
            world.final_prefix.join("bin/tool"),
            fs::Permissions::from_mode(0o644),
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        assert_eq!(report.report.failures[0].kind, Kind::Mode);
    }

    #[test]
    fn a_retargeted_symlink_is_an_integrity_failure() {
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::remove_file(world.final_prefix.join("lib/link.pc")).unwrap();
        std::os::unix::fs::symlink("binary.ld", world.final_prefix.join("lib/link.pc")).unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        assert!(
            report
                .report
                .failures
                .iter()
                .any(|f| f.kind == Kind::Integrity && f.path.contains("link.pc"))
        );
    }

    #[test]
    fn a_failed_relocation_is_caught_even_though_the_bytes_arrived() {
        // The blob verified on arrival, but suppose the staged path never got rewritten: the
        // file on disk names the restore scratch, which no honest final prefix contains.
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::write(
            world.final_prefix.join("lib/thing.pc"),
            format!("prefix={}\n", world.staged.to_string_lossy()),
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(
            !report.ok(),
            "a prefix still pointing into restore scratch must not pass"
        );
        assert_eq!(report.report.failures[0].kind, Kind::Integrity);
    }

    #[test]
    fn a_tampered_fingerprint_marker_is_an_integrity_failure() {
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        fs::write(
            world
                .final_prefix
                .join("conda-meta/.pixi-environment-fingerprint"),
            "ffffffffffffffff",
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        assert!(
            report
                .report
                .failures
                .iter()
                .any(|f| f.kind == Kind::Integrity && f.detail.contains("fingerprint"))
        );
    }

    #[test]
    fn a_schema_one_env_is_reported_unverifiable_not_failed() {
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        let mut manifest = world.manifest.clone();
        manifest.envs.get_mut("dev").unwrap().files = None;

        let report = verify_restored(&manifest, &world.transport, &world.project, None, None);
        assert!(
            report.ok(),
            "an old branch cannot be retrofitted, only reported"
        );
        assert!(report.verified.is_empty());
        assert_eq!(report.unverifiable, ["dev"]);
    }

    #[test]
    fn a_corrupted_oracle_is_itself_a_failure() {
        // The oracle is a verified blob: tamper with it in the transport and the check must
        // refuse to run on it rather than compare against forged digests.
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        let list = world
            .transport
            .join(".pixi-sandbox")
            .join(files_manifest::list_rel_path("dev"));
        fs::write(
            &list,
            b"{\"schema\":1,\"excluded\":[],\"files\":[{\"p\":\"bin/tool\"}]}",
        )
        .unwrap();

        let report = verify_restored(
            &world.manifest,
            &world.transport,
            &world.project,
            None,
            None,
        );
        assert!(!report.ok());
        assert!(
            report
                .report
                .failures
                .iter()
                .any(|f| f.kind == Kind::Integrity && f.detail.contains("file list"))
        );
    }

    #[test]
    fn the_oracle_covers_only_the_selected_envs() {
        let dir = tempfile::tempdir().unwrap();
        let world = world(dir.path());
        let selected = vec!["other".to_string()];
        // `other` does not exist in the manifest, so selection is by name: dev excluded.
        let manifest = world.manifest.clone();
        let report = verify_restored(
            &manifest,
            &world.transport,
            &world.project,
            Some(&selected),
            None,
        );
        // An unknown env is not this function's error to raise (doctor's select_envs does
        // that); here it simply selects nothing, so nothing is checked and nothing fails.
        assert!(report.ok());
        assert!(report.verified.is_empty());
        assert!(report.unverifiable.is_empty());
    }
}

// Moved from the module's inline `#[cfg(test)]` (TASK-83): name kept, body verbatim.

const PHDR: usize = 64;
const PHDR_SIZE: usize = 56;

fn elf(program_type: u32) -> Vec<u8> {
    let mut bytes = vec![0u8; PHDR + PHDR_SIZE];
    bytes[0..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2; // ELFCLASS64
    bytes[5] = 1; // ELFDATA2LSB
    bytes[6] = 1; // EV_CURRENT
    bytes[32..40].copy_from_slice(&(PHDR as u64).to_le_bytes()); // e_phoff
    bytes[52..54].copy_from_slice(&(PHDR as u16).to_le_bytes());
    bytes[54..56].copy_from_slice(&(PHDR_SIZE as u16).to_le_bytes()); // e_phentsize
    bytes[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
    bytes[PHDR..PHDR + 4].copy_from_slice(&program_type.to_le_bytes());
    bytes
}

/// `linkage_of` guards a transport at restore time and `linkage_of_bytes` guards a pin at
/// update time. They are called from different processes, on different files, and must not
/// disagree — so assert they agree rather than that each is individually plausible.
#[test]
fn the_file_and_bytes_entry_points_agree() {
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!("linkage-agree-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut cases: Vec<(&str, Vec<u8>, Linkage)> = vec![
        ("static-elf", elf(1), Linkage::Static),
        ("dynamic-elf", elf(3), Linkage::Dynamic),
        (
            "script",
            b"#!/bin/sh\nexec pixi\n".to_vec(),
            Linkage::Script,
        ),
        (
            "macho",
            [0xcf, 0xfa, 0xed, 0xfe]
                .iter()
                .chain(&[0u8; 28])
                .copied()
                .collect(),
            Linkage::System,
        ),
        (
            "pe",
            b"MZ\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0".to_vec(),
            Linkage::System,
        ),
        ("too-short", b"\x7fELF".to_vec(), Linkage::Unknown),
        ("not-an-executable", vec![0u8; 128], Linkage::Unknown),
    ];

    for (name, bytes, expected) in cases.drain(..) {
        let path = dir.join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(&bytes).unwrap();
        drop(file);

        assert_eq!(linkage_of(&path), expected, "{name} from a file");
        assert_eq!(linkage_of_bytes(&bytes), expected, "{name} from bytes");
    }

    assert_eq!(linkage_of(&dir.join("absent")), Linkage::Unknown);
    std::fs::remove_dir_all(&dir).unwrap();
}
