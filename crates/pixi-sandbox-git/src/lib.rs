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
pub use shell::{Command, Output, PreviewRunner, ProcessRunner, RecordingRunner, Runner, ShellGit};

use std::path::Path;
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

/// The operations this project needs from git — and not one more.
pub trait GitProtocol: Send + Sync + std::fmt::Debug {
    /// Commit `snapshot.dir` as a single parentless commit and force-push it.
    fn publish(&self, snapshot: &Snapshot<'_>) -> Result<Published>;

    /// Materialise the tree of `branch` on `remote` into `dest` (which must be empty).
    fn fetch_into(&self, remote: &str, branch: &str, dest: &Path) -> Result<Published>;

    /// Does the branch exist on the remote? Used by `publish --keep`/rotation and by tests.
    fn branch_exists(&self, remote: &str, branch: &str) -> Result<bool>;

    /// Size of the served payload for `branch`, when the remote is reachable as a local path.
    /// The git wire protocol has no "how big is this branch" query, hence `None` for a URL.
    fn remote_size(&self, remote: &str, branch: &str) -> Result<Option<u64>>;
}

/// Every file under `dir`, as `(relative path with `/` separators, size in bytes)`.
///
/// Refuses symlinks and an empty directory: both are mistakes that would only show up on the
/// airlock as a restore that verifies against a payload nobody can install.
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
pub fn snapshot_bytes(files: &[(String, u64)]) -> u64 {
    files.iter().map(|(_, size)| size).sum()
}
