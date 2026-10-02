//! Publish and fetch, exercised through both implementations of [`GitProtocol`].
//!
//! The mock tests state the contract (`orphan`, force-push, read-only snapshot, byte-exact
//! fetch). The shell tests prove the same things against a real `git` and a local bare remote
//! — and the recording runner asserts the *argv*, which is the only way to see "orphan" (a
//! `commit-tree` without `-p`) and "-−force" as facts rather than as intentions.

use pixi_sandbox_git::{
    Command, Error, FakeGit, GitProtocol, Output, RecordingRunner, Runner, ShellGit, Snapshot,
    snapshot_bytes, snapshot_files,
};
use rstest::rstest;
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
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
        keep: 0,
    }
}

/// A request aimed at a real remote (the mock ignores where `REMOTE` points).
fn request_to<'a>(dir: &'a Path, message: &'a str, remote: &'a str) -> Snapshot<'a> {
    Snapshot {
        remote,
        ..request(dir, message)
    }
}

/// The same request with rotation asked for: keep at most `keep` snapshots on the branch.
fn rotating<'a>(dir: &'a Path, message: &'a str, keep: u32) -> Snapshot<'a> {
    Snapshot {
        keep,
        ..request(dir, message)
    }
}

fn run_git(args: &[&str], cwd: &Path) -> String {
    let out = StdCommand::new("git")
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
    let out = StdCommand::new("git")
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

/// Task-3 / design §2: a force-push does not reclaim server space, so retention cannot mean
/// "append and let gc sort it out" — rotation *rebuilds* the branch so it carries at most N
/// snapshots, newest first, and the ones that fall off are no longer referenced.
#[test]
fn the_mock_rotates_the_branch_to_at_most_keep_snapshots() {
    let git = FakeGit::new();
    let mut tips = Vec::new();
    for round in 1..=4 {
        let body = format!("snapshot {round}");
        let dir = snapshot(&[("payload.txt", body.as_bytes())]);
        git.publish(&rotating(dir.path(), &body, 3)).unwrap();
        tips.push(git.tip(REMOTE, BRANCH).unwrap());
    }

    let history = git.history(REMOTE, BRANCH);
    assert_eq!(
        history.len(),
        3,
        "keep 3 must cap the branch at 3 snapshots"
    );
    assert_eq!(history[0], tips[3], "the newest snapshot is the tip");
    assert_eq!(
        history,
        vec![tips[3].clone(), tips[2].clone(), tips[1].clone()],
        "history is the N most recent snapshots, newest first"
    );
    assert!(
        !history.contains(&tips[0]),
        "the oldest snapshot must fall off the rebuilt history"
    );
    assert_eq!(
        git.files(REMOTE, BRANCH).unwrap()["payload.txt"],
        b"snapshot 4".to_vec(),
        "rotation must not change which payload the branch serves"
    );
    assert_eq!(git.pushes(), 4, "still exactly one push per publish");
}

/// Rotation is opt-in: the default request is the orphan snapshot it has always been.
#[rstest]
#[case(0)]
#[case(1)]
fn the_mock_treats_keep_one_and_no_keep_the_same_way(#[case] keep: u32) {
    let git = FakeGit::new();
    for round in 1..=3 {
        let body = format!("snapshot {round}");
        let dir = snapshot(&[("payload.txt", body.as_bytes())]);
        git.publish(&rotating(dir.path(), &body, keep)).unwrap();
    }
    assert_eq!(
        git.history(REMOTE, BRANCH).len(),
        1,
        "keep={keep} must leave a single-snapshot orphan branch"
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

#[rstest]
#[case("fake")]
#[case("shell")]
fn both_implementations_refuse_a_symlink_in_the_transport(#[case] implementation: &str) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("real.txt"), b"payload").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("real.txt", dir.path().join("link.txt")).unwrap();

    #[cfg(unix)]
    {
        let error = match implementation {
            "fake" => FakeGit::new()
                .publish(&request(dir.path(), "snapshot"))
                .unwrap_err(),
            "shell" => ShellGit::preview()
                .publish(&request(dir.path(), "snapshot"))
                .unwrap_err(),
            unexpected => panic!("unknown implementation {unexpected}"),
        };
        assert!(error.to_string().contains("symlink"), "got: {error}");
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
            keep: 0,
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
    let first = snapshot(&[("payload.txt", b"first")]);
    git.publish(&Snapshot {
        dir: first.path(),
        branch: BRANCH,
        remote: &remote,
        message: "snapshot",
        keep: 0,
    })
    .unwrap();
    let second = snapshot(&[("payload.txt", b"second")]);
    git.publish(&Snapshot {
        dir: second.path(),
        branch: BRANCH,
        remote: &remote,
        message: "snapshot",
        keep: 0,
    })
    .unwrap();

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

/// Task-3 against a real remote: four publishes with `keep 2` must leave a branch that serves
/// two snapshots — the newest on top, the one before it as its parent — and each kept commit
/// must still serve its own payload, or "rotation" would just be truncation of the content.
#[test]
fn the_shell_implementation_rebuilds_a_bounded_history_on_the_remote() {
    let (_tmp, remote) = bare_remote();
    let git = ShellGit::new();

    for round in 1..=4 {
        let body = format!("snapshot {round}");
        let transport = snapshot(&[("payload.txt", body.as_bytes())]);
        git.publish(&Snapshot {
            keep: 2,
            ..request_to(transport.path(), &body, &remote)
        })
        .unwrap();
    }

    let bare = PathBuf::from(&remote);
    assert_eq!(
        run_git(&["rev-list", "--count", BRANCH], &bare),
        "2",
        "keep 2 caps the published history at two snapshots"
    );
    assert_eq!(
        run_git(&["log", "--format=%s", BRANCH], &bare)
            .lines()
            .collect::<Vec<_>>(),
        vec!["snapshot 4", "snapshot 3"],
        "the two most recent snapshots survive, newest first"
    );
    assert_eq!(
        run_git(&["show", &format!("{BRANCH}:payload.txt")], &bare),
        "snapshot 4"
    );
    assert_eq!(
        run_git(&["show", &format!("{BRANCH}~1:payload.txt")], &bare),
        "snapshot 3",
        "a kept snapshot keeps its own tree, not the tip's"
    );
}

/// The claim design §2 will not take on faith: rotation has to be able to make an existing
/// history *smaller*, not merely stop it from growing.
#[test]
fn lowering_keep_shrinks_a_history_that_is_already_on_the_remote() {
    let (_tmp, remote) = bare_remote();
    let git = ShellGit::new();
    let bare = PathBuf::from(&remote);

    for round in 1..=3 {
        let body = format!("snapshot {round}");
        let transport = snapshot(&[("payload.txt", body.as_bytes())]);
        git.publish(&Snapshot {
            keep: 3,
            ..request_to(transport.path(), &body, &remote)
        })
        .unwrap();
    }
    assert_eq!(run_git(&["rev-list", "--count", BRANCH], &bare), "3");

    let transport = snapshot(&[("payload.txt", b"snapshot 4")]);
    git.publish(&Snapshot {
        keep: 1,
        ..request_to(transport.path(), "snapshot 4", &remote)
    })
    .unwrap();

    assert_eq!(
        run_git(&["rev-list", "--count", BRANCH], &bare),
        "1",
        "publishing with keep 1 must rebuild the branch down to a single snapshot"
    );
    assert_eq!(
        run_git(&["show", &format!("{BRANCH}:payload.txt")], &bare),
        "snapshot 4"
    );
}

/// A guard on what makes rotation affordable: re-committing a kept snapshot needs its commit
/// and tree, never its blobs, and the payload is the whole weight of a transport. If the filter
/// or the depth is ever dropped, a rotation starts downloading hundreds of MiB to rewrite a
/// commit object — and nothing else in the suite would notice.
#[test]
fn a_rotating_publish_fetches_metadata_only_and_a_default_one_does_not_fetch_at_all() {
    let (_tmp, remote) = bare_remote();
    let seed = snapshot(&[("payload.txt", b"snapshot 1")]);
    ShellGit::new()
        .publish(&Snapshot {
            keep: 2,
            ..request_to(seed.path(), "snapshot 1", &remote)
        })
        .unwrap();

    let recorded = Arc::new(RecordingRunner::new());
    let git = ShellGit::with_runner(Box::new(recorded.clone()));
    let next = snapshot(&[("payload.txt", b"snapshot 2")]);
    git.publish(&Snapshot {
        keep: 2,
        ..request_to(next.path(), "snapshot 2", &remote)
    })
    .unwrap();

    let fetches: Vec<String> = recorded
        .rendered()
        .into_iter()
        .filter(|command| command.contains(" fetch "))
        .collect();
    assert_eq!(fetches.len(), 1, "one fetch per rotation: {fetches:?}");
    assert!(
        fetches[0].contains("--filter=blob:none"),
        "a rotation must not download the payload it is rotating: {}",
        fetches[0]
    );
    assert!(
        fetches[0].contains("--depth=1"),
        "keep 2 inherits exactly one snapshot: {}",
        fetches[0]
    );

    let plain = Arc::new(RecordingRunner::new());
    let last = snapshot(&[("payload.txt", b"snapshot 3")]);
    ShellGit::with_runner(Box::new(plain.clone()))
        .publish(&request_to(last.path(), "snapshot 3", &remote))
        .unwrap();
    assert!(
        !plain.rendered().iter().any(|c| c.contains(" fetch ")),
        "a publish without --keep must not talk to the remote before pushing: {:?}",
        plain.rendered()
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
            keep: 0,
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
        keep: 0,
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
fn transport_commits_are_authored_by_the_bot_identity() {
    let transport = snapshot(&[("payload.txt", b"payload")]);
    let git = ShellGit::preview();
    let published = git.publish(&request(transport.path(), "snapshot")).unwrap();

    assert!(!published.commands.is_empty());
    assert!(
        published
            .commands
            .iter()
            .all(|c| c.contains("-c user.name=pixi-sandbox[bot]")),
        "every git invocation must carry the bot name: {:?}",
        published.commands
    );
    assert!(
        published
            .commands
            .iter()
            .all(|c| c
                .contains("-c user.email=41898282+github-actions[bot]@users.noreply.github.com")),
        "every git invocation must carry the github-actions noreply address: {:?}",
        published.commands
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
        keep: 0,
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

#[derive(Debug)]
struct FixedRunner(Output);

impl Runner for FixedRunner {
    fn run(&self, _command: &Command) -> pixi_sandbox_git::Result<Output> {
        Ok(self.0.clone())
    }
}

#[test]
fn worktree_status_files_handles_empty_and_invalid_output() {
    let empty = ShellGit::with_runner(Box::new(FixedRunner(Output::default())));
    assert!(
        empty
            .worktree_status_files(Path::new("."))
            .unwrap()
            .is_empty()
    );

    let invalid = ShellGit::with_runner(Box::new(FixedRunner(Output {
        status: 0,
        stdout: vec![0xff],
        stderr: String::new(),
    })));
    assert!(matches!(
        invalid.worktree_status_files(Path::new(".")),
        Err(Error::NonUtf8 { .. })
    ));
}

#[test]
fn worktree_status_files_reports_tracked_changes_for_xtask() {
    let repo = tempfile::tempdir().expect("tempdir");
    run_git(&["init", "-q"], repo.path());
    std::fs::write(repo.path().join("tracked.txt"), "initial").unwrap();
    run_git(&["add", "tracked.txt"], repo.path());
    run_git(
        &[
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-q",
            "-m",
            "initial",
        ],
        repo.path(),
    );
    std::fs::write(repo.path().join("tracked.txt"), "changed").unwrap();

    let files = ShellGit::new().worktree_status_files(repo.path()).unwrap();
    assert_eq!(files, vec!["tracked.txt"]);
}
