//! `starter::clone_publishable` — the tree the starter *would* publish, as a fresh clone.
//!
//! The starter lane proves its template by running the documented development task, and the
//! proof only means something if it runs against the artifact a user gets: a clone of the
//! revision, in a clean directory. That excludes the `.pixi/` the assembly steps necessarily
//! write next door, which is the false positive that used to make the verifier refuse a
//! publishable starter (task-85). Tests build the world with real `git` in tempdirs — D10 keeps
//! this file away from the repository it lives in.

use std::path::Path;
use xtask::starter::{clone_publishable, verify};

mod support;
use support::{STARTER_TAG as TAG, commit_all, commit_count, run_git, runtime_state, starter_tree};

/// The publisher this fixture carries: `starter_tree` writes both version stamps from one
/// argument, and the verifier reads both.
const VERSION: &str = "1.2.3";

/// A starter tree as the workflow leaves it: written, committed once by an earlier revision, then
/// dirtied by the tooling that installed an environment into it.
fn assembled_starter(dir: &Path) {
    starter_tree(dir, VERSION);
    run_git(dir, &["init", "-q"]);
    commit_all(dir, "seed the starter");
    runtime_state(dir);
}

/// The same tree with nothing to ask: the refusal has to come from git having no repository to
/// answer for, not from a file missing out of the starter.
fn starter_without_a_repository(dir: &Path) {
    starter_tree(dir, VERSION);
    runtime_state(dir);
}

/// A clean checkout must contain exactly the sources, so that "the fresh clone works" is a claim
/// about the artifact and not about the machine that assembled it.
#[test]
fn the_clone_carries_the_publishable_tree_and_none_of_the_runtime_state() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);

    let clone = clone_publishable(&dir, &root.path().join("clone")).expect("assemble a clone");

    assert_eq!(clone, root.path().join("clone/starter"));
    for source in [
        "pixi.toml",
        "pixi.lock",
        "pixi-sandbox.toml",
        ".gitignore",
        "README.md",
        "restore.sh",
        ".github/workflows/publish-sandbox.yml",
    ] {
        assert!(
            clone.join(source).exists(),
            "{source} is what the starter would publish, so the clone must carry it"
        );
    }
    assert!(!clone.join(".pixi").exists(), "runtime state leaked");
    assert!(
        !clone.join(".pixi-sandbox").exists(),
        "transport state leaked — it is not in the revision, and the clone must not invent it"
    );
    assert_eq!(commit_count(&clone), "1", "the clone is a fresh clone");
}

/// The point of the exercise: the tree the dev-proof runs in is the tree the verifier accepts,
/// with no ignore rules left to interpret.
#[test]
fn the_clone_is_a_starter_revision_the_verifier_accepts_without_qualification() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);

    let clone = clone_publishable(&dir, &root.path().join("clone")).expect("assemble a clone");
    assert_eq!(
        verify(&clone, TAG).expect("verify runs"),
        Vec::<String>::new()
    );
}

/// A file the assembly steps added that git *would* take has to be in the clone — otherwise the
/// proof runs on the previous revision and says nothing about the one being published.
#[test]
fn a_new_untracked_source_file_reaches_the_clone() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);
    std::fs::create_dir_all(dir.join("docs")).expect("mkdir");
    std::fs::write(dir.join("docs/new.md"), "# added after the seed commit\n").expect("write");

    let clone = clone_publishable(&dir, &root.path().join("clone")).expect("assemble a clone");
    assert!(
        clone.join("docs/new.md").exists(),
        "the clone must be the tree git would commit, not the tree already committed"
    );
}

/// `git` records the executable bit and nothing else about a file's mode, so a launcher copied
/// without it would clone into something a user could not run.
#[cfg(unix)]
#[test]
fn an_executable_launcher_keeps_its_bit_across_the_clone() {
    use std::os::unix::fs::PermissionsExt;

    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);
    std::fs::set_permissions(
        dir.join("restore.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .expect("chmod");

    let clone = clone_publishable(&dir, &root.path().join("clone")).expect("assemble a clone");
    let mode = std::fs::metadata(clone.join("restore.sh"))
        .expect("metadata")
        .permissions()
        .mode();
    assert!(mode & 0o111 != 0, "restore.sh lost its bit: {mode:#o}");
}

/// A clone is not a launderer: runtime state that no ignore rule covers is carried *and* refused,
/// so the proof cannot turn a real finding into a clean one.
#[test]
fn runtime_state_no_ignore_rule_covers_is_carried_and_still_a_finding() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);
    std::fs::remove_file(dir.join(".gitignore")).expect("rm .gitignore");
    commit_all(&dir, "drop the ignore file");
    std::fs::write(dir.join(".pixi/envs/default/rg"), "now unignored\n").expect("write");

    let clone = clone_publishable(&dir, &root.path().join("clone")).expect("assemble a clone");
    assert!(
        clone.join(".pixi/envs/default/rg").exists(),
        "git would commit it, so the clone must carry it"
    );
    let findings = verify(&clone, TAG).expect("verify runs");
    assert!(
        findings.iter().any(|f| f.contains(".pixi")),
        "the clone must not hide a real finding: {findings:?}"
    );
}

#[test]
fn a_directory_that_is_not_a_work_tree_is_refused_naming_the_remedy() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("loose-tree");
    std::fs::create_dir_all(&dir).expect("mkdir");
    starter_without_a_repository(&dir);

    let error = clone_publishable(&dir, &root.path().join("clone")).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("work tree"), "{message}");
    assert!(message.contains("starter"), "{message}");
}

/// The proof runs in a scratch directory the lane owns; silently reusing one that already holds
/// a previous attempt would mix two trees into one verdict.
#[test]
fn an_out_directory_that_is_not_empty_is_refused() {
    let root = tempfile::tempdir().expect("tempdir");
    let dir = root.path().join("starter");
    std::fs::create_dir_all(&dir).expect("mkdir");
    assembled_starter(&dir);
    let out = root.path().join("clone");
    std::fs::create_dir_all(&out).expect("mkdir");
    std::fs::write(out.join("previous-attempt.txt"), "stale\n").expect("write");

    let error = clone_publishable(&dir, &out).unwrap_err();
    let message = format!("{error:#}");
    assert!(message.contains("empty"), "{message}");
}
