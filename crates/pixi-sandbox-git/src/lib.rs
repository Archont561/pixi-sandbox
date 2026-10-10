//! Git access, behind a trait — so `publish` and the airlock's fetch are testable without a
//! remote, a network, or somebody's own repository.
//!
//! Two implementations ship here:
//!
//! * [`ShellGit`] drives a real `git` binary. It takes a [`Runner`], which is where the
//!   testing leverage comes from: [`RecordingRunner`] executes for real *and* records every
//!   argv, [`PreviewRunner`] records and executes nothing (that is what `publish --dry-run`
//!   uses), so the exact command line is assertable without a network.
//! * [`FakeGit`] is an in-memory mock: remotes, branch tips, an operation log, and a switch
//!   that makes a push fail. Integration tests of `publish`/fetch use it and never touch git.
//!
//! # The contract both implementations must keep
//!
//! These are the semantics tests assert, and the reason the trait is *semantic* (`publish`,
//! `fetch_into`) rather than a thin wrapper around argv:
//!
//! 1. **Publishing is an orphan.** One call produces exactly one commit with no parent on the
//!    destination branch and force-pushes it. History is *replaced*, never appended — that is
//!    what makes a branch a snapshot rather than a changelog (design.md §2, decision D1).
//!    With [`Snapshot::keep`] > 1 the branch is *rebuilt* to the N most recent snapshots
//!    instead: still a force-push of a history this call constructed, never an append.
//! 2. **The snapshot directory is read-only.** `publish` never writes into it, never creates a
//!    `.git` in it; the payload is not copied, either — objects and the index go to a scratch
//!    directory next to it, which is removed before returning.
//! 3. **A transport contains plain files.** A symlink is refused up front: the airlock cannot
//!    resolve one, and a broken transport must fail at pack time, not there.
//! 4. **`fetch_into` produces exactly the branch tree** in an empty directory, byte for byte,
//!    and leaves no `.git` behind (the airlock wants the branch *content*, and the tool that
//!    reads it is the same one that verifies the hashes).

mod fake;
mod shell;

pub use fake::{FakeGit, Op};
pub use shell::{
    Command, Output, PreviewRunner, ProcessRunner, RecordingRunner, Runner, ShellGit,
    current_commit,
};

use std::path::{Path, PathBuf};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Scratch directory name used inside the transport while publishing (removed afterwards).
pub const DEFAULT_SCRATCH_NAME: &str = ".pixi-sandbox-publish";

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{program}` could not be run: {source}")]
    Runner {
        program: String,
        #[source]
        source: std::io::Error,
    },

    #[error("`{command}` failed (exit {status}): {stderr}")]
    Command {
        command: String,
        status: i32,
        stderr: String,
    },

    #[error("`{command}` produced output that is not UTF-8")]
    NonUtf8 { command: String },

    /// A push the remote refused — a protected branch, a missing write permission, or a
    /// ruleset. Worth its own variant because the fix is on the remote, not in the payload.
    #[error(
        "the remote {remote} rejected the push to {branch}: {stderr} \
         (design.md §2 / decision D1 — the sandbox branch must be allowed to be force-pushed)"
    )]
    Rejected {
        remote: String,
        branch: String,
        stderr: String,
    },

    #[error("{remote} has no branch {branch}")]
    MissingBranch { remote: String, branch: String },

    #[error("invalid snapshot: {0}")]
    InvalidSnapshot(String),
}

impl Error {
    pub(crate) fn io(path: &Path, source: std::io::Error) -> Self {
        Error::Io {
            path: path.display().to_string(),
            source,
        }
    }
}

/// One publish request. Kept as a struct so both implementations see the same five facts.
#[derive(Debug, Clone, Copy)]
pub struct Snapshot<'a> {
    /// The transport directory (read-only, see the contract above).
    pub dir: &'a Path,
    /// Branch to replace, e.g. `sandbox/dev-linux-64`.
    pub branch: &'a str,
    /// Remote URL or path (`origin` is only a CLI default).
    pub remote: &'a str,
    /// Commit message — the manifest summary, so the branch is self-describing in `git log`.
    pub message: &'a str,
    /// How many snapshots the branch may carry afterwards. `0` and `1` both mean the
    /// single-commit orphan branch; a larger N *rebuilds* the history to the N most recent
    /// snapshots (design.md §2 — a force-push is not a shrink, so retention cannot append).
    pub keep: u32,
}

impl Snapshot<'_> {
    /// Snapshots to leave on the branch: rotation is opt-in, so anything below 2 is the
    /// historical single-snapshot behaviour.
    #[must_use]
    pub fn retained(&self) -> usize {
        self.keep.max(1) as usize
    }
}

/// What a publish or fetch did, for reporting and for CI logs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Published {
    pub commit: String,
    pub files: u64,
    pub bytes: u64,
    /// The git commands that ran (empty for [`FakeGit`]); `--dry-run` prints exactly these.
    pub commands: Vec<String>,
}

/// One reviewed-file commit on a branch of the *consumer's own* checkout (TASK-76's
/// upgrade lane).
///
/// Resets `branch` to the working tree's current HEAD, stages exactly `files`, and commits
/// them. The commit is authored by the implementation's configured identity — the
/// automation bot, never the invoking user's git config. Does not push.
#[derive(Debug, Clone)]
pub struct FileCommit<'a> {
    /// The consumer's checked-out working tree.
    pub work_tree: &'a Path,
    /// Branch to create or reset at the current HEAD, then commit onto. A name derived from
    /// a version, so a re-proposal reuses it.
    pub branch: &'a str,
    /// Paths to stage, relative to `work_tree`. Exactly these — nothing else is ever staged.
    pub files: &'a [PathBuf],
    /// Commit message.
    pub message: &'a str,
}

/// What a [`GitProtocol::commit_files`] call produced.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileCommitted {
    /// Full sha of the new commit — empty when `changed` is `false`.
    pub commit: String,
    /// `false` when the staged files matched HEAD: no commit was created, there is nothing
    /// to propose. This is an answer, not an error.
    pub changed: bool,
    /// `git diff HEAD^ HEAD --binary` — the patch artifact for the delivery fallback. Empty
    /// when `changed` is `false`.
    pub patch: Vec<u8>,
}

/// The operations this project needs from git — and not one more.
pub trait GitProtocol: Send + Sync + std::fmt::Debug {
    /// Commit `snapshot.dir` as a single parentless commit and force-push it.
    ///
    /// # Errors
    ///
    /// Returns an error if the snapshot directory cannot be read, a git command fails, or
    /// the remote rejects the push.
    fn publish(&self, snapshot: &Snapshot<'_>) -> Result<Published>;

    /// Materialise the tree of `branch` on `remote` into `dest` (which must be empty).
    ///
    /// # Errors
    ///
    /// Returns an error if `dest` is not empty, the fetch fails, or the branch tree cannot
    /// be materialised.
    fn fetch_into(&self, remote: &str, branch: &str, dest: &Path) -> Result<Published>;

    /// Does the branch exist on the remote? Used by `publish --keep`/rotation and by tests.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote cannot be queried — an unreachable remote is an
    /// error, not a `false`.
    fn branch_exists(&self, remote: &str, branch: &str) -> Result<bool>;

    /// Size of the served payload for `branch`, when the remote is reachable as a local path.
    /// The git wire protocol has no "how big is this branch" query, hence `None` for a URL.
    ///
    /// # Errors
    ///
    /// Returns an error if the `git` invocation itself cannot be run.
    fn remote_size(&self, remote: &str, branch: &str) -> Result<Option<u64>>;

    /// Reset `commit.branch` to the working tree's HEAD, stage exactly `commit.files`, and
    /// commit them under the implementation's identity. See [`FileCommit`]/[`FileCommitted`]
    /// for the contract; a no-change result is `changed == false`, never an error.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkout, the staging, or the commit fails.
    fn commit_files(&self, commit: &FileCommit<'_>) -> Result<FileCommitted>;

    /// Push `branch` from the working tree at `work_tree` to `remote`. `force` replaces an
    /// existing branch — the upgrade lane re-proposes onto a version-derived name.
    ///
    /// # Errors
    ///
    /// Returns an error if the remote rejects the push.
    fn push_branch(&self, work_tree: &Path, remote: &str, branch: &str, force: bool) -> Result<()>;
}

/// Every file under `dir`, as `(relative path with `/` separators, size in bytes)`.
///
/// Refuses symlinks and an empty directory: both are mistakes that would only show up on the
/// airlock as a restore that verifies against a payload nobody can install.
///
/// # Errors
///
/// Returns an error if `dir` is not a directory, is empty, contains a symlink, or cannot
/// be read.
pub fn snapshot_files(dir: &Path) -> Result<Vec<(String, u64)>> {
    if !dir.is_dir() {
        return Err(Error::InvalidSnapshot(format!(
            "{} is not a directory",
            dir.display()
        )));
    }
    let mut out = Vec::new();
    collect(dir, dir, &mut out)?;
    out.sort();
    if out.is_empty() {
        return Err(Error::InvalidSnapshot(format!(
            "{} is empty — there is nothing to publish",
            dir.display()
        )));
    }
    Ok(out)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, u64)>) -> Result<()> {
    let entries = std::fs::read_dir(dir).map_err(|e| Error::io(dir, e))?;
    for entry in entries {
        let entry = entry.map_err(|e| Error::io(dir, e))?;
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(|e| Error::io(&path, e))?;
        if meta.file_type().is_symlink() {
            return Err(Error::InvalidSnapshot(format!(
                "{} is a symlink; a transport must contain plain files (an airlock cannot resolve it)",
                path.display()
            )));
        }
        if meta.is_dir() {
            collect(root, &path, out)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            out.push((rel, meta.len()));
        }
    }
    Ok(())
}

/// Total size of a snapshot, in bytes.
#[must_use]
pub fn snapshot_bytes(files: &[(String, u64)]) -> u64 {
    files.iter().map(|(_, size)| size).sum()
}
