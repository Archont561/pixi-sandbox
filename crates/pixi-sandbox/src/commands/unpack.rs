//! `unpack` — one packed environment to one raw prefix.
//!
//! Unlike `restore`, this command deliberately does not write Pixi's environment markers. It
//! is useful for a pack delivered outside the branch workflow, while `restore` remains the
//! operation that turns a verified prefix into `<project>/.pixi/envs/<name>`.

use crate::cli::UnpackArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{MANIFEST_DIR, Manifest};
use pixi_sandbox_core::shard;
use pixi_sandbox_core::tools_lock::executable_filename;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn run(args: UnpackArgs) -> Result<()> {
    let input = support::existing_dir(&args.input_dir, "--input-dir")?;
    let output = support::absolute(&args.output_dir)?;
    let resolved = match find_transport(&input) {
        Some(branch) => resolve_transport(&args, branch)?,
        None => resolve_bare_pack(&args, input)?,
    };

    if args.verify_only {
        println!("--verify-only: no output was written");
        return Ok(());
    }

    prepare_output(&output, args.force)?;
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
                resolve_unpacker(args.unpacker.as_deref(), manifest, branch)?,
            )
        }
        ResolvedInput::Bare {
            source, unpacker, ..
        } => (source.clone(), unpacker.clone()),
    };

    let environment = resolved.environment();
    let mut command = Command::new(&unpacker);
    command
        .arg(&source)
        .arg("-o")
        .arg(&stage)
        .arg("-e")
        .arg(environment);
    support::use_work_tmp(&mut command, &work);
    support::run(&mut command).with_context(|| format!("unpacking environment {environment}"))?;

    let prefix = stage.join(environment);
    if !prefix.is_dir() {
        bail!(
            "{} completed but did not create the expected prefix {}",
            unpacker.display(),
            prefix.display()
        );
    }
    fs::rename(&prefix, &output).with_context(|| {
        format!(
            "moving verified unpacked prefix {} to {}",
            prefix.display(),
            output.display()
        )
    })?;
    println!("unpacked {environment} -> {}", output.display());
    println!("  markers are restore's job; this is a raw prefix");
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
        unpacker: PathBuf,
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
    if environment.is_empty() {
        bail!("--env must not be empty");
    }
    println!(
        "no transport manifest above {}; treating it as a bare pack directory",
        input.display()
    );
    let unpacker = match args.unpacker.as_deref() {
        Some(path) => support::absolute(path)?,
        None => support::find_executable("pixi-unpack")
            .ok_or_else(|| anyhow::anyhow!("pixi-unpack is not on PATH; pass --unpacker"))?,
    };
    if !unpacker.is_file() {
        bail!("unpacker is not a file: {}", unpacker.display());
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

fn prepare_output(output: &Path, force: bool) -> Result<()> {
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
            support::remove_path(output)?;
        }
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    Ok(())
}
