//! `restore` — verify an extracted branch, then install its environments into a project.
//!
//! The ordering is the safety contract: validate and hash every selected payload byte first;
//! only then materialise copies under a project-local work directory, unpack them, and rename
//! complete prefixes into place. The fetched branch is never modified.

use crate::cli::{CargoConfigArg, RestoreArgs};
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{Blob, Env, MANIFEST_DIR, Manifest, ToolEntry, Vendor};
use pixi_sandbox_core::shard;
use pixi_sandbox_core::tools_lock::executable_filename;
use pixi_sandbox_core::verify::{self, Linkage};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn run(args: RestoreArgs) -> Result<()> {
    let branch = support::existing_dir(&args.branch_location, "--branch-location")?;
    let project = support::existing_dir(&args.path_to_main_repo_code, "--path-to-main-repo-code")?;
    let manifest_path = Manifest::path_in(&branch);
    let manifest = Manifest::load(&manifest_path)
        .with_context(|| format!("loading {}", manifest_path.display()))?;
    let environments = support::select_envs(&manifest, &args.envs)?;

    println!("verify transport before writing");
    let report = support::verify_transport(&manifest, &branch, Some(&environments))?;
    println!(
        "  {} blob(s), {} MiB — every declared byte matches",
        report.files,
        support::mib(report.bytes)
    );
    if args.verify_only {
        println!("--verify-only: no project files were written");
        return Ok(());
    }

    // Refuse all pre-existing destinations before materialising even an otherwise harmless
    // tool. A failed non-force restore should leave the project exactly as it found it.
    ensure_destinations_available(
        &project,
        &environments,
        manifest.vendor.as_ref(),
        args.no_vendor,
        args.force,
    )?;

    let work = support::work_dir(&project, args.work_dir.as_deref())?;
    ensure_same_filesystem(&work, &project)?;
    let needed = required_space(&manifest, &environments)?;
    support::preflight_space(&work, needed)?;

    println!("materialise tools");
    let tools_dir = materialise_tools(&branch, &manifest, &project, args.force)?;
    let unpacker = tools_dir.join(tool_file_name(&manifest, "pixi-unpack"));
    let unpacker = if unpacker.is_file() {
        unpacker
    } else {
        support::find_executable("pixi-unpack").ok_or_else(|| {
            anyhow::anyhow!(
                "the transport does not contain pixi-unpack and none is on PATH; cannot restore"
            )
        })?
    };

    println!("restore environments");
    for environment in &environments {
        install_environment(
            &branch,
            &manifest,
            environment,
            &project,
            &work,
            &unpacker,
            args.force,
        )?;
    }

    if let Some(vendor) = &manifest.vendor {
        if !args.no_vendor {
            println!("restore vendored cargo dependencies");
            install_vendor(&branch, vendor, &project, &work, args.force)?;
            write_cargo_config(&project, args.cargo_config)?;
        }
    }

    write_sandbox_env(&project, &manifest)?;
    // pixi-unpack's temporary files are disposable. Keep the staged packs by default: when a
    // large airlock restore fails after extraction they are useful evidence, but never leave
    // the tool's own TMPDIR payload around.
    support::remove_path(&work.join("tmp"))?;

    println!("restore complete");
    println!("  source {}/.pixi/sandbox-env.sh", project.display());
    println!("  pixi install --frozen --offline   # must be a no-op");
    if manifest.vendor.is_some() && !args.no_vendor {
        println!("  cargo build --offline             # must use the restored vendor tree");
    }
    Ok(())
}

fn required_space(manifest: &Manifest, environments: &[String]) -> Result<u64> {
    let mut required = 0u64;
    for name in environments {
        let env = manifest
            .envs
            .get(name)
            .expect("select_envs checked every requested environment");
        required = required
            .checked_add(env.packed_size_bytes)
            .and_then(|total| total.checked_add(env.unpacked_size_bytes))
            .ok_or_else(|| anyhow::anyhow!("restore space estimate overflow"))?;
    }
    if let Some(vendor) = &manifest.vendor {
        required = required
            .checked_add(
                vendor
                    .size_bytes
                    .checked_mul(2)
                    .ok_or_else(|| anyhow::anyhow!("vendor size estimate overflow"))?,
            )
            .ok_or_else(|| anyhow::anyhow!("restore space estimate overflow"))?;
    }
    Ok(required)
}

fn ensure_destinations_available(
    project: &Path,
    environments: &[String],
    vendor: Option<&Vendor>,
    no_vendor: bool,
    force: bool,
) -> Result<()> {
    if force {
        return Ok(());
    }
    for environment in environments {
        let target = project.join(".pixi").join("envs").join(environment);
        if target.exists() {
            bail!(
                "{} already exists — pass --force to replace it",
                target.display()
            );
        }
    }
    if vendor.is_some() && !no_vendor {
        let target = project.join(MANIFEST_DIR).join("vendor");
        if target.exists() {
            bail!(
                "{} already exists — pass --force to replace it",
                target.display()
            );
        }
    }
    Ok(())
}

fn materialise_tools(
    branch: &Path,
    manifest: &Manifest,
    project: &Path,
    force: bool,
) -> Result<PathBuf> {
    let destination_root = project.join(".pixi").join("tools").join(&manifest.platform);
    let source_root = branch.join(MANIFEST_DIR);

    for (name, entry) in &manifest.tools {
        let relative = tool_relative(name, entry, &manifest.platform);
        let source = source_root.join(&relative);
        if !source.is_file() {
            bail!(
                "tool {name} is missing from the verified transport: {}",
                source.display()
            );
        }
        let expected_sha256 = entry
            .pinned_sha256
            .clone()
            // Unpinned system tools are allowed only for an explicitly non-fetching pack;
            // calculate a source hash now so materialise still verifies its copy.
            .unwrap_or(shard::sha256_file(&source)?);
        let destination = destination_root.join(tool_file_name(manifest, name));

        if !force
            && destination.is_file()
            && shard::verify_file(&destination, &expected_sha256, entry.size_bytes).is_ok()
        {
            println!("  {name} {}: already present and verified", entry.version);
            continue;
        }

        let blob = Blob {
            path: relative.clone(),
            size: entry.size_bytes,
            sha256: expected_sha256,
            parts: Vec::new(),
        };
        shard::materialise(&source_root, &blob, &destination)?;
        support::make_executable(&destination)?;
        if verify::linkage_of(&destination) == Linkage::Dynamic {
            bail!("restored tool {name} is dynamically linked — refusing an airlock-unsafe tool");
        }
        println!("  {name} {} -> {}", entry.version, destination.display());
    }
    Ok(destination_root)
}

fn tool_relative(name: &str, entry: &ToolEntry, platform: &str) -> String {
    entry
        .path
        .clone()
        .unwrap_or_else(|| format!("tools/{platform}/{}", executable_filename(name, platform)))
        .trim_start_matches(&format!("{MANIFEST_DIR}/"))
        .to_string()
}

/// Preserve the filename recorded by the manifest. This makes a Windows transport contain
/// executable `.exe` files while keeping the manifest map keyed by extension-free tool identity.
fn tool_file_name(manifest: &Manifest, name: &str) -> String {
    manifest
        .tools
        .get(name)
        .and_then(|entry| entry.path.as_deref())
        .and_then(|relative| Path::new(relative).file_name())
        .and_then(|file| file.to_str())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| executable_filename(name, &manifest.platform))
}

fn install_environment(
    branch: &Path,
    manifest: &Manifest,
    environment: &str,
    project: &Path,
    work: &Path,
    unpacker: &Path,
    force: bool,
) -> Result<()> {
    let entry = manifest
        .envs
        .get(environment)
        .expect("select_envs checked requested environment");
    let target = project.join(".pixi").join("envs").join(environment);
    if target.exists() && !force {
        bail!(
            "{} already exists — pass --force to replace it",
            target.display()
        );
    }

    let pack = work.join(format!("pack-{environment}"));
    support::remove_path(&pack)?;
    fs::create_dir_all(&pack).with_context(|| format!("creating {}", pack.display()))?;
    let source_root = branch.join(MANIFEST_DIR);
    // Blob paths are rooted at `envs/<name>/pack/`; pixi-unpack expects the contents of
    // that directory itself, not an additional nested `pack/` component.
    let prefix = format!("envs/{environment}/pack/");
    for blob in &entry.blobs {
        let relative = support::relative_after(&blob.path, &prefix)?;
        shard::materialise(&source_root, blob, &pack.join(relative))?;
    }
    println!(
        "  {environment}: {} blobs materialised into {}",
        entry.blobs.len(),
        pack.display()
    );

    let stage = work.join(format!("stage-{environment}"));
    support::remove_path(&stage)?;
    fs::create_dir_all(&stage).with_context(|| format!("creating {}", stage.display()))?;
    let mut command = Command::new(unpacker);
    command
        .arg(&pack)
        .arg("-o")
        .arg(&stage)
        .arg("-e")
        .arg(environment);
    support::use_work_tmp(&mut command, work);
    support::run(&mut command).with_context(|| format!("unpacking environment {environment}"))?;

    let prefix = stage.join(environment);
    if !prefix.is_dir() {
        bail!(
            "{} completed but did not create {}",
            unpacker.display(),
            prefix.display()
        );
    }
    if target.exists() {
        // `--force` was checked before any source bytes were materialised; deletion happens
        // only after the replacement prefix is complete in its stage directory.
        support::remove_path(&target)?;
    }
    let parent = target
        .parent()
        .expect("environment target always has an envs parent");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::rename(&prefix, &target).with_context(|| {
        format!(
            "moving complete environment {} to {}",
            prefix.display(),
            target.display()
        )
    })?;
    write_markers(&target, entry)?;
    println!(
        "  {environment} -> {} ({} MiB unpacked)",
        target.display(),
        support::mib(entry.unpacked_size_bytes)
    );
    Ok(())
}

fn write_markers(prefix: &Path, entry: &Env) -> Result<()> {
    let conda_meta = prefix.join("conda-meta");
    fs::create_dir_all(&conda_meta)
        .with_context(|| format!("creating {}", conda_meta.display()))?;
    let resolved_prefix = prefix
        .canonicalize()
        .with_context(|| format!("canonicalising restored prefix {}", prefix.display()))?;
    fs::write(
        conda_meta.join("pixi_env_prefix"),
        resolved_prefix.to_string_lossy().as_bytes(),
    )
    .context("writing pixi_env_prefix marker")?;
    if let Some(fingerprint) = &entry.pixi_environment_fingerprint {
        fs::write(
            conda_meta.join(".pixi-environment-fingerprint"),
            fingerprint.as_bytes(),
        )
        .context("writing pixi environment fingerprint")?;
    }
    Ok(())
}

fn install_vendor(
    branch: &Path,
    vendor: &Vendor,
    project: &Path,
    work: &Path,
    force: bool,
) -> Result<()> {
    let target = project.join(MANIFEST_DIR).join("vendor");
    if target.exists() && !force {
        bail!(
            "{} already exists — pass --force to replace it",
            target.display()
        );
    }

    let stage = work.join("vendor-stage");
    support::remove_path(&stage)?;
    let staged_tree = stage.join("vendor");
    fs::create_dir_all(&staged_tree)
        .with_context(|| format!("creating {}", staged_tree.display()))?;
    let source_root = branch.join(MANIFEST_DIR);

    match vendor.mode.as_str() {
        "loose" => {
            for blob in &vendor.blobs {
                let relative = support::relative_after(&blob.path, "vendor/")?;
                shard::materialise(&source_root, blob, &staged_tree.join(relative))?;
            }
        }
        "tarballs" => {
            let archives = stage.join("archives");
            fs::create_dir_all(&archives)
                .with_context(|| format!("creating {}", archives.display()))?;
            for blob in &vendor.blobs {
                let relative = support::relative_after(&blob.path, "vendor/")?;
                let archive = archives.join(relative);
                shard::materialise(&source_root, blob, &archive)?;
                unpack_vendor_archive(&archive, &staged_tree)?;
            }
        }
        other => bail!("unsupported vendor mode {other:?} in manifest"),
    }

    if target.exists() {
        support::remove_path(&target)?;
    }
    let parent = target
        .parent()
        .expect("vendor target always has a .pixi-sandbox parent");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::rename(&staged_tree, &target).with_context(|| {
        format!(
            "moving complete vendor tree {} to {}",
            staged_tree.display(),
            target.display()
        )
    })?;
    println!(
        "  vendor -> {} ({} crates · {} MiB · {})",
        target.display(),
        vendor.crates,
        support::mib(vendor.size_bytes),
        vendor.mode
    );
    Ok(())
}

fn unpack_vendor_archive(archive_path: &Path, destination: &Path) -> Result<()> {
    let file =
        File::open(archive_path).with_context(|| format!("opening {}", archive_path.display()))?;
    let mut archive = tar::Archive::new(file);
    for entry in archive
        .entries()
        .with_context(|| format!("reading {}", archive_path.display()))?
    {
        let mut entry =
            entry.with_context(|| format!("reading entry in {}", archive_path.display()))?;
        let path = entry
            .path()
            .with_context(|| format!("reading entry path in {}", archive_path.display()))?;
        if path.is_absolute()
            || path.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
            || entry.header().entry_type().is_symlink()
            || entry.header().entry_type().is_hard_link()
        {
            bail!(
                "unsafe entry {} in vendor archive {}",
                path.display(),
                archive_path.display()
            );
        }
        let unpacked = entry
            .unpack_in(destination)
            .with_context(|| format!("unpacking {}", archive_path.display()))?;
        if !unpacked {
            bail!(
                "vendor archive {} tried to escape its destination",
                archive_path.display()
            );
        }
    }
    Ok(())
}

fn write_cargo_config(project: &Path, mode: CargoConfigArg) -> Result<()> {
    let snippet = format!(
        "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n\
         [source.vendored-sources]\ndirectory = \"{MANIFEST_DIR}/vendor\"\n"
    );
    let config = project.join(".cargo").join("config.toml");
    match mode {
        CargoConfigArg::None => println!("  cargo config: left alone (--cargo-config none)"),
        CargoConfigArg::Print => print!("{snippet}"),
        CargoConfigArg::Auto if config.exists() => println!(
            "  {} already exists — left alone (use --cargo-config write to replace it)",
            config.display()
        ),
        CargoConfigArg::Auto | CargoConfigArg::Write => {
            let parent = config.parent().expect("config path has a .cargo parent");
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
            fs::write(&config, snippet).with_context(|| format!("writing {}", config.display()))?;
            println!(
                "  wrote {} with a project-relative vendor directory",
                config.display()
            );
        }
    }
    Ok(())
}

fn write_sandbox_env(project: &Path, manifest: &Manifest) -> Result<()> {
    let platform = &manifest.platform;
    let pixi_file = tool_file_name(manifest, "pixi");
    let path = project.join(".pixi").join("sandbox-env.sh");
    let parent = path.parent().expect("sandbox env path has a .pixi parent");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let script = format!(
        "# generated by pixi-sandbox restore — source .pixi/sandbox-env.sh\n\
         _pixi_sandbox_dir=\"$(CDPATH= cd -- \"$(dirname -- \"${{BASH_SOURCE[0]}}\")\" && pwd)\"\n\
         export PATH=\"$_pixi_sandbox_dir/tools/{platform}:$PATH\"\n\
         export CARGO_NET_OFFLINE=true\n\
         pixi() {{ command \"$_pixi_sandbox_dir/tools/{platform}/{pixi_file}\" \"$@\"; }}\n"
    );
    fs::write(&path, script).with_context(|| format!("writing {}", path.display()))?;
    support::make_executable(&path)?;
    Ok(())
}

/// Default work directories are already inside the project. An explicit work directory must
/// also share the target filesystem or the final rename would silently become an expensive copy
/// (or fail halfway on a cross-device rename).
fn ensure_same_filesystem(work: &Path, project: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let work_device = fs::metadata(work)
            .with_context(|| format!("reading {}", work.display()))?
            .dev();
        let project_device = fs::metadata(project)
            .with_context(|| format!("reading {}", project.display()))?
            .dev();
        if work_device != project_device {
            bail!(
                "{} is not on the same filesystem as {}; choose a work dir under the project",
                work.display(),
                project.display()
            );
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (work, project);
    }
    Ok(())
}
