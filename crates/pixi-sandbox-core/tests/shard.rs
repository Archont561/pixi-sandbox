//! Sharding is the one place where a bug silently corrupts a user's environment,
//! so it gets property tests, not just examples.

use pixi_sandbox_core::manifest::{Blob, Part};
use pixi_sandbox_core::shard::{
    join_parts, materialise, part_path, read_blob, record_file, sha256_bytes, split_file,
    verify_file,
};
use proptest::prelude::*;
use rstest::rstest;
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

/// TASK-80 AC#3: `assemble` and `read_blob` take a part's *final component* off a path the
/// branch supplies, so a part path of `.` used to panic here rather than fail. Both must now
/// return `Error::Invalid` naming the path: a hostile transport gets a diagnostic and an
/// aborted restore, not an aborted process. `materialise` reaches `assemble` through
/// `join_parts`, which is the road `restore` takes.
#[rstest]
#[case(".")]
#[case("./")]
fn a_part_path_with_no_file_name_is_an_error_not_a_panic(#[case] shape: &str) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let parts = vec![Part {
        path: shape.to_string(),
        size: 4,
        sha256: sha256_bytes(b"test"),
    }];

    // assemble, via join_parts: staged in a sibling and renamed only once it verifies, so
    // the refusal must leave no file behind.
    let err = join_parts(
        &root.join("out/big.conda"),
        root,
        &parts,
        &sha256_bytes(b"test"),
        4,
    )
    .expect_err("a part path with no file name must be refused");
    assert!(err.to_string().contains("no file name"), "got: {err}");
    assert!(
        !root.join("out/big.conda").exists(),
        "no half-written output"
    );

    // read_blob: the same loop, on the road `verify_env_restored` takes to the file oracle.
    let blob = Blob {
        path: "envs/dev/files.json".to_string(),
        size: 4,
        sha256: sha256_bytes(b"test"),
        parts,
    };
    let err = read_blob(root, &blob).expect_err("a part path with no file name must be refused");
    assert!(err.to_string().contains("no file name"), "got: {err}");
    assert!(
        err.to_string().contains(shape),
        "the diagnostic must name the path: {err}"
    );
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

/// The failure a real restore produced: `.pixi/tools/<platform>/pixi` is the binary driving
/// the session, and copying the new bytes straight onto it is ETXTBSY ("Text file busy") on
/// Linux — the restore died at `materialise tools`, naming the source path. Staging beside
/// the destination and renaming replaces the directory entry instead, which the kernel allows
/// while the old inode is still executing.
#[test]
#[cfg(unix)]
fn materialise_replaces_a_tool_that_is_currently_executing() {
    use std::os::unix::fs::PermissionsExt;
    use std::process::{Command, Stdio};

    // A real ELF binary is required — a `#!` script is not a busy text file — and `sleep` is
    // the one every Unix runner has. Skipped rather than failed where it is absent.
    let system_sleep = Path::new("/bin/sleep");
    if !system_sleep.is_file() {
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let branch = dir.path().join("branch");
    fs::create_dir_all(&branch).unwrap();
    fs::copy(system_sleep, branch.join("tool")).unwrap();
    let blob = record_file(&branch, "tool", 1 << 30).unwrap();

    let dst = dir.path().join("tools/linux-64/tool");
    fs::create_dir_all(dst.parent().unwrap()).unwrap();
    fs::copy(system_sleep, &dst).unwrap();
    fs::set_permissions(&dst, fs::Permissions::from_mode(0o755)).unwrap();

    let mut running = Command::new(&dst)
        .arg("30")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the staged tool runs");

    let result = materialise(&branch, &blob, &dst);

    let _ = running.kill();
    let _ = running.wait();
    result.expect("a tool being executed must still be replaceable");

    verify_file(&dst, &blob.sha256, blob.size).unwrap();
    // And nothing staged is left behind next to it.
    let leftovers: Vec<_> = fs::read_dir(dst.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temp siblings left: {leftovers:?}");
}

/// Staging introduced a second path that can appear in an error, and the bug the staging fix
/// was about was an error naming the *wrong* one. So the rule is explicit: a blob the branch
/// does not have is a fact about the branch and must name the branch path, never the hidden
/// sibling the restore was about to write.
#[test]
fn materialise_names_the_branch_path_when_the_blob_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("branch");
    write(&src.join("pack/channel/noarch/pkg.conda"), b"payload");
    let blob = record_file(&src, "pack/channel/noarch/pkg.conda", 1024).unwrap();
    fs::remove_file(src.join("pack/channel/noarch/pkg.conda")).unwrap();

    let dst = dir.path().join("out/pkg.conda");
    let err = materialise(&src, &blob, &dst).expect_err("a missing blob cannot be materialised");
    let message = err.to_string();
    assert!(
        message.contains("branch/pack/channel/noarch/pkg.conda"),
        "the error must name the source in the branch: {message}"
    );
    assert!(
        !message.contains(".pkg.conda.join"),
        "the staging sibling is an implementation detail, not the cause: {message}"
    );
}

/// All-or-nothing (D7), now that there is a staging step to get it wrong: a blob whose bytes
/// do not match the manifest must leave the file already on disk exactly as it was — a
/// half-replaced tool is worse than an un-replaced one — and must not leave the staged copy
/// lying next to it.
#[test]
fn a_blob_that_does_not_verify_leaves_the_destination_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("branch");
    write(&src.join("tool"), b"the packed bytes");
    let blob = record_file(&src, "tool", 1 << 20).unwrap();
    // Same length, different content: the size check passes and the digest does not, which is
    // the shape a truncated fetch or a tampered branch actually has.
    write(&src.join("tool"), b"the TAMPERED one");

    let dst = dir.path().join("tools/linux-64/tool");
    write(&dst, b"the copy already restored here");

    let err = materialise(&src, &blob, &dst).expect_err("a tampered blob must not be installed");
    assert!(
        err.to_string().contains("sha256") || err.to_string().contains("does not match"),
        "the error must be about integrity: {err}"
    );
    assert_eq!(
        fs::read(&dst).unwrap(),
        b"the copy already restored here",
        "the destination was replaced by a blob that failed verification"
    );
    let leftovers: Vec<_> = fs::read_dir(dst.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(
        leftovers.is_empty(),
        "staged copy left behind: {leftovers:?}"
    );
}

/// The other half of that rule: everything which is not a missing blob happened on the side
/// being written, so the error must name the staging path — a "permission denied" pointing at
/// the read-only *branch* would send an operator to fix the wrong directory.
#[test]
#[cfg(unix)]
fn materialise_names_the_staging_path_when_the_destination_cannot_be_written() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("branch");
    write(&src.join("tool"), b"the packed bytes");
    let blob = record_file(&src, "tool", 1 << 20).unwrap();

    let dst_dir = dir.path().join("tools/linux-64");
    fs::create_dir_all(&dst_dir).unwrap();
    fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o555)).unwrap();
    // root ignores the mode bits, and a suite that silently asserts nothing is worse than one
    // that is openly skipped here.
    if fs::write(dst_dir.join(".probe"), b"x").is_ok() {
        let _ = fs::remove_file(dst_dir.join(".probe"));
        fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o755)).unwrap();
        return;
    }

    let err = materialise(&src, &blob, &dst_dir.join("tool"))
        .expect_err("an unwritable destination cannot be materialised");
    let message = err.to_string();
    fs::set_permissions(&dst_dir, fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        message.contains(".tool.join"),
        "the error must name the staging path it failed to write: {message}"
    );
    assert!(
        !message.contains("branch/tool"),
        "the branch is readable; blaming it sends the fix to the wrong directory: {message}"
    );
}
