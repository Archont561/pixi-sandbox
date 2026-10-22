//! `unpack` — one packed environment to one raw prefix.
//!
//! Unlike `restore`, this command deliberately does not write Pixi's environment markers. It
//! is useful for a pack delivered outside the branch workflow, while `restore` remains the
//! operation that turns a verified prefix into `<project>/.pixi/envs/<name>`.

use crate::cli::UnpackArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{MANIFEST_DIR, Manifest};
use pixi_sandbox_core::platform::executable_filename;
use pixi_sandbox_core::shard;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(args: &UnpackArgs) -> Result<()> {
    let input = support::existing_dir(&args.input_dir, "--input-dir")?;
    let output = support::absolute(&args.output_dir)?;
    let resolved = match find_transport(&input) {
        Some(branch) => resolve_transport(args, branch)?,
        None => resolve_bare_pack(args, input)?,
    };

    if let ResolvedInput::Bare {
        source,
        unpacker: None,
        ..
    } = &resolved
    {
        // Bare packs have no outer transport verification. Inspect their local
        // channel before even --force is allowed to remove an existing output.
        pixi_sandbox::native_pack::verify(source)?;
    }

    if args.verify_only {
        println!("--verify-only: no output was written");
        return Ok(());
    }

    check_output(&output, args.force)?;
    let output_parent = output
        .parent()
        .ok_or_else(|| anyhow::anyhow!("--output-dir has no parent: {}", output.display()))?;
    let work = support::work_dir(output_parent, args.work_dir.as_deref())?;
    let stage = work.join(format!("unpack-{}", resolved.environment()));
    support::remove_path(&stage)?;
    fs::create_dir_all(&stage).with_context(|| format!("creating {}", stage.display()))?;

    let (source, unpacker) = match &resolved {
        ResolvedInput::Transport {
            branch,
            manifest,
            environment,
        } => {
            let packed = stage.join("pack");
            fs::create_dir_all(&packed)
                .with_context(|| format!("creating {}", packed.display()))?;
            let source_root = branch.join(MANIFEST_DIR);
            // `pack` is part of the transport path, not part of the staged pack directory.
            let prefix = format!("envs/{environment}/pack/");
            for blob in &manifest.envs[environment].blobs {
                let relative = support::relative_after(&blob.path, &prefix)?;
                shard::materialise(&source_root, blob, &packed.join(relative))?;
            }
            (
                packed,
                if args.unpacker.is_none()
                    && support::uses_native_pack(manifest, branch, environment)?
                {
                    None
                } else {
                    Some(resolve_unpacker(
                        args.unpacker.as_deref(),
                        manifest,
                        branch,
                    )?)
                },
            )
        }
        ResolvedInput::Bare {
            source, unpacker, ..
        } => (source.clone(), unpacker.clone()),
    };

    let environment = resolved.environment();
    if let Some(unpacker) = &unpacker {
        let mut command = Command::new(unpacker);
        command
            .arg(&source)
            .arg("-o")
            .arg(&stage)
            .arg("-e")
            .arg(environment);
        support::use_work_tmp(&mut command, &work);
        support::run(&mut command)
            .with_context(|| format!("unpacking environment {environment}"))?;
    } else {
        pixi_sandbox::native_pack::install(
            &source,
            &stage.join(environment),
            &output,
            &stage.join("package-cache"),
        )?;
    }

    let prefix = stage.join(environment);
    if !prefix.is_dir() {
        bail!(
            "{} completed but did not create the expected prefix {}",
            unpacker.as_deref().map_or_else(
                || "native installer".to_string(),
                |path| path.display().to_string()
            ),
            prefix.display()
        );
    }
    verify_staged_output(&resolved, &prefix, &output, &work)?;
    promote_prefix(&prefix, &output, args.force)?;
    println!("unpacked {environment} -> {}", output.display());
    println!("  markers are restore's job; this is a raw prefix");
    Ok(())
}

fn verify_staged_output(
    resolved: &ResolvedInput,
    prefix: &Path,
    output: &Path,
    work: &Path,
) -> Result<()> {
    if let ResolvedInput::Transport {
        branch,
        manifest,
        environment,
    } = resolved
    {
        // Raw unpack deliberately omits Pixi markers, but every payload file still has to
        // match its oracle before a prior output may be retired.
        let mut raw_manifest = (**manifest).clone();
        raw_manifest
            .envs
            .get_mut(environment)
            .expect("selected environment exists")
            .pixi_environment_fingerprint = None;
        let checked = pixi_sandbox_core::verify::verify_staged(
            &raw_manifest,
            branch,
            output,
            environment,
            prefix,
            work,
        );
        if !checked.report.failures.is_empty() {
            bail!(
                "unpacked tree does not match the transport oracle: {:?}",
                checked.report.failures
            );
        }
    }
    Ok(())
}

enum ResolvedInput {
    Transport {
        branch: PathBuf,
        manifest: Box<Manifest>,
        environment: String,
    },
    Bare {
        source: PathBuf,
        unpacker: Option<PathBuf>,
        environment: String,
    },
}

impl ResolvedInput {
    fn environment(&self) -> &str {
        match self {
            Self::Transport { environment, .. } | Self::Bare { environment, .. } => environment,
        }
    }
}

/// The transport can be passed directly, or the user can point at a pack nested below it.
fn find_transport(input: &Path) -> Option<PathBuf> {
    let mut current = input.to_path_buf();
    loop {
        if Manifest::path_in(&current).is_file() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn resolve_transport(args: &UnpackArgs, branch: PathBuf) -> Result<ResolvedInput> {
    let manifest_path = Manifest::path_in(&branch);
    let manifest = Manifest::load(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;
    let environment = match &args.env {
        Some(name) => name.clone(),
        None if manifest.envs.len() == 1 => manifest
            .envs
            .keys()
            .next()
            .expect("len checked above")
            .clone(),
        None => bail!(
            "this transport holds {} — choose one with --env",
            manifest.envs.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
    };
    if !manifest.envs.contains_key(&environment) {
        bail!("environment {environment:?} is not in this transport");
    }

    let selected = vec![environment.clone()];
    let report = support::verify_transport(&manifest, &branch, Some(&selected))?;
    println!(
        "verified {environment}: {} blob(s), {} MiB before writing",
        report.files,
        support::mib(report.bytes)
    );
    Ok(ResolvedInput::Transport {
        branch,
        manifest: Box::new(manifest),
        environment,
    })
}

fn resolve_bare_pack(args: &UnpackArgs, input: PathBuf) -> Result<ResolvedInput> {
    let environment = args.env.clone().unwrap_or_else(|| "env".to_string());
    let mut components = Path::new(&environment).components();
    if !matches!(components.next(), Some(std::path::Component::Normal(_)))
        || components.next().is_some()
        || environment.contains(['\\', ':'])
    {
        bail!("unsafe environment name {environment:?}");
    }
    println!(
        "no transport manifest above {}; treating it as a bare pack directory",
        input.display()
    );
    let unpacker = args
        .unpacker
        .as_deref()
        .map(support::absolute)
        .transpose()?;
    if let Some(path) = &unpacker
        && !path.is_file()
    {
        bail!("unpacker is not a file: {}", path.display());
    }

    Ok(ResolvedInput::Bare {
        source: input,
        unpacker,
        environment,
    })
}

fn resolve_unpacker(
    explicit: Option<&Path>,
    manifest: &Manifest,
    branch: &Path,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        let path = support::absolute(path)?;
        if !path.is_file() {
            bail!("--unpacker is not a file: {}", path.display());
        }
        return Ok(path);
    }

    if let Some(entry) = manifest.tools.get("pixi-unpack") {
        let fallback = format!(
            "tools/{}/{}",
            manifest.platform,
            executable_filename("pixi-unpack", &manifest.platform)
        );
        let relative = entry.path.as_deref().unwrap_or(&fallback);
        let path = branch.join(MANIFEST_DIR).join(relative);
        if path.is_file() {
            return Ok(path);
        }
    }
    support::find_executable("pixi-unpack")
        .ok_or_else(|| anyhow::anyhow!("the transport has no pixi-unpack and none is on PATH"))
}

fn check_output(output: &Path, force: bool) -> Result<()> {
    match fs::symlink_metadata(output) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).with_context(|| format!("reading {}", output.display())),
        Ok(metadata) => {
            let empty_directory = metadata.file_type().is_dir()
                && !metadata.file_type().is_symlink()
                && fs::read_dir(output)
                    .with_context(|| format!("reading {}", output.display()))?
                    .next()
                    .is_none();
            if !empty_directory && !force {
                bail!(
                    "{} already exists and is not empty — pass --force to replace it",
                    output.display()
                );
            }
        }
    }
    Ok(())
}

/// Do not retire an existing output until installation succeeds. Keep a same-parent
/// backup until the stage rename succeeds, including when --work-dir crosses devices.
pub(super) fn promote_prefix(prefix: &Path, output: &Path, force: bool) -> Result<()> {
    check_output(output, force)?;
    if fs::symlink_metadata(output).is_err() {
        return fs::rename(prefix, output)
            .with_context(|| format!("promoting unpacked prefix to {}", output.display()));
    }
    let parent = output.parent().context("output has no parent")?;
    let backup_dir = tempfile::Builder::new()
        .prefix(".pixi-sandbox-unpack-")
        .tempdir_in(parent)?;
    let backup = backup_dir.path().join("previous");
    fs::rename(output, &backup).context("retiring previous unpack output")?;
    if let Err(error) = fs::rename(prefix, output) {
        if let Err(rollback) = fs::rename(&backup, output) {
            let saved = backup_dir.keep();
            bail!(
                "promotion failed: {error}; rollback failed: {rollback}; previous output retained in {}",
                saved.join("previous").display()
            );
        }
        return Err(error).context("promoting unpacked prefix; previous output restored");
    }
    backup_dir
        .close()
        .context("removing previous unpack output")
}
