//! Persistent per-user registration of a restored transport's Pixi tools (task-33).
//!
//! `restore` verifies the branch before writing a byte and checks the tree it produced
//! against the manifest's per-file oracle (D13) before this module runs; registration is the
//! last step of a successful restore. It puts *launchers* — small managed scripts that `exec`
//! the manifest-verified tool copies under `<project>/.pixi/tools/<platform>/` — into a
//! per-user bin directory, and adds that directory to the shell's persistent PATH, so `pixi`
//! and `pixi sandbox` work in a fresh shell without sourcing `.pixi/sandbox-env.sh`.
//!
//! Why launchers and not copies or symlinks: a copy is an untracked binary that drifts from
//! the manifest's verified bytes; a symlink is the same idea with an inode indirection, and on
//! Windows creating one needs developer mode or administrator rights — the one thing the
//! user-level mechanism must not require (task-33 AC#2). A text launcher is recognisable by
//! its marker line, safely retargetable by rewriting, and needs no privileges anywhere.
//!
//! Everything here is written against *explicit* roots: the caller resolves HOME, the bin
//! directory and the profile file, so no code path in this module reads the environment and
//! tests never touch a developer's real home (D10's rule, applied to the user level).

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// Recognition marker for everything this module writes. A file carrying this line belongs to
/// pixi-sandbox and may be rewritten (retargeted) by a later restore; anything else found
/// where we want to write is the user's file and is refused unless `--force` says otherwise.
pub const MANAGED_MARKER: &str = "managed by pixi-sandbox";

/// The tools worth a user-level launcher. `pixi` is the entry point; `pixi-sandbox` is how
/// `pixi sandbox` resolves, because pixi discovers `pixi-<command>` binaries on `PATH` — so
/// both must be registered for `pixi sandbox` to work. The unpacker is restore machinery, not
/// a user command, and stays unregistered.
pub const REGISTERED_TOOLS: [&str; 2] = ["pixi", "pixi-sandbox"];

/// Profile markers for the managed PATH block. The block is replaced as a unit, which is what
/// makes the edit idempotent and what lets a changed bin directory retarget cleanly.
const PATH_BLOCK_BEGIN: &str = "# >>> pixi-sandbox user tools (managed block) >>>";
const PATH_BLOCK_END: &str = "# <<< pixi-sandbox user tools (managed block) <<<";

/// The launcher flavour for the host that will run it. Chosen from the restoring host at the
/// single call site (`cfg!(windows)`), and a parameter everywhere else so both flavours are
/// testable on every platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LauncherKind {
    Posix,
    Windows,
}

impl LauncherKind {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Posix
        }
    }
}

/// What happened to one launcher, so the caller can say it precisely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherChange {
    Created,
    /// Rewritten to point at this restore's tools; the previous target is kept for the message
    /// because "the most recently registered restore becomes the user-level tool source" is a
    /// documented semantic, not something to do silently.
    Retargeted {
        from: PathBuf,
    },
    AlreadyCurrent,
}

/// What happened to the persistent PATH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathChange {
    /// POSIX: the managed block was appended to this profile file.
    Added { profile: PathBuf },
    /// POSIX: a managed block existed and was replaced (the bin directory changed).
    Replaced { profile: PathBuf },
    /// POSIX: the block is already there, byte for byte.
    AlreadyPresent { profile: PathBuf },
    /// Windows: the HKCU `Path` value gained the bin directory (or already had it). Only the
    /// Windows PATH updater constructs these; they stay in the one enum so callers match a
    /// single type on every platform.
    #[cfg_attr(not(windows), allow(dead_code))]
    RegistryAdded,
    #[cfg_attr(not(windows), allow(dead_code))]
    RegistryAlreadyPresent,
}

/// The registration service. Every root is explicit — this is the whole point of the design:
/// the CLI resolves HOME/SHELL/`--user-bin` once, tests pass tempdirs, and the code below
/// cannot accidentally reach the real user home.
pub struct UserTools<'a> {
    /// Per-user bin directory that receives the launchers, e.g. `~/.local/bin`.
    pub bin_dir: &'a Path,
    /// POSIX profile file receiving the managed PATH block; `None` on Windows, where the
    /// user PATH lives in the registry instead.
    pub profile: Option<&'a Path>,
    pub force: bool,
}

impl UserTools<'_> {
    /// Write the launchers and update the persistent PATH. Returns what changed; printing is
    /// the caller's job so `restore` keeps one voice.
    pub fn register(
        &self,
        kind: LauncherKind,
        tools: &[(String, PathBuf)],
    ) -> Result<(Vec<(String, LauncherChange)>, PathChange)> {
        fs::create_dir_all(self.bin_dir)
            .with_context(|| format!("creating user bin directory {}", self.bin_dir.display()))?;

        let mut launchers = Vec::new();
        for (name, target) in tools {
            launchers.push((
                name.clone(),
                install_launcher(kind, self.bin_dir, name, target, self.force)?,
            ));
        }

        #[cfg(unix)]
        let path = self.update_posix_profile()?;
        #[cfg(windows)]
        let path = self.update_windows_user_path()?;

        Ok((launchers, path))
    }

    /// POSIX: an idempotent managed block in the detected shell's profile.
    #[cfg(unix)]
    fn update_posix_profile(&self) -> Result<PathChange> {
        let profile = self.profile.ok_or_else(|| {
            anyhow::anyhow!("internal error: POSIX registration without a profile file")
        })?;
        update_profile_path(profile, self.bin_dir)
    }

    /// Windows: the user-level `Path` in the registry, no administrator rights needed.
    #[cfg(windows)]
    fn update_windows_user_path(&self) -> Result<PathChange> {
        let script = powershell_user_path_script(self.bin_dir)?;
        let mut command = std::process::Command::new("powershell");
        command
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
            ])
            .arg("-Command")
            .arg(&script);
        let output = command
            .output()
            .with_context(|| "starting powershell to update the user PATH")?;
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() {
            bail!(
                "updating the Windows user PATH failed ({}):\n{}",
                output.status,
                text.trim()
            );
        }
        if text.contains(PIXI_SANDBOX_PATH_PRESENT) {
            Ok(PathChange::RegistryAlreadyPresent)
        } else {
            Ok(PathChange::RegistryAdded)
        }
    }
}

/// The single launcher file name for a tool on this launcher flavour.
pub fn launcher_file_name(name: &str, kind: LauncherKind) -> String {
    match kind {
        LauncherKind::Posix => name.to_string(),
        // `.cmd` rather than `.exe`: a batch file runs from any shell (cmd, PowerShell, Run)
        // without execution-policy questions, and creating it needs no privileges.
        LauncherKind::Windows => format!("{name}.cmd"),
    }
}

fn launcher_content(kind: LauncherKind, target: &Path) -> Result<String> {
    let target = target
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("tool path is not valid UTF-8: {}", target.display()))?;
    Ok(match kind {
        LauncherKind::Posix => format!(
            "#!/bin/sh\n\
             # {MANAGED_MARKER} - regenerated by every restore; safe to delete\n\
             # target: {target}\n\
             exec \"{target}\" \"$@\"\n"
        ),
        LauncherKind::Windows => format!(
            "@echo off\r\n\
             rem {MANAGED_MARKER} - regenerated by every restore; safe to delete\r\n\
             rem target: {target}\r\n\
             \"{target}\" %*\r\n"
        ),
    })
}

/// Write one launcher, refusing anything we do not own. The rules, in order:
///
/// 1. Nothing there → create it.
/// 2. A managed launcher (marker line) → rewrite it, retargeting to this restore's tools.
/// 3. The identical managed launcher → no change (an idempotent repeat of the same restore).
/// 4. Anything else — a real user file, e.g. a `pixi global install` trampoline or a
///    hand-written wrapper — → refuse unless `--force`, naming the remedies.
fn install_launcher(
    kind: LauncherKind,
    bin_dir: &Path,
    name: &str,
    target: &Path,
    force: bool,
) -> Result<LauncherChange> {
    let path = bin_dir.join(launcher_file_name(name, kind));
    if path.is_dir() {
        bail!(
            "{} is a directory — remove it before registering user tools",
            path.display()
        );
    }
    let content = launcher_content(kind, target)?;
    match fs::read_to_string(&path) {
        Ok(existing) => {
            if !existing.contains(MANAGED_MARKER) && !force {
                bail!(
                    "{} already exists and is not managed by pixi-sandbox\n\
                     the restore itself is complete; to get user tools either:\n\
                     - remove that file and restore again,\n\
                     - pass --force to replace it, or\n\
                     - pass --user-tools skip to leave your setup untouched",
                    path.display()
                );
            }
            if existing == content {
                return Ok(LauncherChange::AlreadyCurrent);
            }
            let from = recorded_target(&existing).unwrap_or_else(|| path.clone());
            fs::write(&path, &content)
                .with_context(|| format!("retargeting {}", path.display()))?;
            make_executable(&path)?;
            Ok(LauncherChange::Retargeted { from })
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
            make_executable(&path)?;
            Ok(LauncherChange::Created)
        }
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

/// The previous target of a managed launcher, for the retarget message.
fn recorded_target(launcher: &str) -> Option<PathBuf> {
    launcher
        .lines()
        .find_map(|line| {
            line.strip_prefix("# target: ")
                .or_else(|| line.strip_prefix("rem target: "))
        })
        .map(PathBuf::from)
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)
            .with_context(|| format!("reading permissions for {}", path.display()))?
            .permissions();
        permissions.set_mode(permissions.mode() | 0o111);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("making {} executable", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// The managed PATH block for POSIX profiles. `case`-guarded so sourcing it twice does not
/// duplicate the entry — the same idempotency `scripts/restore.sh` relies on for its own PATH
/// work, spelled the portable way so bash, zsh, dash and busybox sh all agree.
pub fn path_block(bin_dir: &Path) -> Result<String> {
    let dir = bin_dir.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "user bin directory is not valid UTF-8: {}",
            bin_dir.display()
        )
    })?;
    // The block is spliced into shell syntax inside double quotes and a `case` pattern, so a
    // bin directory carrying characters that change meaning there is refused rather than
    // escaped into something nobody can read back. Spaces are fine; quotes, dollars, globs
    // and backslashes are not.
    if dir
        .chars()
        .any(|c| matches!(c, '"' | '$' | '`' | '\\' | '*' | '?' | '[' | ']') || c.is_control())
    {
        bail!(
            "user bin directory {dir:?} contains characters the profile PATH block cannot \
             quote safely; choose a plain path"
        );
    }
    Ok(format!(
        "{PATH_BLOCK_BEGIN}\n\
         case \":$PATH:\" in\n\
         \x20 *\":{dir}:\"*) ;;\n\
         \x20 *) export PATH=\"{dir}:$PATH\" ;;\n\
         esac\n\
         {PATH_BLOCK_END}\n"
    ))
}

/// Add (or replace) the managed PATH block in a POSIX profile file. All existing managed
/// blocks are removed first — however many there are, from whichever bin directory — and one
/// fresh block is appended, so the edit is idempotent and a changed `--user-bin` retargets
/// cleanly instead of stacking.
pub fn update_profile_path(profile: &Path, bin_dir: &Path) -> Result<PathChange> {
    let block = path_block(bin_dir)?;
    let existing = fs::read_to_string(profile).unwrap_or_default();
    let stripped = strip_path_blocks(&existing);
    if stripped == existing {
        let mut text = existing;
        if !text.is_empty() && !text.ends_with('\n') {
            text.push('\n');
        }
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(&block);
        write_profile(profile, &text)?;
        return Ok(PathChange::Added {
            profile: profile.to_path_buf(),
        });
    }
    let mut text = stripped;
    // The same append the `Added` path performs, so an unchanged block round-trips byte for
    // byte and a repeat run reports `AlreadyPresent` instead of rewriting the file.
    if !text.is_empty() {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push('\n');
    }
    text.push_str(&block);
    if text == existing {
        return Ok(PathChange::AlreadyPresent {
            profile: profile.to_path_buf(),
        });
    }
    write_profile(profile, &text)?;
    Ok(PathChange::Replaced {
        profile: profile.to_path_buf(),
    })
}

fn write_profile(profile: &Path, text: &str) -> Result<()> {
    if let Some(parent) = profile.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(profile, text).with_context(|| format!("writing {}", profile.display()))
}

/// Remove every managed block (markers inclusive) plus the blank separator line this module
/// writes above one. Text outside the blocks is preserved byte for byte.
fn strip_path_blocks(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == PATH_BLOCK_BEGIN {
            // Skip to the end marker; tolerate a truncated block (no end marker) by stopping
            // at the end of the file — a half-deleted block is still ours.
            for inner in lines.by_ref() {
                if inner.trim() == PATH_BLOCK_END {
                    break;
                }
            }
            // Drop the separator blank line that preceded the block, if any.
            if out.ends_with("\n\n") {
                out.pop();
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    // `lines()` drops a trailing newline distinction that matters for idempotency: a file
    // ending without one round-trips with one added. Re-attach exactly what was there.
    if !text.is_empty() && !text.ends_with('\n') && out.ends_with('\n') {
        out.pop();
    }
    out
}

/// The sentinel the Windows PATH script prints when the bin directory was already present, so
/// the caller can report "already there" rather than claiming a change.
#[cfg_attr(not(any(windows, test)), allow(dead_code))]
const PIXI_SANDBOX_PATH_PRESENT: &str = "pixi-sandbox-user-path-present";

/// The PowerShell that adds the bin directory to the *user* PATH (HKCU\Environment), no
/// administrator rights involved. Built as a pure function so its shape is testable on every
/// platform even though it only ever runs on Windows.
///
/// The raw value is read with `DoNotExpandEnvironmentNames` first: a user PATH that says
/// `%USERPROFILE%\bin` must keep saying that. Writing the *expanded* form back — what the
/// plain Get/SetEnvironmentVariable dance does — would bake every reference open, so when
/// the raw value carries `%…%` the script refuses and says so instead of guessing.
#[cfg(any(windows, test))]
pub fn powershell_user_path_script(bin_dir: &Path) -> Result<String> {
    let dir = bin_dir.to_str().ok_or_else(|| {
        anyhow::anyhow!(
            "user bin directory is not valid UTF-8: {}",
            bin_dir.display()
        )
    })?;
    if dir.contains('\'') || dir.chars().any(char::is_control) {
        bail!("user bin directory {dir:?} cannot be quoted safely for the Windows user PATH");
    }
    Ok(format!(
        "$ErrorActionPreference = 'Stop'\n\
         $d = '{dir}'\n\
         $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Environment')\n\
         $raw = [string]$key.GetValue('Path', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)\n\
         $key.Close()\n\
         if ($raw -like '*%*') {{ Write-Error 'the user PATH contains %...% references pixi-sandbox will not expand; add the directory yourself'; exit 1 }}\n\
         $p = [Environment]::GetEnvironmentVariable('Path', 'User')\n\
         $parts = @($p -split ';' | Where-Object {{ $_ -ne '' }})\n\
         if ($parts -contains $d) {{ '{PIXI_SANDBOX_PATH_PRESENT}'; exit 0 }}\n\
         [Environment]::SetEnvironmentVariable('Path', (($parts + $d) -join ';'), 'User')\n"
    ))
}

/// Where the per-user bin directory lives by default. POSIX uses the de-facto standard
/// `~/.local/bin`; Windows gets its own `~\.pixi-sandbox\bin` because `~\.pixi\bin` belongs
/// to `pixi global install` and its trampolines (D4) — colliding with it would be confusing
/// by design, and the whole point is a recognisable, pixi-sandbox-owned location.
pub fn default_bin_dir(home: &Path) -> PathBuf {
    if cfg!(windows) {
        home.join(".pixi-sandbox").join("bin")
    } else {
        home.join(".local").join("bin")
    }
}

/// The profile file the detected shell actually reads, with a notice when the choice needs a
/// human caveat (fish cannot source POSIX profiles; an undetectable shell falls back to the
/// POSIX default). `bash` prefers `.bash_profile` when it exists because a login bash reads
/// it *instead of* `.profile`; `zsh` uses `.zshrc` because that is the one file every
/// interactive zsh reads, login or not.
pub fn detect_profile(home: &Path, shell: Option<&std::ffi::OsStr>) -> (PathBuf, Option<String>) {
    let basename = shell
        .map(Path::new)
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_string();
    match basename.as_str() {
        "bash" => {
            let bash_profile = home.join(".bash_profile");
            if bash_profile.is_file() {
                (bash_profile, None)
            } else {
                (home.join(".profile"), None)
            }
        }
        "zsh" => (home.join(".zshrc"), None),
        "fish" => (
            home.join(".profile"),
            Some(
                "fish does not source POSIX profiles; run `fish_add_path <bin-dir>` once yourself"
                    .to_string(),
            ),
        ),
        other => {
            let notice = if other.is_empty() {
                Some("SHELL is not set; the PATH block was written to ~/.profile".to_string())
            } else {
                Some(format!(
                    "unknown shell {other:?}; the PATH block was written to ~/.profile"
                ))
            };
            (home.join(".profile"), notice)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tool(name: &str) -> (String, PathBuf) {
        (
            name.to_string(),
            PathBuf::from("/project/.pixi/tools/linux-64").join(name),
        )
    }

    #[test]
    fn launchers_exec_the_verified_tool_and_carry_the_marker() {
        let posix = launcher_content(
            LauncherKind::Posix,
            Path::new("/p/.pixi/tools/linux-64/pixi"),
        )
        .unwrap();
        assert!(posix.contains(MANAGED_MARKER));
        assert!(posix.contains("exec \"/p/.pixi/tools/linux-64/pixi\" \"$@\""));
        assert!(posix.contains("# target: /p/.pixi/tools/linux-64/pixi"));

        let windows = launcher_content(
            LauncherKind::Windows,
            Path::new(r"C:\p\.pixi\tools\win-64\pixi.exe"),
        )
        .unwrap();
        assert!(windows.contains(MANAGED_MARKER));
        assert!(windows.contains(r#""C:\p\.pixi\tools\win-64\pixi.exe" %*"#));
    }

    #[test]
    fn first_registration_creates_launchers_and_adds_the_profile_block() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join(".local/bin");
        let profile = temp.path().join(".profile");
        let tools = UserTools {
            bin_dir: &bin,
            profile: Some(&profile),
            force: false,
        };

        let (launchers, path) = tools
            .register(LauncherKind::Posix, &[tool("pixi"), tool("pixi-sandbox")])
            .unwrap();
        assert_eq!(
            launchers
                .iter()
                .map(|(name, change)| (name.as_str(), change))
                .collect::<Vec<_>>(),
            vec![
                ("pixi", &LauncherChange::Created),
                ("pixi-sandbox", &LauncherChange::Created),
            ]
        );
        assert_eq!(
            path,
            PathChange::Added {
                profile: profile.clone()
            }
        );
        assert!(
            fs::read_to_string(bin.join("pixi"))
                .unwrap()
                .contains(MANAGED_MARKER)
        );
        let block = fs::read_to_string(&profile).unwrap();
        assert!(block.contains("export PATH="));
        assert!(block.contains(bin.to_str().unwrap()));
        assert_eq!(block.matches(PATH_BLOCK_BEGIN).count(), 1);
    }

    #[test]
    fn repeat_registration_is_idempotent() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        let profile = temp.path().join(".profile");
        {
            let tools = UserTools {
                bin_dir: &bin,
                profile: Some(&profile),
                force: false,
            };
            tools
                .register(LauncherKind::Posix, &[tool("pixi")])
                .unwrap();
        }
        let before = fs::read_to_string(bin.join("pixi")).unwrap();
        let profile_before = fs::read_to_string(&profile).unwrap();

        let tools = UserTools {
            bin_dir: &bin,
            profile: Some(&profile),
            force: false,
        };
        let (launchers, path) = tools
            .register(LauncherKind::Posix, &[tool("pixi")])
            .unwrap();
        assert_eq!(launchers[0].1, LauncherChange::AlreadyCurrent);
        assert_eq!(
            path,
            PathChange::AlreadyPresent {
                profile: profile.clone()
            }
        );
        assert_eq!(fs::read_to_string(bin.join("pixi")).unwrap(), before);
        assert_eq!(fs::read_to_string(&profile).unwrap(), profile_before);
    }

    #[test]
    fn re_restoring_another_project_retargets_only_managed_entries() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        let profile = temp.path().join(".profile");
        {
            let tools = UserTools {
                bin_dir: &bin,
                profile: Some(&profile),
                force: false,
            };
            tools
                .register(
                    LauncherKind::Posix,
                    &[(
                        "pixi".to_string(),
                        PathBuf::from("/old/project/.pixi/tools/linux-64/pixi"),
                    )],
                )
                .unwrap();
        }
        // A user file in the same bin directory is none of our business.
        fs::write(bin.join("user-tool"), "#!/bin/sh\n").unwrap();

        let tools = UserTools {
            bin_dir: &bin,
            profile: Some(&profile),
            force: false,
        };
        let (launchers, _path) = tools
            .register(
                LauncherKind::Posix,
                &[(
                    "pixi".to_string(),
                    PathBuf::from("/new/project/.pixi/tools/linux-64/pixi"),
                )],
            )
            .unwrap();
        assert_eq!(
            launchers[0].1,
            LauncherChange::Retargeted {
                from: PathBuf::from("/old/project/.pixi/tools/linux-64/pixi")
            }
        );
        let launcher = fs::read_to_string(bin.join("pixi")).unwrap();
        assert!(launcher.contains("exec \"/new/project/.pixi/tools/linux-64/pixi\""));
        // The unrelated file is untouched, byte for byte.
        assert_eq!(
            fs::read_to_string(bin.join("user-tool")).unwrap(),
            "#!/bin/sh\n"
        );
        // One PATH block, not two.
        assert_eq!(
            fs::read_to_string(&profile)
                .unwrap()
                .matches(PATH_BLOCK_BEGIN)
                .count(),
            1
        );
    }

    #[test]
    fn an_unmanaged_collision_is_refused_unless_forced() {
        let temp = tempfile::tempdir().unwrap();
        let bin = temp.path().join("bin");
        let profile = temp.path().join(".profile");
        fs::create_dir_all(&bin).unwrap();
        // A `pixi global install` trampoline: real, user-owned, no marker.
        fs::write(bin.join("pixi"), "#!/bin/sh\nexec /somewhere/pixi \"$@\"\n").unwrap();

        let tools = UserTools {
            bin_dir: &bin,
            profile: Some(&profile),
            force: false,
        };
        let error = tools
            .register(LauncherKind::Posix, &[tool("pixi")])
            .unwrap_err()
            .to_string();
        assert!(error.contains("is not managed by pixi-sandbox"), "{error}");
        assert!(error.contains("--user-tools skip"), "{error}");
        // The refusal wrote nothing: the profile has no block and the file is unchanged.
        assert!(!profile.exists());
        assert_eq!(
            fs::read_to_string(bin.join("pixi")).unwrap(),
            "#!/bin/sh\nexec /somewhere/pixi \"$@\"\n"
        );

        let forced = UserTools {
            bin_dir: &bin,
            profile: Some(&profile),
            force: true,
        };
        forced
            .register(LauncherKind::Posix, &[tool("pixi")])
            .unwrap();
        assert!(
            fs::read_to_string(bin.join("pixi"))
                .unwrap()
                .contains(MANAGED_MARKER)
        );
    }

    #[test]
    fn a_changed_bin_directory_replaces_the_block_and_keeps_user_text() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join(".profile");
        fs::write(&profile, "export EDITOR=vi\n").unwrap();
        let old_bin = temp.path().join("old-bin");
        let new_bin = temp.path().join("new-bin");

        update_profile_path(&profile, &old_bin).unwrap();
        assert_eq!(
            update_profile_path(&profile, &old_bin).unwrap(),
            PathChange::AlreadyPresent {
                profile: profile.clone()
            }
        );
        assert_eq!(
            update_profile_path(&profile, &new_bin).unwrap(),
            PathChange::Replaced {
                profile: profile.clone()
            }
        );

        let text = fs::read_to_string(&profile).unwrap();
        assert!(
            text.starts_with("export EDITOR=vi\n"),
            "user text survives: {text}"
        );
        assert_eq!(text.matches(PATH_BLOCK_BEGIN).count(), 1);
        assert!(text.contains(new_bin.to_str().unwrap()));
        assert!(!text.contains(old_bin.to_str().unwrap()));
    }

    #[test]
    fn stale_and_duplicated_blocks_collapse_into_one() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join(".profile");
        let bin = temp.path().join("bin");
        // Two blocks, as if an old version stacked them; one is even truncated garbage.
        fs::write(
            &profile,
            format!(
                "# header\n{PATH_BLOCK_BEGIN}\nexport PATH=old\n{PATH_BLOCK_END}\n# middle\n{PATH_BLOCK_BEGIN}\nexport PATH=older\n"
            ),
        )
        .unwrap();

        update_profile_path(&profile, &bin).unwrap();
        let text = fs::read_to_string(&profile).unwrap();
        assert!(text.contains("# header\n"));
        assert!(text.contains("# middle"));
        assert_eq!(text.matches(PATH_BLOCK_BEGIN).count(), 1);
        assert!(text.contains(bin.to_str().unwrap()));
    }

    #[test]
    fn a_profile_that_does_not_exist_yet_is_created() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join(".bash_profile");
        let bin = temp.path().join("bin");
        assert_eq!(
            update_profile_path(&profile, &bin).unwrap(),
            PathChange::Added {
                profile: profile.clone()
            }
        );
        assert!(profile.is_file());
    }

    #[test]
    fn unsafe_bin_directories_are_refused_rather_than_misquoted() {
        let temp = tempfile::tempdir().unwrap();
        let profile = temp.path().join(".profile");
        for weird in ["has\"quote", "has$dollar", "has*glob", "has\ttab"] {
            let error = update_profile_path(&profile, Path::new(weird))
                .unwrap_err()
                .to_string();
            assert!(error.contains("cannot quote safely"), "{weird}: {error}");
        }
        // A space is fine and survives round-tripping.
        let spaced = temp.path().join("my tools");
        update_profile_path(&profile, &spaced).unwrap();
        assert!(fs::read_to_string(&profile).unwrap().contains("my tools"));
    }

    #[test]
    fn profile_detection_follows_the_detected_shell() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();

        let (bash_profile, notice) =
            detect_profile(home, Some(std::ffi::OsStr::new("/usr/bin/bash")));
        assert_eq!(bash_profile, home.join(".profile"));
        assert!(notice.is_none());

        fs::write(home.join(".bash_profile"), "# login shell config\n").unwrap();
        let (bash_profile, notice) =
            detect_profile(home, Some(std::ffi::OsStr::new("/usr/bin/bash")));
        assert_eq!(bash_profile, home.join(".bash_profile"));
        assert!(notice.is_none());

        let (zshrc, notice) =
            detect_profile(home, Some(std::ffi::OsStr::new("/opt/homebrew/bin/zsh")));
        assert_eq!(zshrc, home.join(".zshrc"));
        assert!(notice.is_none());

        let (fish_profile, notice) =
            detect_profile(home, Some(std::ffi::OsStr::new("/usr/bin/fish")));
        assert_eq!(fish_profile, home.join(".profile"));
        assert!(notice.unwrap().contains("fish_add_path"));

        let (fallback, notice) = detect_profile(home, None);
        assert_eq!(fallback, home.join(".profile"));
        assert!(notice.unwrap().contains("SHELL is not set"));
    }

    #[test]
    fn windows_launchers_take_the_cmd_suffix_and_the_script_quotes_the_directory() {
        assert_eq!(
            launcher_file_name("pixi", LauncherKind::Windows),
            "pixi.cmd"
        );
        assert_eq!(launcher_file_name("pixi", LauncherKind::Posix), "pixi");

        let script =
            powershell_user_path_script(Path::new(r"C:\Users\me\.pixi-sandbox\bin")).unwrap();
        assert!(script.contains(r"$d = 'C:\Users\me\.pixi-sandbox\bin'"));
        assert!(script.contains("DoNotExpandEnvironmentNames"));
        assert!(script.contains("SetEnvironmentVariable('Path'"));
        // The refusal path for %...% references is part of the script, not an afterthought.
        assert!(script.contains("-like '*%*'"));

        assert!(powershell_user_path_script(Path::new("C:\\it's")).is_err());
    }

    #[test]
    fn the_path_block_is_idempotent_when_sourced_twice() {
        let block = path_block(Path::new("/home/me/.local/bin")).unwrap();
        let mut script = String::new();
        for _ in 0..2 {
            script.push_str(&block);
        }
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("profile"), &script).unwrap();
        // Sourcing the block twice must leave exactly one occurrence of the directory on PATH.
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "PATH=/usr/bin:/bin; . {}; case \":$PATH:\" in \\\n\
                 *\":/home/me/.local/bin:/home/me/.local/bin:\"*) echo DOUBLED ;; \\\n\
                 *) echo OK ;; esac",
                temp.path().join("profile").display()
            ))
            .output()
            .unwrap();
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "OK");
    }
}
