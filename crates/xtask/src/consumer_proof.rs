//! The release lane's consumer proof (TASK-89), as tested Rust instead of workflow shell.
//!
//! The proof downloads the binary a release is about to publish, lets that binary generate a
//! consumer publisher, and then walks the whole lifecycle a consumer would: pack, publish to a
//! throwaway remote, clone it back, and restore offline. Doc-13's slice 5 moved it in front of
//! the publish so a failing proof stops a release instead of annotating one — and moved it
//! verbatim, spending ~60 lines of `run:` block behind four `multiline-run-allowed` opt-outs in
//! `release.yml`. This module is the port of those lines, in the shape
//! [`crate::starter`] already uses for the starter lane: the recipe's decisions live in pure
//! functions, its refusals in their own function, and its tests drive tempdir fixtures.
//!
//! The port changes the spelling, never the claim. Three properties of the shell recipe are
//! load-bearing and are preserved deliberately:
//!
//! * **The isolation levels.** `pixi lock`, `pixi install --frozen` and `pack` run with an
//!   isolated `HOME` but an inherited `PATH`; `restore` and the restored `pixi --version` run
//!   under an *empty* environment (`env -i` with `PATH=/usr/bin:/bin`). That restricted `PATH`
//!   is the offline claim itself — with nothing else on `PATH`, nothing can reach a network.
//!   See [`isolated_env`] and [`empty_env`]. The shell also had a third, looser level: `doctor`
//!   and `publish` ran with no override at all. This port gives them the isolated `HOME` too,
//!   which removes an ambient dependency and so tightens the proof rather than changing it.
//! * **The evidence set.** Ten files are uploaded as the run's artifact, and a proof whose
//!   evidence silently shrank would still be green. [`EVIDENCE_FILES`] names them.
//! * **The publisher assertions.** The two literals the proof greps the generated publisher for
//!   live in [`PUBLISHER_ASSERTIONS`], which is where repo check 13 finds them.
//!
//! What this module cannot do is run the lifecycle locally: `pixi lock` in the consumer fixture
//! resolves against prefix.dev, which an airlocked host cannot reach. Its argv and environment
//! shapes are proven here; executing them end to end is the release-run proof (task-89 AC#3).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// The transport branch the proof publishes into its throwaway remote.
pub const CONSUMER_BRANCH: &str = "sandbox/consumer-proof-linux-64";

/// The consumer's Pixi project, byte for byte as the shell heredoc wrote it.
///
/// The `ripgrep` dependency and its comment are part of the recipe, not decoration: a
/// dependency-free environment solves to no platform-specific lock entry at all, and
/// `pixi-pack` then fails with "platform not found in lockfile: linux-64".
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
pub const CONSUMER_PIXI_TOML: &str = r#"[workspace]
name = "consumer-proof"
channels = ["https://prefix.dev/conda-forge"]
platforms = ["linux-64"]

[dependencies]
# A dependency-free environment solves to no platform-specific lock entry at all
# (pixi omits per-platform data when there is nothing to record), so pixi-pack later
# fails with "platform not found in lockfile: linux-64" and an empty candidate list.
# One small, fast-to-fetch package (same choice as tests/fixtures/demo-project) is
# enough to make the environment real without slowing the proof down.
ripgrep = "*"
"#;

/// Every file the proof writes into its evidence directory, and the run uploads as an artifact.
///
/// Named here so that a step which quietly stops writing one of them fails a test instead of
/// shrinking the artifact behind a green run.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
pub const EVIDENCE_FILES: [&str; 10] = [
    "version.txt",
    "help.txt",
    "publish-sandbox.first.yml",
    "publish-sandbox.yml",
    "pack.log",
    "doctor.log",
    "publish.log",
    "restore.log",
    "restored-pixi-version.txt",
    "manifest.json",
];

/// Write the clean consumer fixture and the empty evidence directory for one proof run.
///
/// Both directories are wiped first, exactly as the shell's `rm -rf` did. That is not
/// tidiness: a `publish-sandbox.yml` left over from an earlier run would let the
/// regenerate-and-compare step compare the previous release against itself and pass.
///
/// # Errors
///
/// Fails when a directory cannot be removed or created, or a fixture file cannot be written.
pub fn fixture(consumer: &Path, evidence: &Path, label: &str) -> Result<()> {
    for dir in [consumer, evidence] {
        if dir.exists() {
            std::fs::remove_dir_all(dir)
                .with_context(|| format!("cannot clear {}", dir.display()))?;
        }
        std::fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    }
    std::fs::create_dir_all(consumer.join(".github/workflows"))
        .context("cannot create the consumer's workflow directory")?;
    // `init` writes into `.pixi`, and the fixture pre-creates it the way the shell did.
    std::fs::create_dir_all(consumer.join(".pixi"))
        .context("cannot create the consumer's .pixi")?;
    std::fs::write(consumer.join("pixi.toml"), CONSUMER_PIXI_TOML)
        .context("cannot write the consumer's pixi.toml")?;
    std::fs::write(
        consumer.join("README.md"),
        format!("consumer fixture: {label}\n"),
    )
    .context("cannot write the consumer's README")?;
    Ok(())
}

/// The properties the proof asserts of the publisher the binary generates.
///
/// ADR-30 / TASK-126: require the release checksum manifest and an actual verification
/// command, not the now-removed conda-installed fetch-release entrypoint. Repo check 13
/// holds these literals against the generator's source; execution tests also corrupt bytes.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
pub const PUBLISHER_ASSERTIONS: [&str; 2] = ["sha256sum --check selected.sha256", "SHA256SUMS"];

/// Report every asserted property the generated publisher's text does not carry.
///
/// Findings rather than a bail, in the shape [`crate::starter::publish_refusals`] uses: one run
/// names everything the generator dropped instead of stopping at the first.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn assert_publisher(rendered: &str) -> Vec<String> {
    PUBLISHER_ASSERTIONS
        .iter()
        .filter(|literal| !rendered.contains(**literal))
        .map(|literal| {
            format!("the generated publisher does not contain {literal:?}, which the proof asserts")
        })
        .collect()
}

/// The `PATH` the offline steps run with: the base system and nothing else.
///
/// This string is the offline claim. With no runner-installed tool reachable, a restore that
/// succeeds cannot have downloaded anything to do it.
pub const OFFLINE_PATH: &str = "/usr/bin:/bin";

/// Environment overrides for the steps that need the runner's own tools.
///
/// `HOME` alone: the recipe isolates the consumer's home so no developer or runner
/// configuration leaks into the proof, while `PATH` stays inherited so `pixi`, `git` and the
/// built binary remain reachable. Applies to `pixi lock`, `pixi install --frozen` and `pack`.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn isolated_env(home: &Path) -> Vec<(&'static str, String)> {
    vec![("HOME", home.display().to_string())]
}

/// Environment overrides for the steps that must not be able to reach a network.
///
/// The `env -i` half of the recipe: an empty environment with the consumer's `HOME` and a
/// base-system `PATH`. Applies to `restore` and to asking the restored `pixi` its version.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn empty_env(home: &Path) -> Vec<(&'static str, String)> {
    vec![
        ("HOME", home.display().to_string()),
        ("PATH", OFFLINE_PATH.to_string()),
    ]
}

/// Install the built binary as the consumer's own CLI, executable, at `dest`.
///
/// This is the recipe's `install -m 0755`: the proof must run the artifact the build produced
/// rather than one already on `PATH`, so the copy and its mode bit are the whole point.
///
/// # Errors
///
/// Fails when the artifact cannot be read (the usual cause being a build whose upload step
/// produced nothing) or the copy cannot be written.
pub fn adopt(artifact: &Path, dest: &Path) -> Result<()> {
    std::fs::copy(artifact, dest)
        .with_context(|| format!("cannot adopt {} as the consumer's CLI", artifact.display()))?;
    make_executable(dest)
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755))
        .with_context(|| format!("cannot make {} executable", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

/// How a step's process environment is built.
///
/// The recipe's two isolation levels, named: `Ambient` keeps the runner's environment and
/// applies the overrides (so `pixi` and `git` stay reachable), `Empty` clears it and applies
/// only the overrides (so nothing outside the base system is reachable at all).
#[derive(Clone, Copy, Debug)]
pub enum Isolation<'a> {
    /// Inherit the environment, then apply `vars`.
    Ambient(&'a [(&'static str, String)]),
    /// Clear the environment, then apply `vars`.
    Empty(&'a [(&'static str, String)]),
}

/// Run `program` and return its stdout, failing on a non-zero exit.
///
/// # Errors
///
/// Fails when the program cannot be spawned or exits non-zero; the message names the program,
/// its arguments and its stderr, because that is all a CI log will have.
pub fn run_captured(
    program: &Path,
    args: &[&str],
    cwd: Option<&Path>,
    isolation: Isolation<'_>,
) -> Result<String> {
    let mut command = std::process::Command::new(program);
    command.args(args);
    if let Some(dir) = cwd {
        command.current_dir(dir);
    }
    match isolation {
        Isolation::Ambient(vars) => {
            command.envs(vars.iter().map(|(k, v)| (*k, v.as_str())));
        }
        Isolation::Empty(vars) => {
            command
                .env_clear()
                .envs(vars.iter().map(|(k, v)| (*k, v.as_str())));
        }
    }
    let rendered = format!("{} {}", program.display(), args.join(" "));
    let output = command
        .output()
        .with_context(|| format!("cannot run {rendered}"))?;
    if !output.status.success() {
        bail!(
            "{rendered} exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_string())
}

/// Write evidence and echo it, the `| tee` half of the recipe: the log keeps the output and the
/// artifact keeps the file.
fn tee_evidence(evidence: &Path, name: &str, text: &str) -> Result<()> {
    println!("{text}");
    write_evidence(evidence, name, text)
}

/// Write evidence without echoing it, the `> file` half of the recipe.
fn write_evidence(evidence: &Path, name: &str, text: &str) -> Result<()> {
    std::fs::write(evidence.join(name), format!("{text}\n"))
        .with_context(|| format!("cannot write the {name} evidence"))
}

/// Record the binary's own `--version` and `--help` as evidence.
///
/// `--version` runs under [`empty_env`]: a released binary that needs the runner's environment
/// to report its own version is not a binary a consumer can run. `--help` runs with the ambient
/// environment, exactly as the shell recipe had it — the asymmetry is the recipe's, and this
/// port does not change what the proof proves.
///
/// # Errors
///
/// Fails when either invocation exits non-zero, or the evidence cannot be written. No evidence
/// file is written for a step that failed, so a red run cannot leave a result behind.
pub fn record_version(binary: &Path, home: &Path, evidence: &Path) -> Result<()> {
    let empty = empty_env(home);
    let version = run_captured(binary, &["--version"], None, Isolation::Empty(&empty))
        .context("the binary cannot report its own version")?;
    tee_evidence(evidence, "version.txt", &version)?;
    let help = run_captured(binary, &["--help"], None, Isolation::Ambient(&[]))
        .context("the binary cannot parse its own arguments")?;
    write_evidence(evidence, "help.txt", &help)
}

/// Where `pixi-sandbox init` writes the consumer's publisher, relative to the project root.
pub const PUBLISHER_REL_PATH: &str = ".github/workflows/publish-sandbox.yml";

/// Let the built binary generate the consumer's publisher, prove the render is reproducible,
/// and prove it carries the properties the proof asserts.
///
/// Rendering twice and comparing is the recipe's point: a publisher whose content depends on
/// when it was generated would give every consumer a different upgrade lane each run.
///
/// One deviation from the shell, which changes the evidence and not the verdict: the second
/// render is copied to the evidence *before* the comparison, where the shell compared and only
/// then copied. A red run therefore leaves both renders behind to diff, instead of the operator
/// having to reproduce the disagreement to see it.
///
/// # Errors
///
/// Fails when either `init` fails, when the two renders differ, or when the rendered publisher
/// is missing an asserted property.
pub fn render_publisher(
    binary: &Path,
    consumer: &Path,
    home: &Path,
    evidence: &Path,
) -> Result<()> {
    let isolated = isolated_env(home);
    let publisher = consumer.join(PUBLISHER_REL_PATH);

    run_captured(
        binary,
        &["init", "--force"],
        Some(consumer),
        Isolation::Ambient(&isolated),
    )
    .context("the built binary cannot generate a consumer publisher")?;
    let first = std::fs::read(&publisher)
        .with_context(|| format!("init produced no {PUBLISHER_REL_PATH}"))?;
    copy_evidence(evidence, "publish-sandbox.first.yml", &publisher)?;

    run_captured(
        binary,
        &["init", "--force"],
        Some(consumer),
        Isolation::Ambient(&isolated),
    )
    .context("the built binary cannot regenerate the consumer publisher")?;
    let second = std::fs::read(&publisher)
        .with_context(|| format!("the second init produced no {PUBLISHER_REL_PATH}"))?;
    copy_evidence(evidence, "publish-sandbox.yml", &publisher)?;

    if first != second {
        bail!(
            "{PUBLISHER_REL_PATH} is not reproducible: the second `init --force` rendered different bytes from the first. Both renders are in the evidence."
        );
    }

    let findings = assert_publisher(&String::from_utf8_lossy(&second));
    if !findings.is_empty() {
        for finding in &findings {
            eprintln!("::error::{finding}");
        }
        // The findings go into the bail too, not only to stderr: `main` reports the error
        // chain as one annotation, and an operator should not have to scroll for the names.
        bail!(
            "the generated publisher is missing {} asserted property/ies: {}",
            findings.len(),
            findings.join("; ")
        );
    }
    Ok(())
}

/// Copy a file into the evidence directory byte for byte, the recipe's `cp`.
fn copy_evidence(evidence: &Path, name: &str, from: &Path) -> Result<()> {
    std::fs::copy(from, evidence.join(name))
        .with_context(|| format!("cannot copy {} into the evidence", from.display()))
        .map(|_| ())
}

/// `pixi-sandbox pack` for one platform, packing the consumer's own tree.
///
/// `--pixi-bin` and `--self-bin` are what make the transport self-contained: the explicitly selected Pixi executable is embedded at pack time and the packing binary travels inside the payload, which is
/// what lets the restore run with no network.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn pack_argv(transport: &Path, platform: &str, self_bin: &Path) -> Vec<String> {
    argv(&[
        "pack",
        "--repo-root",
        ".",
        "--envs",
        "default",
        "--output-dir",
        &display(transport),
        "--platform",
        platform,
        "--pixi-bin",
        "pixi",
        "--self-bin",
        &display(self_bin),
    ])
}

/// `pixi-sandbox doctor --verify` over the packed transport.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn doctor_argv(transport: &Path) -> Vec<String> {
    argv(&[
        "doctor",
        "--branch-location",
        &display(transport),
        "--verify",
    ])
}

/// `pixi-sandbox publish` of the transport into the throwaway remote.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn publish_argv(transport: &Path, remote: &Path) -> Vec<String> {
    argv(&[
        "publish",
        "--input-dir",
        &display(transport),
        "--branch-name",
        CONSUMER_BRANCH,
        "--remote",
        &display(remote),
    ])
}

/// `pixi-sandbox restore` into an existing directory, touching no user tools.
///
/// `--user-tools skip` keeps the restore out of the runner's HOME; `--output-path` must already
/// exist, which is why [`lifecycle`] creates it.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn restore_argv(checkout: &Path, restored: &Path) -> Vec<String> {
    argv(&[
        "restore",
        "--branch-location",
        &display(checkout),
        "--output-path",
        &display(restored),
        "--user-tools",
        "skip",
    ])
}

fn argv(words: &[&str]) -> Vec<String> {
    words.iter().map(|w| (*w).to_string()).collect()
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

/// Every directory the lifecycle owns, in the order the recipe creates them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LifecyclePaths {
    /// The consumer project: fixture, adopted binary and generated publisher.
    pub consumer: PathBuf,
    /// Where `pack` writes the transport.
    pub transport: PathBuf,
    /// The throwaway bare remote the transport is published to.
    pub remote: PathBuf,
    /// A fresh clone of that remote, which is where the restore runs from.
    pub checkout: PathBuf,
    /// The directory the transport is restored into.
    pub restored: PathBuf,
    /// The consumer's isolated `HOME`.
    pub home: PathBuf,
    /// The evidence directory the run uploads.
    pub evidence: PathBuf,
}

/// Every reason the lifecycle cannot start, judged before any tool is invoked.
///
/// The shell enforced this ordering by failing somewhere inside `pack`, three tools after the
/// step that was actually skipped. The verdict is unchanged; the failure simply lands where the
/// mistake was made.
#[doc(hidden)] // test boundary (tests/consumer_proof.rs)
#[must_use]
pub fn lifecycle_refusals(paths: &LifecyclePaths) -> Vec<String> {
    let required = [
        ("pixi.toml", "the consumer fixture was not prepared"),
        (
            "pixi-sandbox",
            "the built binary was not adopted as the consumer's CLI",
        ),
        (
            PUBLISHER_REL_PATH,
            "the consumer publisher was not generated",
        ),
    ];
    required
        .iter()
        .filter(|(rel, _)| !paths.consumer.join(rel).exists())
        .map(|(rel, why)| format!("{why}: {} is missing", paths.consumer.join(rel).display()))
        .collect()
}

/// Run the whole lifecycle: lock, commit, install, pack, doctor, publish, clone, restore.
///
/// The git half goes through [`pixi_sandbox_git::ShellGit`] (invariant 8), whose per-command
/// identity replaces the shell's `git config user.name`/`user.email` pair.
///
/// # Errors
///
/// Fails when a precondition is missing, or when any step of the lifecycle fails. Each step's
/// output is written to the evidence directory before the next one runs.
pub fn lifecycle(paths: &LifecyclePaths, platform: &str) -> Result<()> {
    let refusals = lifecycle_refusals(paths);
    if !refusals.is_empty() {
        for refusal in &refusals {
            eprintln!("::error::{refusal}");
        }
        // Named in the bail as well as on stderr, so the one annotation `main` prints carries
        // the reasons and not just their count.
        bail!(
            "refusing to run the consumer lifecycle: {} precondition(s) unmet: {}",
            refusals.len(),
            refusals.join("; ")
        );
    }

    // `restore` requires `--output-path` to exist already (`support::existing_dir`), which is
    // why the shell `mkdir -p`'d it; the other directories are created by the tools that own
    // them, so pre-creating them would only hide a failure to do so.
    std::fs::create_dir_all(&paths.restored)
        .with_context(|| format!("cannot create {}", paths.restored.display()))?;
    std::fs::create_dir_all(&paths.home)
        .with_context(|| format!("cannot create {}", paths.home.display()))?;

    commit_the_consumer(paths)?;
    prove_the_transport(paths, platform)
}

/// Lock the consumer's environment, commit the fixture, and install it.
///
/// `pixi` comes from the runner's `PATH`, so these run with an inherited `PATH` and an isolated
/// `HOME`; the empty environment belongs to the restore steps alone.
fn commit_the_consumer(paths: &LifecyclePaths) -> Result<()> {
    use pixi_sandbox_git::ShellGit;

    let isolated = isolated_env(&paths.home);
    let ambient = Isolation::Ambient(&isolated);
    let consumer = paths.consumer.as_path();
    let pixi = Path::new("pixi");

    // The per-command identity replaces the shell's `git config user.name`/`user.email` pair.
    let git = ShellGit::new().identity("consumer-proof", "consumer-proof@example.invalid");
    git.init_repo_branch(consumer, "main")?;

    println!(
        "{}",
        run_captured(pixi, &["lock"], Some(consumer), ambient)?
    );
    git.add_files(
        consumer,
        &[
            "README.md",
            "pixi.toml",
            "pixi.lock",
            "pixi-sandbox.toml",
            "restore.sh",
            ".github/workflows",
        ],
    )?;
    git.commit(consumer, "test: consumer fixture")?;
    println!(
        "{}",
        run_captured(pixi, &["install", "--frozen"], Some(consumer), ambient)?
    );
    Ok(())
}

/// Pack the consumer, publish the transport, clone it back and restore it offline.
fn prove_the_transport(paths: &LifecyclePaths, platform: &str) -> Result<()> {
    use pixi_sandbox_git::ShellGit;

    let isolated = isolated_env(&paths.home);
    let empty = empty_env(&paths.home);
    let ambient = Isolation::Ambient(&isolated);
    let offline = Isolation::Empty(&empty);
    let cli = paths.consumer.join("pixi-sandbox");
    let git = ShellGit::new();

    tee_evidence(
        &paths.evidence,
        "pack.log",
        &run_captured(
            &cli,
            &argv_refs(&pack_argv(
                &paths.transport,
                platform,
                Path::new("./pixi-sandbox"),
            )),
            Some(paths.consumer.as_path()),
            ambient,
        )?,
    )?;
    tee_evidence(
        &paths.evidence,
        "doctor.log",
        &run_captured(
            &cli,
            &argv_refs(&doctor_argv(&paths.transport)),
            None,
            ambient,
        )?,
    )?;

    git.init_bare(&paths.remote)?;
    tee_evidence(
        &paths.evidence,
        "publish.log",
        &run_captured(
            &cli,
            &argv_refs(&publish_argv(&paths.transport, &paths.remote)),
            None,
            ambient,
        )?,
    )?;

    git.clone_fresh(&paths.remote, &paths.checkout)?;
    git.fetch_shallow(&paths.checkout, "origin", CONSUMER_BRANCH)?;
    git.checkout_detached(&paths.checkout, "FETCH_HEAD")?;

    // The restore runs the binary that travelled *inside* the transport, under an empty
    // environment: with nothing but the base system reachable, a successful restore cannot have
    // fetched anything.
    let packed_cli = paths
        .checkout
        .join(".pixi-sandbox/tools")
        .join(platform)
        .join("pixi-sandbox");
    tee_evidence(
        &paths.evidence,
        "restore.log",
        &run_captured(
            &packed_cli,
            &argv_refs(&restore_argv(&paths.checkout, &paths.restored)),
            None,
            offline,
        )?,
    )?;
    let restored_pixi = paths
        .restored
        .join(".pixi/tools")
        .join(platform)
        .join("pixi");
    tee_evidence(
        &paths.evidence,
        "restored-pixi-version.txt",
        &run_captured(&restored_pixi, &["--version"], None, offline)?,
    )?;

    copy_evidence(
        &paths.evidence,
        "manifest.json",
        &paths.transport.join(".pixi-sandbox/manifest.json"),
    )
}

/// Borrow a `Vec<String>` argv as the `&[&str]` [`run_captured`] takes.
fn argv_refs(argv: &[String]) -> Vec<&str> {
    argv.iter().map(String::as_str).collect()
}
