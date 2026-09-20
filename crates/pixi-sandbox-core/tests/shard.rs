//! Sharding is the one place where a bug silently corrupts a user's environment,
//! so it gets property tests, not just examples.

use pixi_sandbox_core::manifest::Blob;
use pixi_sandbox_core::shard::{
    join_parts, materialise, part_path, record_file, sha256_bytes, split_file, verify_file,
};
use proptest::prelude::*;
use std::fs;
use std::path::Path;

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, bytes).unwrap();
}

proptest! {
    /// Any content, any shard limit: split → join is the identity.
    #[test]
    fn split_then_join_is_identity(
        content in proptest::collection::vec(any::<u8>(), 1..40_000),
        limit in 1usize..8_000,
    ) {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let rel = "pack/channel/noarch/thing-1.0-0.conda";
        write(&root.join(rel), &content);

        let blob: Blob = record_file(root, rel, limit as u64).unwrap();
        prop_assume!(!blob.parts.is_empty() || content.len() as u64 <= limit as u64);

        if blob.parts.is_empty() {
            // under the limit: file stays as-is
            prop_assert!(root.join(rel).exists());
            return Ok(());
        }

        prop_assert!(!root.join(rel).exists(), "source must be replaced by its parts");
        prop_assert_eq!(blob.sha256.clone(), sha256_bytes(&content));
        for (i, part) in blob.parts.iter().enumerate() {
            prop_assert_eq!(&part.path, &part_path(rel, i));
            prop_assert!(part.size <= limit as u64);
        }

        let out = root.join("out/thing.conda");
        let parts_root = root.join(Path::new(rel).parent().unwrap());
        join_parts(&out, &parts_root, &blob.parts, &blob.sha256, blob.size).unwrap();
        prop_assert_eq!(fs::read(&out).unwrap(), content);
    }
}

#[test]
fn small_file_is_not_split() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("a.bin"), b"hello");
    let blob = record_file(root, "a.bin", 1024).unwrap();
    assert!(blob.parts.is_empty());
    assert_eq!(blob.size, 5);
    assert!(root.join("a.bin").exists());
}

#[test]
fn part_boundary_is_exact() {
    // exactly at the limit must not split; one byte over must split into two parts
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("exact.bin"), &[7u8; 100]);
    let at = record_file(root, "exact.bin", 100).unwrap();
    assert!(at.parts.is_empty());

    write(&root.join("over.bin"), &[7u8; 101]);
    let over = record_file(root, "over.bin", 100).unwrap();
    assert_eq!(over.parts.len(), 2);
    assert_eq!(over.parts[0].size, 100);
    assert_eq!(over.parts[1].size, 1);
}

#[test]
fn join_refuses_a_corrupted_part() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("big.bin"), &vec![1u8; 300]);
    let blob = record_file(root, "big.bin", 100).unwrap();
    assert_eq!(blob.parts.len(), 3);

    // tamper with the second part, in place (this is what a hostile branch looks like)
    let victim = root.join(&blob.parts[1].path);
    let mut bytes = fs::read(&victim).unwrap();
    bytes[0] ^= 0xff;
    fs::write(&victim, &bytes).unwrap();

    let err = join_parts(
        &root.join("out.bin"),
        root,
        &blob.parts,
        &blob.sha256,
        blob.size,
    )
    .expect_err("corrupted part must be rejected");
    assert!(err.to_string().contains("integrity"), "got: {err}");
    assert!(!root.join("out.bin").exists(), "no half-written output");
}

#[test]
fn join_refuses_a_missing_part() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("big.bin"), &vec![1u8; 300]);
    let blob = record_file(root, "big.bin", 100).unwrap();
    fs::remove_file(root.join(&blob.parts[2].path)).unwrap();
    let err = join_parts(
        &root.join("out.bin"),
        root,
        &blob.parts,
        &blob.sha256,
        blob.size,
    )
    .expect_err("missing part must be rejected");
    assert!(err.to_string().contains("missing split part"), "got: {err}");
}

#[test]
fn materialise_copies_unsplit_blobs_and_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("branch");
    write(&src.join("pack/channel/noarch/pkg.conda"), b"payload");
    let blob = record_file(&src, "pack/channel/noarch/pkg.conda", 1024).unwrap();

    let dst = dir.path().join("out/pkg.conda");
    materialise(&src, &blob, &dst).unwrap();
    assert_eq!(fs::read(&dst).unwrap(), b"payload");
    verify_file(&dst, &blob.sha256, blob.size).unwrap();
}

#[test]
fn split_file_rejects_a_pointless_split() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("small.bin");
    fs::write(&path, b"tiny").unwrap();
    let err = split_file(&path, "small.bin", 1 << 20).expect_err("should refuse");
    assert!(err.to_string().contains("pointless"), "got: {err}");
}
