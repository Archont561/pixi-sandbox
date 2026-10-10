//! The real implementation: a `git` binary driven through a [`Runner`].

use crate::{
    DEFAULT_SCRATCH_NAME, Error, FileCommit, FileCommitted, GitProtocol, Published, Result,
    Snapshot, snapshot_bytes, snapshot_files,
};
use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    #[must_use]
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
    #[must_use]
    pub fn ok(&self) -> bool {
        self.status == 0
    }

    #[must_use]
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

/// Read the short commit at `root`, when it is a Git work tree.
///
/// Pack treats source provenance as optional, so an unavailable Git executable, a non-repository,
/// or non-UTF-8 output all become `None`; the command still lives behind this crate's runner.
#[must_use]
pub fn current_commit(root: &Path) -> Option<String> {
    current_commit_with(&ProcessRunner, root)
}

fn current_commit_with(runner: &dyn Runner, root: &Path) -> Option<String> {
    let output = runner
        .run(
            &Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .cwd(root),
        )
        .ok()?;
    output
        .ok()
        .then(|| String::from_utf8(output.stdout).ok())
        .flatten()
        .map(|commit| commit.trim().to_string())
        .filter(|commit| !commit.is_empty())
}

/// Runs for real *and* remembers every command — used by tests that want to assert the argv
/// (for example that `commit-tree` is called without `-p`, i.e. the commit is an orphan).
#[derive(Debug, Default)]
pub struct RecordingRunner {
    inner: ProcessRunner,
    seen: Mutex<Vec<Command>>,
}

impl RecordingRunner {
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn new() -> Self {
        ShellGit {
            runner: Box::new(ProcessRunner),
            program: "git".to_string(),
            // Transport commits are tool-authored, and GitHub's convention for that is the
            // `[bot]` suffix (github-actions[bot], dependabot[bot]). The email is the
            // github-actions app's noreply address — the same one auto-release.yml configures —
            // so a CI push renders the bot avatar while the name keeps the pixi-sandbox
            // identity; a locally published transport carries the same authorship instead of
            // silently falling back to the publishing human's git config.
            name: "pixi-sandbox[bot]".to_string(),
            email: "41898282+github-actions[bot]@users.noreply.github.com".to_string(),
            scratch_name: DEFAULT_SCRATCH_NAME.to_string(),
            log: Mutex::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_runner(runner: Box<dyn Runner>) -> Self {
        ShellGit {
            runner,
            ..Self::new()
        }
    }

    /// A `ShellGit` that runs nothing: every call returns the commands it *would* run.
    #[must_use]
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

    /// Run a command that is allowed to fail (a branch that does not exist yet, a server that
    /// refuses a filter). Logged like any other, so `--dry-run` still shows it.
    fn try_run(&self, command: &Command) -> Option<Output> {
        self.log
            .lock()
            .expect("git log lock")
            .push(command.rendered());
        match self.runner.run(command) {
            Ok(output) if output.ok() => Some(output),
            _ => None,
        }
    }

    /// The parent chain a rotating publish commits on top of, or `None` for an orphan.
    ///
    /// Rotation *rebuilds* (design.md §2): the kept snapshots are re-committed here as a fresh
    /// parentless chain, so the pushed history is exactly what this call constructed and the
    /// dropped snapshots are unreferenced. Reusing the fetched commits as parents instead would
    /// drag their whole ancestry along, which is the growth the flag exists to stop.
    fn rebuilt_parent(&self, snapshot: &Snapshot<'_>, scratch: &Path) -> Result<Option<String>> {
        let inherited = snapshot.retained() - 1;
        if inherited == 0 {
            return Ok(None);
        }

        // `--filter=blob:none` keeps a rotation from downloading the payload it is rotating:
        // only commits and trees are needed to re-commit them, and the remote already has every
        // blob. Servers may refuse the filter, so a plain shallow fetch is the fallback, and a
        // branch that does not exist yet simply has nothing to inherit.
        let fetch = |extra: Option<&str>| {
            let mut command = self
                .git(["fetch", "--no-tags"])
                .arg(format!("--depth={inherited}"));
            if let Some(extra) = extra {
                command = command.arg(extra);
            }
            command
                .arg(snapshot.remote)
                .arg(format!("refs/heads/{}", snapshot.branch))
                .env("GIT_DIR", path(scratch))
        };
        if self.try_run(&fetch(Some("--filter=blob:none"))).is_none()
            && self.try_run(&fetch(None)).is_none()
        {
            return Ok(None);
        }

        let listed = self.run_text(
            &self
                .git([
                    "rev-list",
                    &format!("--max-count={inherited}"),
                    "FETCH_HEAD",
                ])
                .env("GIT_DIR", path(scratch)),
        )?;
        // rev-list is newest first; re-commit oldest first so the chain comes out in order.
        let kept: Vec<String> = listed.lines().rev().map(str::to_string).collect();

        let mut parent: Option<String> = None;
        for commit in kept {
            let tree = self.run_text(
                &self
                    .git(["rev-parse", &format!("{commit}^{{tree}}")])
                    .env("GIT_DIR", path(scratch)),
            )?;
            let message = self.run_text(
                &self
                    .git(["log", "-1", "--format=%B", &commit])
                    .env("GIT_DIR", path(scratch)),
            )?;
            parent = Some(self.commit_tree(&tree, parent.as_deref(), &message, scratch)?);
        }
        Ok(parent)
    }

    /// One commit object: parentless unless a rotation gave it a parent.
    fn commit_tree(
        &self,
        tree: &str,
        parent: Option<&str>,
        message: &str,
        scratch: &Path,
    ) -> Result<String> {
        let mut command = self.git(["commit-tree"]).arg(tree);
        if let Some(parent) = parent {
            command = command.args(["-p", parent]);
        }
        self.run_text(
            &command
                .args(["-m", message])
                .env("GIT_DIR", path(scratch))
                .env("GIT_INDEX_FILE", path(&scratch.join("index"))),
        )
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

    // ------------------------------------------------------------------
    // Working-tree primitives for repository automation (xtask `commit-release`, the
    // consumer-checkout operations on the GitProtocol trait below). These are still git, so
    // they live here rather than as bare `Command::new("git")` calls in a consumer crate
    // (D9). Every command carries the caller's `root` as cwd and this instance's identity
    // through the `-c` pair, so an automation commit never depends on the invoking user's
    // git config.
    // ------------------------------------------------------------------

    /// Run a command and return its raw output, for git invocations whose non-zero exit is
    /// an answer rather than an error (`ls-remote --exit-code` reports "no such ref" as 2).
    fn run_raw(&self, command: &Command) -> Result<Output> {
        self.log
            .lock()
            .expect("git log lock")
            .push(command.rendered());
        self.runner.run(command)
    }

    /// Does `tag` already exist on `remote`?
    pub fn remote_tag_exists(&self, remote: &str, tag: &str) -> Result<bool> {
        let command = self
            .git(["ls-remote", "--exit-code", "--tags"])
            .arg(remote)
            .arg(format!("refs/tags/{tag}"));
        let output = self.run_raw(&command)?;
        match output.status {
            0 => Ok(true),
            2 => Ok(false),
            _ => Err(Error::Command {
                command: command.rendered(),
                status: output.status,
                stderr: output.stderr.trim().to_string(),
            }),
        }
    }

    /// Stage exactly the listed files in the working tree at `root`.
    pub fn add_files(&self, root: &Path, files: &[impl AsRef<Path>]) -> Result<()> {
        let mut command = self.git(["add", "--"]).cwd(root);
        for file in files {
            command = command.arg(path(file.as_ref()));
        }
        self.run(&command).map(|_| ())
    }

    /// Files modified in the working tree but not staged — `git diff --name-only`, which is
    /// worktree-against-index and therefore clean of the staged entries themselves. The
    /// near-equivalents are not equivalent: `git status --porcelain` and `git diff-index
    /// HEAD` also report *staged* changes, so run after a `git add` they name every file
    /// just staged and reject every release. Automation that just staged a list and asks
    /// "what is modified and unaccounted for?" wants this question and no other.
    pub fn unstaged_modifications(&self, root: &Path) -> Result<Vec<String>> {
        let text = self.run_text(&self.git(["diff", "--name-only"]).cwd(root))?;
        Ok(text.lines().map(str::to_string).collect())
    }

    /// Report files changed in the working tree, including staged and unstaged entries.
    ///
    /// This is the porcelain form used by release preparation to account for every tracked
    /// modification without invoking Git from the consumer crate.
    pub fn worktree_status_files(&self, root: &Path) -> Result<Vec<String>> {
        let output = self.run(
            &self
                .git(["status", "--porcelain", "--untracked-files=no"])
                .cwd(root),
        )?;
        let text = String::from_utf8(output.stdout).map_err(|_| Error::NonUtf8 {
            command: "git status --porcelain --untracked-files=no".to_string(),
        })?;
        Ok(text
            .trim_end()
            .lines()
            .filter(|line| line.len() > 3)
            .map(|line| line[3..].to_string())
            .collect())
    }

    /// Commit the staged changes under the instance's identity.
    pub fn commit(&self, root: &Path, message: &str) -> Result<()> {
        self.run(&self.git(["commit", "-m"]).arg(message).cwd(root))
            .map(|_| ())
    }

    /// An annotated tag whose message is its own name, the release convention.
    pub fn tag_annotated(&self, root: &Path, tag: &str) -> Result<()> {
        self.run(
            &self
                .git(["tag", "-a"])
                .arg(tag)
                .arg("-m")
                .arg(tag)
                .cwd(root),
        )
        .map(|_| ())
    }

    /// Push one refspec to `remote` from the working tree at `root`.
    pub fn push_refspec(&self, root: &Path, remote: &str, refspec: &str) -> Result<()> {
        self.run(&self.git(["push"]).arg(remote).arg(refspec).cwd(root))
            .map(|_| ())
    }

    /// The unstaged diff of the working tree at `root`, unpaged.
    pub fn worktree_diff(&self, root: &Path) -> Result<String> {
        self.run_text(&self.git(["--no-pager", "diff"]).cwd(root))
    }

    /// Initialise a fresh repository at `root` — the airlock host starts as an empty repo
    /// and only ever fetches into it (task-36 `airlock-fetch`).
    pub fn init_repo(&self, root: &Path) -> Result<()> {
        self.run(&self.git(["init", "-q"]).cwd(root)).map(|_| ())
    }

    /// Register `url` under `name` in the repository at `root`.
    pub fn remote_add(&self, root: &Path, name: &str, url: &str) -> Result<()> {
        self.run(&self.git(["remote", "add"]).arg(name).arg(url).cwd(root))
            .map(|_| ())
    }

    /// Fetch `refspec` from `remote` at depth 1 into the repository at `root` — the shallow
    /// fetch a developer's machine makes, which is the path the airlock proof exercises.
    pub fn fetch_shallow(&self, root: &Path, remote: &str, refspec: &str) -> Result<()> {
        self.run(
            &self
                .git(["fetch", "-q", "--depth", "1"])
                .arg(remote)
                .arg(refspec)
                .cwd(root),
        )
        .map(|_| ())
    }

    /// Check `branch` out as a linked worktree at `target`.
    pub fn worktree_add(
        &self,
        root: &Path,
        target: &Path,
        branch: &str,
        force: bool,
    ) -> Result<()> {
        let mut command = self.git(["worktree", "add", "-q"]);
        if force {
            command = command.arg("--force");
        }
        self.run(&command.arg(path(target)).arg(branch).cwd(root))
            .map(|_| ())
    }

    /// `--stat` form of [`Self::worktree_diff`], for summaries.
    pub fn worktree_diff_stat(&self, root: &Path) -> Result<String> {
        self.run_text(&self.git(["--no-pager", "diff", "--stat"]).cwd(root))
    }

    /// Is `root` a checked-out work tree, i.e. is there an index and a set of ignore rules to ask?
    ///
    /// `false` is only ever the answer git itself gives ("not a git repository", exit 128); a
    /// `git` that cannot run, or a work tree git refuses to answer for, is an error rather than
    /// a silent `false`, because the caller's fallback is the conservative one and must not be
    /// reached by accident.
    pub fn is_work_tree(&self, root: &Path) -> Result<bool> {
        let command = self.git(["rev-parse", "--is-inside-work-tree"]).cwd(root);
        let output = self.run_raw(&command)?;
        match output.status {
            // A bare repository answers `false` with exit 0, so the answer is read, not assumed.
            0 => Ok(String::from_utf8_lossy(&output.stdout).trim() == "true"),
            128 => Ok(false),
            _ => Err(Error::Command {
                command: command.rendered(),
                status: output.status,
                stderr: output.stderr.trim().to_string(),
            }),
        }
    }

    /// Clone `from` into `to`, sharing no objects with it.
    ///
    /// `--no-hardlinks` is the point: a plain local `git clone` hardlinks the source's object
    /// store, so the copy is still *of* that directory in a way a consumer's `git clone <url>`
    /// never is. A proof that runs in the clone is otherwise partly a proof about the bytes the
    /// tooling left lying next door.
    pub fn clone_fresh(&self, from: &Path, to: &Path) -> Result<()> {
        self.run(
            &self
                .git(["clone", "-q", "--no-hardlinks"])
                .arg(path(from))
                .arg(path(to)),
        )
        .map(|_| ())
    }

    /// Every path `git add -A` would put in the next commit: what the index already holds, plus
    /// what is untracked and *not* ignored.
    ///
    /// This is the question "what would we publish?" answers to, where the near-equivalents do
    /// not: `git status --porcelain` omits tracked-and-unmodified files, so a runtime directory
    /// already committed by an earlier revision would go unreported, and `git ls-files` alone
    /// omits the new file nobody has staged yet. `-z` keeps a path with a space intact, and
    /// paths come back relative to the repository root — which `cwd` is.
    pub fn ls_publishable(&self, root: &Path) -> Result<Vec<String>> {
        let command = self
            .git([
                "ls-files",
                "--cached",
                "--others",
                "--exclude-standard",
                "-z",
            ])
            .cwd(root);
        let output = self.run(&command)?;
        let text = String::from_utf8(output.stdout).map_err(|_| Error::NonUtf8 {
            command: command.rendered(),
        })?;
        Ok(text
            .split('\0')
            .filter(|line| !line.is_empty())
            .map(ToString::to_string)
            .collect())
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
        // `.pixi-sandbox-publish-<pid>-<counter>/` into the published branch. The counter
        // matters because cargo's default test runner publishes several snapshots in parallel.
        // Removed by the guard even if a later step fails.
        let scratch_parent = snapshot.dir.parent().unwrap_or(snapshot.dir);
        let scratch = scratch_parent.join(format!(
            "{}-{}-{}",
            self.scratch_name,
            std::process::id(),
            SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
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
        // No parent by default: the commit is parentless, so the branch is an orphan by
        // construction. `--keep N` gives it the rebuilt chain of the N-1 kept snapshots.
        let parent = self.rebuilt_parent(snapshot, &scratch)?;
        let commit = self.commit_tree(&tree, parent.as_deref(), snapshot.message, &scratch)?;
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

    fn commit_files(&self, commit: &FileCommit<'_>) -> Result<FileCommitted> {
        let root = commit.work_tree;
        // Create or reset the branch at the checkout's current HEAD — a re-proposal onto a
        // version-derived name starts from the same main commit, never from the last attempt.
        self.run(&self.git(["checkout", "-B"]).arg(commit.branch).cwd(root))?;
        // Stage exactly the listed files; nothing else in the tree is ever staged.
        self.add_files(root, commit.files)?;
        // `git diff --cached --quiet` answers "are there staged changes" with its exit code:
        // 0 means the files matched HEAD and there is nothing to propose.
        let staged = self.run_raw(&self.git(["diff", "--cached", "--quiet"]).cwd(root))?;
        match staged.status {
            0 => {
                return Ok(FileCommitted {
                    commit: String::new(),
                    changed: false,
                    patch: Vec::new(),
                });
            }
            1 => {}
            status => {
                return Err(Error::Command {
                    command: "git diff --cached --quiet".to_string(),
                    status,
                    stderr: staged.stderr.trim().to_string(),
                });
            }
        }
        self.commit(root, commit.message)?;
        let sha = self.run_text(&self.git(["rev-parse", "HEAD"]).cwd(root))?;
        // The patch artifact is the commit's binary diff against its parent — raw bytes,
        // because `--binary` output is not UTF-8.
        let patch = self
            .run(&self.git(["diff", "HEAD^", "HEAD", "--binary"]).cwd(root))?
            .stdout;
        Ok(FileCommitted {
            commit: sha,
            changed: true,
            patch,
        })
    }

    fn push_branch(&self, work_tree: &Path, remote: &str, branch: &str, force: bool) -> Result<()> {
        let mut command = self.git(["push"]).cwd(work_tree);
        if force {
            command = command.arg("--force");
        }
        let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
        let command = command.arg(remote).arg(&refspec);
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
