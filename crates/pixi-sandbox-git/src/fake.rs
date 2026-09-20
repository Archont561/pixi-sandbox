//! An in-memory git: remotes, branch tips, an operation log, and a way to fail a push.
//!
//! Integration tests use this instead of a real repository, which keeps them fast, offline and
//! — the point of the exercise — impossible to run against somebody's actual project.

use crate::{Error, GitProtocol, Published, Result, Snapshot, snapshot_bytes, snapshot_files};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

/// One recorded call, so a test can assert *what* happened, not just the end state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Op {
    Publish {
        remote: String,
        branch: String,
        commit: String,
        files: u64,
    },
    Push {
        remote: String,
        branch: String,
        forced: bool,
    },
    Fetch {
        remote: String,
        branch: String,
        dest: String,
    },
    BranchExists {
        remote: String,
        branch: String,
    },
    RemoteSize {
        remote: String,
        branch: String,
    },
}

#[derive(Debug, Clone, Default)]
struct RemoteBranch {
    commit: String,
    /// The commit history the remote would serve. A publish replaces it (orphan + force);
    /// a mock that appended here would let a broken publish pass its tests.
    history: Vec<String>,
    files: BTreeMap<String, Vec<u8>>,
}

#[derive(Debug, Default)]
struct State {
    branches: BTreeMap<(String, String), RemoteBranch>,
    log: Vec<Op>,
    counter: u64,
    fail_push: Option<String>,
}

/// A mock remote. Cheap to build, impossible to misuse.
#[derive(Debug, Default)]
pub struct FakeGit {
    state: Mutex<State>,
}

impl FakeGit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Make the next push fail (a protected branch, a revoked token, …). The remote is left
    /// unchanged — a rejected push must not look like a successful one.
    pub fn fail_next_push(&self, reason: &str) {
        self.state.lock().expect("fake git lock").fail_push = Some(reason.to_string());
    }

    pub fn ops(&self) -> Vec<Op> {
        self.state.lock().expect("fake git lock").log.clone()
    }

    pub fn tip(&self, remote: &str, branch: &str) -> Option<String> {
        self.state
            .lock()
            .expect("fake git lock")
            .branches
            .get(&(remote.to_string(), branch.to_string()))
            .map(|b| b.commit.clone())
    }

    /// How many commits the remote would serve for this branch. Always 1 after a publish:
    /// that is the orphan-branch contract (design.md §2).
    pub fn history(&self, remote: &str, branch: &str) -> Vec<String> {
        self.state
            .lock()
            .expect("fake git lock")
            .branches
            .get(&(remote.to_string(), branch.to_string()))
            .map(|b| b.history.clone())
            .unwrap_or_default()
    }

    pub fn files(&self, remote: &str, branch: &str) -> Option<BTreeMap<String, Vec<u8>>> {
        self.state
            .lock()
            .expect("fake git lock")
            .branches
            .get(&(remote.to_string(), branch.to_string()))
            .map(|b| b.files.clone())
    }

    /// How often a push was attempted (successful or not) — asserts one push per publish.
    pub fn pushes(&self) -> usize {
        self.state
            .lock()
            .expect("fake git lock")
            .log
            .iter()
            .filter(|op| matches!(op, Op::Push { .. }))
            .count()
    }
}

impl GitProtocol for FakeGit {
    fn publish(&self, snapshot: &Snapshot<'_>) -> Result<Published> {
        // Validate the snapshot exactly like the real implementation does.
        let listing = snapshot_files(snapshot.dir)?;
        let bytes = snapshot_bytes(&listing);

        let mut state = self.state.lock().expect("fake git lock");
        state.counter += 1;
        let branch = (snapshot.remote.to_string(), snapshot.branch.to_string());
        if let Some(reason) = state.fail_push.take() {
            state.log.push(Op::Push {
                remote: snapshot.remote.to_string(),
                branch: snapshot.branch.to_string(),
                forced: true,
            });
            return Err(Error::Rejected {
                remote: snapshot.remote.to_string(),
                branch: snapshot.branch.to_string(),
                stderr: reason,
            });
        }

        let commit = format!("mock{:036x}", state.counter);
        let mut files = BTreeMap::new();
        for (path, _) in &listing {
            let full = snapshot.dir.join(path);
            let content = std::fs::read(&full).map_err(|e| Error::io(&full, e))?;
            files.insert(path.clone(), content);
        }

        state.branches.insert(
            branch,
            RemoteBranch {
                commit: commit.clone(),
                history: vec![commit.clone()],
                files,
            },
        );
        state.log.push(Op::Publish {
            remote: snapshot.remote.to_string(),
            branch: snapshot.branch.to_string(),
            commit: commit.clone(),
            files: listing.len() as u64,
        });
        state.log.push(Op::Push {
            remote: snapshot.remote.to_string(),
            branch: snapshot.branch.to_string(),
            forced: true,
        });

        Ok(Published {
            commit,
            files: listing.len() as u64,
            bytes,
            commands: Vec::new(),
        })
    }

    fn fetch_into(&self, remote: &str, branch: &str, dest: &Path) -> Result<Published> {
        let mut state = self.state.lock().expect("fake git lock");
        state.log.push(Op::Fetch {
            remote: remote.to_string(),
            branch: branch.to_string(),
            dest: dest.display().to_string(),
        });
        let Some(entry) = state
            .branches
            .get(&(remote.to_string(), branch.to_string()))
        else {
            return Err(Error::MissingBranch {
                remote: remote.to_string(),
                branch: branch.to_string(),
            });
        };
        if dest.exists()
            && std::fs::read_dir(dest)
                .map_err(|e| Error::io(dest, e))?
                .next()
                .is_some()
        {
            return Err(Error::InvalidSnapshot(format!(
                "{} is not empty — fetch into an empty directory",
                dest.display()
            )));
        }

        let mut bytes = 0u64;
        for (path, content) in &entry.files {
            let out = dest.join(path);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            std::fs::write(&out, content).map_err(|e| Error::io(&out, e))?;
            bytes += content.len() as u64;
        }

        Ok(Published {
            commit: entry.commit.clone(),
            files: entry.files.len() as u64,
            bytes,
            commands: Vec::new(),
        })
    }

    fn branch_exists(&self, remote: &str, branch: &str) -> Result<bool> {
        let mut state = self.state.lock().expect("fake git lock");
        state.log.push(Op::BranchExists {
            remote: remote.to_string(),
            branch: branch.to_string(),
        });
        Ok(state
            .branches
            .contains_key(&(remote.to_string(), branch.to_string())))
    }

    fn remote_size(&self, remote: &str, branch: &str) -> Result<Option<u64>> {
        let mut state = self.state.lock().expect("fake git lock");
        state.log.push(Op::RemoteSize {
            remote: remote.to_string(),
            branch: branch.to_string(),
        });
        match state
            .branches
            .get(&(remote.to_string(), branch.to_string()))
        {
            Some(entry) => Ok(Some(
                entry.files.values().map(|c| c.len() as u64).sum::<u64>(),
            )),
            None => Err(Error::MissingBranch {
                remote: remote.to_string(),
                branch: branch.to_string(),
            }),
        }
    }
}
