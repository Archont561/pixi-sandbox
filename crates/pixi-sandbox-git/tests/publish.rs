//! Publish and fetch, exercised through both implementations of [`GitProtocol`].
//!
//! The mock tests state the contract (`orphan`, force-push, read-only snapshot, byte-exact
//! fetch). The shell tests prove the same things against a real `git` and a local bare remote
//! — and the recording runner asserts the *argv*, which is the only way to see "orphan" (a
//! `commit-tree` without `-p`) and "-−force" as facts rather than as intentions.

use pixi_sandbox_git::{
    FakeGit, GitProtocol, RecordingRunner, ShellGit, Snapshot, snapshot_bytes, snapshot_files,
};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

const REMOTE: &str = "/tmp/does-not-matter.git";
const BRANCH: &str = "sandbox/demo-linux-64";

fn snapshot(files: &[(&str, &[u8])]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (path, content) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().expect("parent")).expect("mkdir");
        std::fs::write(full, content).expect("write");
    }
    dir
}

fn request<'a>(dir: &'a Path, message: &'a str) -> Snapshot<'a> {
    Snapshot {
        dir,
        branch: BRANCH,
        remote: REMOTE,
        message,
    }
}

fn run_git(args: &[&str], cwd: &Path) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn bare_remote() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("remote.git");
    let out = Command::new("git")
        .args(["init", "-q", "--bare"])
        .arg(&path)
        .output()
        .expect("git runs");
    assert!(out.status.success());
    let remote = path.to_string_lossy().into_owned();
    (dir, remote)
}

// ---------------------------------------------------------------- the mock (fast, offline)

#[test]
fn the_mock_publishes_then_fetches_the_same_bytes() {
    let git = FakeGit::new();
    let transport = snapshot(&[
        (".pixi-sandbox/manifest.json", b"{\"schema\":1}"),
        (
            ".pixi-sandbox/envs/dev/pack/channel/noarch/a.conda",
            b"payload",
        ),
        ("README.md", b"# Offline sandbox\n"),
    ]);

    let published = git.publish(&request(transport.path(), "snapshot")).unwrap();
    assert_eq!(published.files, 3);
    assert_eq!(
        published.bytes,
        snapshot_bytes(&snapshot_files(transport.path()).unwrap())
    );
    assert!(git.branch_exists(REMOTE, BRANCH).unwrap());

    let dest = tempfile::tempdir().unwrap();
    let fetched = git.fetch_into(REMOTE, BRANCH, dest.path()).unwrap();
    assert_eq!(fetched.commit, published.commit);
    assert_eq!(fetched.files, 3);

    let files = git.files(REMOTE, BRANCH).unwrap();
    for (path, content) in files {
        assert_eq!(
            std::fs::read(dest.path().join(&path)).unwrap(),
            content,
            "{path} must survive the round trip byte for byte"
        );
    }
    // one publish, one force-push, one fetch — in that order
    assert_eq!(git.pushes(), 1);
    assert!(matches!(
        git.ops()[1],
        pixi_sandbox_git::Op::Push { forced: true, .. }
    ));
}

#[test]
fn the_mock_replaces_history_instead_of_appending_to_it() {
    let git = FakeGit::new();
    let first = snapshot(&[("payload.txt", b"first")]);
    let second = snapshot(&[("payload.txt", b"second")]);

    git.publish(&request(first.path(), "snapshot 1")).unwrap();
    let tip = git.tip(REMOTE, BRANCH).unwrap();
    git.publish(&request(second.path(), "snapshot 2")).unwrap();

    assert_eq!(git.pushes(), 2, "each publish pushes");
    assert_eq!(
        git.history(REMOTE, BRANCH).len(),
        1,
        "an orphan branch force-pushed twice still serves exactly one commit"
    );
    assert_ne!(tip, git.tip(REMOTE, BRANCH).unwrap());
    assert_eq!(
        git.files(REMOTE, BRANCH).unwrap()["payload.txt"],
        b"second".to_vec()
    );
}

#[test]
fn a_rejected_push_keeps_the_remote_unchanged_and_names_it() {
    let git = FakeGit::new();
    let good = snapshot(&[("payload.txt", b"good")]);
    git.publish(&request(good.path(), "snapshot")).unwrap();

    git.fail_next_push("protected branch");
    let bad = snapshot(&[("payload.txt", b"bad")]);
    let error = git.publish(&request(bad.path(), "snapshot")).unwrap_err();
    let message = error.to_string();

    assert!(message.contains(REMOTE), "got: {message}");
    assert!(message.contains(BRANCH), "got: {message}");
    assert!(message.contains("protected branch"), "got: {message}");
    assert_eq!(
        git.files(REMOTE, BRANCH).unwrap()["payload.txt"],
        b"good".to_vec(),
        "a rejected push must not be visible on the remote"
    );
    assert_eq!(git.pushes(), 2, "the attempt is still recorded");
}

#[test]
fn fetching_an_unknown_branch_is_an_error_not_an_empty_directory() {
    let git = FakeGit::new();
    let dest = tempfile::tempdir().unwrap();
    let error = git
        .fetch_into(REMOTE, "sandbox/nope", dest.path())
        .unwrap_err();
    assert!(error.to_string().contains("has no branch"), "got: {error}");
}

#[test]
fn both_implementations_refuse_a_symlink_in_the_transport() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("real.txt"), b"payload").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();

    #[cfg(unix)]
    {
        for error in [
            FakeGit::new()
                .publish(&request(dir.path(), "snapshot"))
                .unwrap_err(),
            ShellGit::preview()
                .publish(&request(dir.path(), "snapshot"))
                .unwrap_err(),
        ] {
            assert!(error.to_string().contains("symlink"), "got: {error}");
        }
    }
}

// --------------------------------------------------- the real thing (a local bare remote)

#[test]
fn the_shell_implementation_publishes_and_fetches_byte_for_byte() {
    let (tmp, remote) = bare_remote();
    let transport = snapshot(&[
        (".pixi-sandbox/manifest.json", b"{\"schema\":1}"),
        (
            ".pixi-sandbox/envs/dev/pack/channel/noarch/a.conda",
            b"payload",
        ),
        ("AGENTS.md", b"# restore me\n"),
    ]);
    let git = ShellGit::new();
    let published = git
        .publish(&Snapshot {
            dir: transport.path(),
            branch: BRANCH,
            remote: &remote,
            message: "snapshot",
        })
        .unwrap();
    assert_eq!(published.commit.len(), 40);
    assert_eq!(published.files, 3);

    let dest = tmp.path().join("branch");
    let fetched = git.fetch_into(&remote, BRANCH, &dest).unwrap();
    assert_eq!(fetched.commit, published.commit);
    assert!(fetched.files >= 3);
    assert_eq!(
        std::fs::read_to_string(dest.join(".pixi-sandbox/manifest.json")).unwrap(),
        "{\"schema\":1}"
    );
    assert!(
        !dest.join(".git").exists(),
        "the airlock gets the branch content, not a checkout"
    );
}

#[test]
fn a_second_publish_replaces_the_branch_history() {
    let (tmp, remote) = bare_remote();
    let git = ShellGit::new();
    for body in [b"first".as_slice(), b"second".as_slice()] {
        let transport = snapshot(&[("payload.txt", body)]);
        git.publish(&Snapshot {
            dir: transport.path(),
            branch: BRANCH,
            remote: &remote,
            message: "snapshot",
        })
        .unwrap();
    }

    let bare = PathBuf::from(&remote);
    let count = run_git(&["rev-list", "--count", BRANCH], &bare);
    assert_eq!(
        count, "1",
        "lineage must be one commit, not an append-only history"
    );
    assert_eq!(
        run_git(&["log", "-1", "--format=%s", BRANCH], &bare),
        "snapshot"
    );

    let dest = tmp.path().join("branch");
    git.fetch_into(&remote, BRANCH, &dest).unwrap();
    assert_eq!(
        std::fs::read_to_string(dest.join("payload.txt")).unwrap(),
        "second"
    );
}

#[test]
fn the_shell_implementation_never_writes_into_the_transport() {
    let (_tmp, _remote) = bare_remote();
    let transport = snapshot(&[("payload.txt", b"payload")]);
    let before = snapshot_files(transport.path()).unwrap();

    let remote = _remote.clone();
    ShellGit::new()
        .publish(&Snapshot {
            dir: transport.path(),
            branch: BRANCH,
            remote: &remote,
            message: "snapshot",
        })
        .unwrap();

    assert_eq!(snapshot_files(transport.path()).unwrap(), before);
    assert!(!transport.path().join(".git").exists());
    let leftovers: Vec<_> = std::fs::read_dir(transport.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        leftovers,
        vec!["payload.txt".to_string()],
        "the scratch git dir must be gone: {leftovers:?}"
    );
}

#[test]
fn the_recording_runner_makes_the_orphan_and_the_force_visible() {
    let (_tmp, remote) = bare_remote();
    let transport = snapshot(&[("payload.txt", b"payload")]);
    let recorded = Arc::new(RecordingRunner::new());
    let git = ShellGit::with_runner(Box::new(recorded.clone()));

    git.publish(&Snapshot {
        dir: transport.path(),
        branch: BRANCH,
        remote: &remote,
        message: "snapshot",
    })
    .unwrap();

    let commands = recorded.rendered();
    let push = commands
        .iter()
        .find(|c| c.contains("push --force"))
        .unwrap_or_else(|| panic!("no push in {commands:#?}"));
    assert!(push.contains(&remote), "got: {push}");
    assert!(
        push.contains(&format!("refs/heads/{BRANCH}")),
        "got: {push}"
    );

    let commit = commands
        .iter()
        .find(|c| c.contains("commit-tree"))
        .unwrap_or_else(|| panic!("no commit-tree in {commands:#?}"));
    assert!(
        !commit.contains(" -p "),
        "a parentless commit is what makes the branch an orphan: {commit}"
    );
    assert!(
        commands
            .iter()
            .any(|c| c.contains("GIT_WORK_TREE=") || c.contains("GIT_INDEX_FILE=")),
        "the payload must be committed through a scratch index: {commands:#?}"
    );
}

#[test]
fn a_preview_run_reports_the_commands_and_touches_nothing() {
    let transport = snapshot(&[("payload.txt", b"payload")]);
    let git = ShellGit::preview();
    let published = git.publish(&request(transport.path(), "snapshot")).unwrap();

    assert!(!published.commands.is_empty());
    assert!(
        published
            .commands
            .iter()
            .any(|c| c.contains("push --force"))
    );
    assert_eq!(
        std::fs::read_dir(transport.path()).unwrap().count(),
        1,
        "a dry run must not create the scratch directory"
    );
}

#[test]
fn the_remote_size_is_knowable_for_a_local_remote_and_not_for_a_url() {
    let (_tmp, remote) = bare_remote();
    let transport = snapshot(&[("payload.txt", b"payload payload payload")]);
    let git = ShellGit::new();
    git.publish(&Snapshot {
        dir: transport.path(),
        branch: BRANCH,
        remote: &remote,
        message: "snapshot",
    })
    .unwrap();

    let Some(size) = git.remote_size(&remote, BRANCH).unwrap() else {
        panic!("a local bare remote has a knowable object-store size");
    };
    assert!(size > 0, "got {size}");
    assert!(
        git.remote_size("https://example.invalid/repo.git", BRANCH)
            .unwrap()
            .is_none(),
        "there is no cheap size query over the git wire protocol"
    );
}
