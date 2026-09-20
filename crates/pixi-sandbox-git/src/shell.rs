//! The real implementation: a `git` binary driven through a [`Runner`].

use crate::{
    DEFAULT_SCRATCH_NAME, Error, GitProtocol, Published, Result, Snapshot, snapshot_bytes,
    snapshot_files,
};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::Mutex;

/// A command, as data — so tests can assert it without executing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl Command {
    pub fn new(program: impl Into<String>) -> Self {
        Command {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, dir: impl Into<PathBuf>) -> Self {
        self.cwd = Some(dir.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }

    /// A shell-ish rendering, for logs, `--dry-run`, and readable test failures.
    pub fn rendered(&self) -> String {
        let env: String = self.env.iter().map(|(k, v)| format!("{k}={v} ")).collect();
        let args: String = self
            .args
            .iter()
            .map(|a| format!("{a} "))
            .collect::<String>()
            .trim_end()
            .to_string();
        format!("{env}{} {args}", self.program)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Output {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

impl Output {
    pub fn ok(&self) -> bool {
        self.status == 0
    }

    pub fn utf8(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
}

/// How commands get executed. Implement this to record, preview, or fake git.
pub trait Runner: Send + Sync + std::fmt::Debug {
    fn run(&self, command: &Command) -> Result<Output>;
}

/// Lets a test keep a handle to a runner while [`ShellGit`] owns the box.
impl<T: Runner + ?Sized> Runner for std::sync::Arc<T> {
    fn run(&self, command: &Command) -> Result<Output> {
        (**self).run(command)
    }
}

/// Executes the command for real.
#[derive(Debug, Default)]
pub struct ProcessRunner;

impl Runner for ProcessRunner {
    fn run(&self, command: &Command) -> Result<Output> {
        let mut cmd = StdCommand::new(&command.program);
        cmd.args(&command.args);
        if let Some(cwd) = &command.cwd {
            cmd.current_dir(cwd);
        }
        for (key, value) in &command.env {
            cmd.env(key, value);
        }
        let out = cmd.output().map_err(|e| Error::Runner {
            program: command.program.clone(),
            source: e,
        })?;
        Ok(Output {
            status: out.status.code().unwrap_or(-1),
            stdout: out.stdout,
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}

/// Runs for real *and* remembers every command — used by tests that want to assert the argv
/// (for example that `commit-tree` is called without `-p`, i.e. the commit is an orphan).
#[derive(Debug, Default)]
pub struct RecordingRunner {
    inner: ProcessRunner,
    seen: Mutex<Vec<Command>>,
}

impl RecordingRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> Vec<Command> {
        self.seen.lock().expect("runner lock").clone()
    }

    pub fn rendered(&self) -> Vec<String> {
        self.commands().iter().map(Command::rendered).collect()
    }
}

impl Runner for RecordingRunner {
    fn run(&self, command: &Command) -> Result<Output> {
        self.seen.lock().expect("runner lock").push(command.clone());
        self.inner.run(command)
    }
}

/// Records and executes nothing: the honest way to implement `--dry-run`.
///
/// Outputs are plausible placeholders (a zero object id), which keeps a preview run of
/// `publish` walking the same code path as the real thing.
#[derive(Debug, Default)]
pub struct PreviewRunner {
    seen: Mutex<Vec<Command>>,
}

impl PreviewRunner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commands(&self) -> Vec<Command> {
        self.seen.lock().expect("runner lock").clone()
    }

    pub fn rendered(&self) -> Vec<String> {
        self.commands().iter().map(Command::rendered).collect()
    }
}

pub(crate) const ZERO_OBJECT_ID: &str = "0000000000000000000000000000000000000000";

impl Runner for PreviewRunner {
    fn run(&self, command: &Command) -> Result<Output> {
        self.seen.lock().expect("runner lock").push(command.clone());
        Ok(Output {
            status: 0,
            stdout: ZERO_OBJECT_ID.as_bytes().to_vec(),
            stderr: String::new(),
        })
    }
}

/// Git, by running `git`.
#[derive(Debug)]
pub struct ShellGit {
    runner: Box<dyn Runner>,
    program: String,
    name: String,
    email: String,
    scratch_name: String,
    log: Mutex<Vec<String>>,
}

impl Default for ShellGit {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellGit {
    pub fn new() -> Self {
        ShellGit {
            runner: Box::new(ProcessRunner),
            program: "git".to_string(),
            name: "pixi-sandbox".to_string(),
            email: "pixi-sandbox@invalid".to_string(),
            scratch_name: DEFAULT_SCRATCH_NAME.to_string(),
            log: Mutex::new(Vec::new()),
        }
    }

    pub fn with_runner(runner: Box<dyn Runner>) -> Self {
        ShellGit {
            runner,
            ..Self::new()
        }
    }

    /// A `ShellGit` that runs nothing: every call returns the commands it *would* run.
    pub fn preview() -> Self {
        Self::with_runner(Box::new(PreviewRunner::new()))
    }

    pub fn program(mut self, program: impl Into<String>) -> Self {
        self.program = program.into();
        self
    }

    pub fn identity(mut self, name: impl Into<String>, email: impl Into<String>) -> Self {
        self.name = name.into();
        self.email = email.into();
        self
    }

    pub fn scratch_name(mut self, name: impl Into<String>) -> Self {
        self.scratch_name = name.into();
        self
    }

    /// Every command this instance has run (or previewed), oldest first.
    pub fn log(&self) -> Vec<String> {
        self.log.lock().expect("git log lock").clone()
    }

    fn git<I, S>(&self, args: I) -> Command
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Command::new(self.program.clone())
            .args(["-c", &format!("user.name={}", self.name)])
            .args(["-c", &format!("user.email={}", self.email)])
            .args(args)
    }

    fn run(&self, command: &Command) -> Result<Output> {
        self.log
            .lock()
            .expect("git log lock")
            .push(command.rendered());
        let output = self.runner.run(command)?;
        if !output.ok() {
            return Err(Error::Command {
                command: command.rendered(),
                status: output.status,
                stderr: output.stderr.trim().to_string(),
            });
        }
        Ok(output)
    }

    fn run_text(&self, command: &Command) -> Result<String> {
        let output = self.run(command)?;
        match String::from_utf8(output.stdout) {
            Ok(text) => Ok(text.trim().to_string()),
            Err(_) => Err(Error::NonUtf8 {
                command: command.rendered(),
            }),
        }
    }

    fn commands_since(&self, from: usize) -> Vec<String> {
        self.log().split_off(from.min(self.log().len()))
    }

    fn push(&self, remote: &str, branch: &str, git_dir: &Path) -> Result<()> {
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let command = self
            .git(["push", "--force"])
            .arg(remote)
            .arg(&refspec)
            .env("GIT_DIR", path(git_dir));
        match self.run(&command) {
            Err(Error::Command { stderr, .. }) => Err(Error::Rejected {
                remote: remote.to_string(),
                branch: branch.to_string(),
                stderr,
            }),
            other => other.map(|_| ()),
        }
    }
}

/// Everything a transport needs from a checkout of a branch, and nothing else.
impl GitProtocol for ShellGit {
    fn publish(&self, snapshot: &Snapshot<'_>) -> Result<Published> {
        let files = snapshot_files(snapshot.dir)?;
        let from = self.log().len();

        // Objects and the index live in a scratch git dir placed *next to* the transport
        // (same filesystem as the transport, never /tmp), while the work tree stays exactly
        // where it is: no copy of the payload, and no `.git` in a directory we do not own.
        // The scratch is deliberately outside the transport so `git add -A -f` does not
        // include the scratch's own files (e.g. `index.lock`) in the orphan commit.
        // A `.git` directory inside the work tree is ignored by git, but a custom-named
        // GIT_DIR is not — the previous in-transport location therefore leaked
        // `.pixi-sandbox-publish-<pid>/` into the published branch.
        // Removed by the guard even if a later step fails.
        let scratch_parent = snapshot.dir.parent().unwrap_or(snapshot.dir);
        let scratch = scratch_parent.join(format!("{}-{}", self.scratch_name, std::process::id()));
        let _guard = Scratch::new(scratch.clone());
        let index = scratch.join("index");

        self.run(&self.git(["init", "-q", "--bare"]).arg(path(&scratch)))?;

        let with_scratch = |command: Command| {
            command
                .env("GIT_DIR", path(&scratch))
                .env("GIT_WORK_TREE", path(snapshot.dir))
                .env("GIT_INDEX_FILE", path(&index))
        };

        self.run(&with_scratch(self.git(["add", "-A", "-f"])).cwd(snapshot.dir))?;
        let tree = self.run_text(&with_scratch(self.git(["write-tree"])))?;
        // No `-p`: the commit is parentless, so the branch is an orphan by construction.
        let commit = self.run_text(&with_scratch(
            self.git(["commit-tree"])
                .arg(&tree)
                .args(["-m", snapshot.message]),
        ))?;
        self.run(&with_scratch(
            self.git(["update-ref"])
                .arg(format!("refs/heads/{}", snapshot.branch))
                .arg(&commit),
        ))?;
        self.push(snapshot.remote, snapshot.branch, &scratch)?;

        Ok(Published {
            commit,
            files: files.len() as u64,
            bytes: snapshot_bytes(&files),
            commands: self.commands_since(from),
        })
    }

    fn fetch_into(&self, remote: &str, branch: &str, dest: &Path) -> Result<Published> {
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
        let from = self.log().len();
        self.run(&self.git(["init", "-q"]).arg(path(dest)))?;
        self.run(
            &self
                .git(["fetch", "--no-tags"])
                .arg(remote)
                .arg(format!("refs/heads/{branch}"))
                .cwd(dest),
        )?;
        let commit = self.run_text(&self.git(["rev-parse", "FETCH_HEAD"]).cwd(dest))?;

        // Materialise the tree with plumbing rather than a worktree checkout: no tar, no
        // external tools, and byte-exact control over what lands in `dest`.
        let listing = self.run(&self.git(["ls-tree", "-r", "-z", "FETCH_HEAD"]).cwd(dest))?;
        let mut files = 0u64;
        let mut bytes = 0u64;
        for (mode, kind, oid, path) in parse_ls_tree(&listing.stdout) {
            if kind != "blob" {
                return Err(Error::InvalidSnapshot(format!(
                    "the branch contains a {kind} at {path} (mode {mode}); a transport must be plain files"
                )));
            }
            let blob = self
                .runner
                .run(&self.git(["cat-file", "blob", &oid]).cwd(dest))?;
            if !blob.ok() {
                return Err(Error::Command {
                    command: format!("git cat-file blob {oid}"),
                    status: blob.status,
                    stderr: blob.stderr.trim().to_string(),
                });
            }
            let out = dest.join(&path);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
            }
            std::fs::write(&out, &blob.stdout).map_err(|e| Error::io(&out, e))?;
            files += 1;
            bytes += blob.stdout.len() as u64;
        }

        // The airlock wants the branch *content*; the tool that reads it verifies hashes.
        let git_dir = dest.join(".git");
        std::fs::remove_dir_all(&git_dir).map_err(|e| Error::io(&git_dir, e))?;

        Ok(Published {
            commit,
            files,
            bytes,
            commands: self.commands_since(from),
        })
    }

    fn branch_exists(&self, remote: &str, branch: &str) -> Result<bool> {
        let command = self
            .git(["ls-remote", "--heads"])
            .arg(remote)
            .arg(format!("refs/heads/{branch}"));
        Ok(!self.run_text(&command)?.is_empty())
    }

    fn remote_size(&self, remote: &str, branch: &str) -> Result<Option<u64>> {
        // Only knowable without speaking to a server: a local path with an object store.
        let repo = Path::new(remote);
        if !repo.join("objects").is_dir() {
            return Ok(None);
        }
        let command = self.git(["count-objects", "-v"]).cwd(repo);
        let output = self.runner.run(&command)?;
        if !output.ok() {
            return Ok(None);
        }
        let mut bytes = 0u64;
        for line in output.utf8().lines() {
            if let Some((key, value)) = line.split_once(": ")
                && let Ok(kib) = value.trim().parse::<u64>()
                && matches!(key, "size-pack" | "size")
            {
                bytes += kib * 1024;
            }
        }
        let _ = branch; // the object store is per repository, not per branch
        Ok(Some(bytes))
    }
}

/// `git ls-tree -r -z` output: `<mode> <type> <object>\t<path>\0`.
pub(crate) fn parse_ls_tree(bytes: &[u8]) -> Vec<(String, String, String, String)> {
    bytes
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .filter_map(|chunk| {
            let tab = chunk.iter().position(|b| *b == b'\t')?;
            let (meta, path) = chunk.split_at(tab);
            let meta = String::from_utf8_lossy(meta).into_owned();
            let mut fields = meta.split(' ');
            let mode = fields.next()?.to_string();
            let kind = fields.next()?.to_string();
            let oid = fields.next()?.to_string();
            Some((
                mode,
                kind,
                oid,
                String::from_utf8_lossy(&path[1..]).into_owned(),
            ))
        })
        .collect()
}

/// Removes the scratch git dir on drop, whatever happened.
struct Scratch(PathBuf);

impl Scratch {
    fn new(path: PathBuf) -> Self {
        Scratch(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
