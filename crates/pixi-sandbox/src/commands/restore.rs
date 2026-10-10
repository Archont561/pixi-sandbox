//! `restore` — verify an extracted branch, then install its environments into a project.
//!
//! The ordering is the safety contract: validate and hash every selected payload byte first;
//! only then materialise copies under a project-local work directory, unpack them, and rename
//! complete prefixes into place. The fetched branch is never modified.

use crate::cli::{CargoConfigArg, RestoreArgs, UserToolsPolicy};
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::user_tools::{self, LauncherChange, LauncherKind, PathChange, UserTools};
use pixi_sandbox_core::manifest::{Blob, Env, MANIFEST_DIR, Manifest, ToolEntry, Vendor};
use pixi_sandbox_core::shard;
use pixi_sandbox_core::tools_lock::executable_filename;
use pixi_sandbox_core::verify::{self, Linkage};
use std::fs::{self, File};
use std::path::{Component, Path, PathBuf};
use std::process::Command;

pub fn run(args: RestoreArgs) -> Result<()> {
    let branch = support::existing_dir(&args.branch_location, "--branch-location")?;
    let project = support::existing_dir(&args.output_path, "--output-path")?;
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

    let mut vendored = false;
    let mut cargo_wiring = CargoWiring::NotVendored;
    if let Some(vendor) = &manifest.vendor {
        if !args.no_vendor {
            println!("restore vendored cargo dependencies");
            install_vendor(&branch, vendor, &project, &work, args.force)?;
            cargo_wiring = configure_cargo_vendor(&project, &environments, args.cargo_config)?;
            vendored = true;
        }
    }

    remove_legacy_sandbox_env(&project)?;

    // task-33: registration is gated on *two* verifications. The branch was verified before
    // anything was written; this checks the tree that came out the other end against the
    // manifest's per-file oracle (D13) — the same check `doctor --verify-restored` runs, so a
    // restore that reports success has proven both sides before it touches the user's home.
    // A failure here is a restore failure: it bails *before* the work dir is cleaned, because
    // the staged packs are the evidence.
    let restored = verify::verify_restored(
        &manifest,
        &branch,
        &project,
        Some(&environments),
        args.work_dir.as_deref(),
    );
    report_restored_tree(&restored)?;

    // Everything under the work dir is scratch this command created, and on a success nothing
    // needs it: the installed environment holds its own directory entries and shares inodes
    // with the staged blobs, so removing the stage cannot orphan a byte. Issue #18 measured
    // 5.5 GiB of `.restore-work` surviving a restore that reported `restore complete`. A failed
    // restore returns above and keeps the staged packs as evidence, which is why the cleanup
    // lives here and not in a drop guard.
    clean_work_dir(&work, &environments, vendored)?;

    // The last step, and only after both verifications passed: make the manifest-verified
    // `pixi` and `pixi-sandbox` copies persistently discoverable. A refusal here (an
    // unmanaged collision in the user bin directory) does not undo the restore — the message
    // says so and names the remedies — but it does fail the command, because exit 0 must mean
    // "the tools are registered, or you asked us not to".
    register_user_tools(
        &manifest,
        &tools_dir,
        args.user_tools,
        args.user_bin.as_deref(),
        args.force,
    )?;

    println!("restore complete");
    println!("  pixi install --frozen --offline      # must be a no-op");
    match cargo_wiring {
        CargoWiring::SandboxHome { cargo_home } => {
            println!(
                "  cargo vendor: wired via {} and Pixi activation hooks",
                cargo_home.display()
            );
            println!("  pixi run --frozen -- cargo build --offline");
        }
        CargoWiring::ProjectConfig { config } => {
            println!("  cargo vendor: wired via {}", config.display());
            println!("  pixi run --frozen -- cargo build --offline");
        }
        CargoWiring::Printed => println!(
            "  cargo vendor: NOT wired (--cargo-config print); write the printed config before building offline"
        ),
        CargoWiring::None => println!(
            "  cargo vendor: NOT wired (--cargo-config none); configure Cargo before building offline"
        ),
        CargoWiring::NotVendored => {}
    }
    println!(
        "  no .pixi/sandbox-env.sh is generated; use pixi as the only entrypoint (`pixi run ...`)"
    );
    if matches!(args.user_tools, UserToolsPolicy::Register) {
        println!(
            "  open a new shell (or put the managed user bin on PATH here) for pixi and pixi sandbox"
        );
    }
    Ok(())
}

/// Print the restored-tree verdict in the same voice `doctor --verify-restored` uses, and
/// fail closed on any mismatch. Schema-1 environments are reported `unverifiable`, never
/// failed (D13): a published branch outlives the tool that packed it, and registration
/// proceeds on branch verification alone for those.
fn report_restored_tree(restored: &pixi_sandbox_core::verify::RestoredReport) -> Result<()> {
    println!("verify restored tree");
    for name in &restored.verified {
        println!(
            "  env {name}: checked against the manifest's file list — {} entry(ies), 0 failure(s)",
            restored.report.files
        );
    }
    for name in &restored.unverifiable {
        println!(
            "  env {name}: no per-file digests in this manifest (schema 1 predates the oracle) \
             — content not verified; branch verification still passed"
        );
    }
    if !restored.report.failures.is_empty() {
        for failure in &restored.report.failures {
            println!(
                "  {}: {}: {}",
                failure.path,
                failure.kind.as_str(),
                failure.detail
            );
        }
        bail!(
            "the restored project does not match the manifest ({} failure(s)); the work dir is \
             kept for inspection",
            restored.report.failures.len()
        );
    }
    if !restored.verified.is_empty() {
        println!("  OK — the restored tree matches the manifest");
    }
    Ok(())
}

/// task-33: register the restored `pixi` and `pixi-sandbox` for the user. `pixi` is the entry
/// point and `pixi sandbox` resolves through PATH, so both get launchers; they point at the
/// manifest-verified copies under `.pixi/tools/<platform>/` and nothing else (decision-2 §4.1:
/// the `tools` entry is the canonical executable, never an environment-embedded copy).
fn register_user_tools(
    manifest: &Manifest,
    tools_dir: &Path,
    policy: UserToolsPolicy,
    user_bin: Option<&Path>,
    force: bool,
) -> Result<()> {
    if matches!(policy, UserToolsPolicy::Skip) {
        println!("user tools: none registered (--user-tools skip; the project itself is complete)");
        return Ok(());
    }

    let mut tools = Vec::new();
    for name in user_tools::REGISTERED_TOOLS {
        if manifest.tools.contains_key(name) {
            tools.push((
                name.to_string(),
                tools_dir.join(tool_file_name(manifest, name)),
            ));
        }
    }
    if tools.is_empty() {
        println!(
            "user tools: none registered (the transport embeds neither pixi nor pixi-sandbox)"
        );
        return Ok(());
    }
    if !tools.iter().any(|(name, _)| name == "pixi-sandbox") {
        println!("  note: this transport embeds no pixi-sandbox; `pixi sandbox` needs one on PATH");
    }

    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "cannot register user tools: neither HOME nor USERPROFILE is set\n\
                 the restored project itself is complete; pass --user-tools skip, set HOME, \
                 or pass --user-bin <dir>"
            )
        })?;
    let bin_dir = match user_bin {
        Some(dir) => support::absolute(dir)?,
        None => user_tools::default_bin_dir(&home),
    };

    // The profile choice follows the detected shell on POSIX; Windows keeps its user PATH in
    // the registry and gets no profile edit.
    #[cfg(unix)]
    let (profile, notice) = {
        let detected = user_tools::detect_profile(&home, std::env::var_os("SHELL").as_deref());
        (Some(detected.0), detected.1)
    };
    // `None::<String>` is load-bearing: the only use of `notice` is `println!("{notice}")`,
    // which leaves the element type unconstrained, so a bare `None` only fails to infer on the
    // targets that compile this branch — the Windows leg of the release, and nothing else.
    #[cfg(not(unix))]
    let (profile, notice) = (None::<PathBuf>, None::<String>);

    let service = UserTools {
        bin_dir: &bin_dir,
        profile: profile.as_deref(),
        force,
    };
    let (launchers, path) = service
        .register(LauncherKind::current(), &tools)
        .with_context(
            || "the restored project itself is complete, but registering user tools was refused",
        )?;

    println!("register user tools");
    let mut retargeted = false;
    for (name, change) in &launchers {
        let launcher = bin_dir.join(user_tools::launcher_file_name(
            name,
            LauncherKind::current(),
        ));
        match change {
            LauncherChange::Created => {
                println!("  {name} -> {}", launcher.display());
            }
            LauncherChange::Retargeted { from } => {
                retargeted = true;
                println!(
                    "  {name}: {} retargeted from {}",
                    launcher.display(),
                    from.display()
                );
            }
            LauncherChange::AlreadyCurrent => {
                println!("  {name}: {} (already current)", launcher.display());
            }
        }
    }
    if retargeted {
        println!(
            "  the most recently registered restore is now the user-level source of these tools"
        );
    }
    match &path {
        PathChange::Added { profile } => {
            println!(
                "  PATH: added {} to {}",
                bin_dir.display(),
                profile.display()
            );
        }
        PathChange::Replaced { profile } => println!(
            "  PATH: {} updated in {} (the bin directory changed)",
            bin_dir.display(),
            profile.display()
        ),
        PathChange::AlreadyPresent { profile } => {
            println!(
                "  PATH: {} already on PATH in {}",
                bin_dir.display(),
                profile.display()
            );
        }
        PathChange::RegistryAdded => {
            println!(
                "  PATH: {} added to the user PATH (registry)",
                bin_dir.display()
            );
        }
        PathChange::RegistryAlreadyPresent => {
            println!("  PATH: {} already on the user PATH", bin_dir.display());
        }
    }
    if let Some(notice) = notice {
        println!("  note: {notice}");
    }
    println!(
        "  note: an already-running shell cannot be changed; open a new one, or run \
         `export PATH=\"{}:$PATH\"` in this one",
        bin_dir.display()
    );
    Ok(())
}

/// Remove this restore's scratch: the unpacker's TMPDIR, one materialised pack and one stage per
/// environment, and the vendor stage. The directory itself goes only when we emptied it, so an
/// explicit `--work-dir` that was holding something else survives the restore that borrowed it.
fn clean_work_dir(work: &Path, environments: &[String], vendored: bool) -> Result<()> {
    support::remove_path(&work.join("tmp"))?;
    for environment in environments {
        support::remove_path(&work.join(format!("pack-{environment}")))?;
        support::remove_path(&work.join(format!("stage-{environment}")))?;
    }
    if vendored {
        support::remove_path(&work.join("vendor-stage"))?;
    }
    if work
        .read_dir()
        .is_ok_and(|mut entries| entries.next().is_none())
    {
        fs::remove_dir(work).with_context(|| format!("removing {}", work.display()))?;
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
        .map_or_else(
            || executable_filename(name, &manifest.platform),
            ToOwned::to_owned,
        )
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

    // The archive bytes were verified before extraction. Relocate textual conda metadata while
    // the replacement prefix is still staged, so no installed file points into restore scratch.
    let relocated = relocate_text_prefixes(&prefix, &target)?;
    if relocated > 0 {
        println!("  {environment}: relocated {relocated} text file(s) to the final prefix");
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

/// Replace a staging prefix in text while leaving fixed-width binary payloads untouched.
fn relocate_text_prefixes(staged_prefix: &Path, final_prefix: &Path) -> Result<usize> {
    let staged = staged_prefix
        .canonicalize()
        .with_context(|| format!("canonicalising staged prefix {}", staged_prefix.display()))?;
    let old = staged
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("staged environment path is not valid UTF-8"))?;
    let new = final_prefix
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("final environment path is not valid UTF-8"))?;
    let mut changed = 0;

    for path in shard::files_under(&staged)? {
        let bytes = fs::read(&path)
            .with_context(|| format!("reading {} for relocation", path.display()))?;
        if bytes.contains(&0) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&bytes) else {
            continue;
        };
        if !text.contains(old) {
            continue;
        }

        let permissions = fs::metadata(&path)?.permissions();
        let mut writable = permissions.clone();
        if writable.readonly() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                writable.set_mode(writable.mode() | 0o200);
            }
            #[cfg(not(unix))]
            writable.set_readonly(false);
            fs::set_permissions(&path, writable)?;
        }
        let write = fs::write(&path, text.replace(old, new));
        let restore_permissions = fs::set_permissions(&path, permissions);
        write.with_context(|| format!("relocating prefix in {}", path.display()))?;
        restore_permissions
            .with_context(|| format!("restoring permissions on {}", path.display()))?;
        changed += 1;
    }
    Ok(changed)
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

#[derive(Debug)]
enum CargoWiring {
    NotVendored,
    SandboxHome { cargo_home: PathBuf },
    ProjectConfig { config: PathBuf },
    Printed,
    None,
}

fn cargo_source_snippet(vendor_dir: &Path) -> String {
    let directory = toml::Value::String(vendor_dir.to_string_lossy().into_owned()).to_string();
    format!(
        "[source.crates-io]\nreplace-with = \"vendored-sources\"\n\n\
         [source.vendored-sources]\ndirectory = {directory}\n"
    )
}

fn configure_cargo_vendor(
    project: &Path,
    environments: &[String],
    mode: CargoConfigArg,
) -> Result<CargoWiring> {
    let vendor_dir = project.join(MANIFEST_DIR).join("vendor");
    let snippet = cargo_source_snippet(&vendor_dir);
    match mode {
        CargoConfigArg::Auto => {
            let cargo_home = project.join(MANIFEST_DIR).join("cargo-home");
            write_sandbox_cargo_home(&cargo_home, &snippet)?;
            write_cargo_activation_hooks(project, environments, &cargo_home)?;
            println!(
                "  cargo config: wrote {} and Pixi activation hooks (project .cargo left alone)",
                cargo_home.join("config.toml").display()
            );
            Ok(CargoWiring::SandboxHome { cargo_home })
        }
        CargoConfigArg::Write => {
            let config = project.join(".cargo").join("config.toml");
            let parent = config.parent().expect("config path has a .cargo parent");
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
            fs::write(&config, snippet).with_context(|| format!("writing {}", config.display()))?;
            println!(
                "  wrote {} with an absolute vendor directory",
                config.display()
            );
            Ok(CargoWiring::ProjectConfig { config })
        }
        CargoConfigArg::Print => {
            println!(
                "# Cargo source replacement for .pixi-sandbox/cargo-home/config.toml (or another Cargo config file)"
            );
            print!("{snippet}");
            Ok(CargoWiring::Printed)
        }
        CargoConfigArg::None => {
            println!("  cargo config: left alone (--cargo-config none)");
            Ok(CargoWiring::None)
        }
    }
}

fn write_sandbox_cargo_home(cargo_home: &Path, snippet: &str) -> Result<()> {
    fs::create_dir_all(cargo_home.join("bin"))
        .with_context(|| format!("creating {}", cargo_home.join("bin").display()))?;
    let config = cargo_home.join("config.toml");
    fs::write(&config, snippet).with_context(|| format!("writing {}", config.display()))?;
    Ok(())
}

fn write_cargo_activation_hooks(
    project: &Path,
    environments: &[String],
    cargo_home: &Path,
) -> Result<()> {
    for environment in environments {
        let activate_dir = project
            .join(".pixi")
            .join("envs")
            .join(environment)
            .join("etc")
            .join("conda")
            .join("activate.d");
        fs::create_dir_all(&activate_dir)
            .with_context(|| format!("creating {}", activate_dir.display()))?;
        write_posix_cargo_activation(&activate_dir, cargo_home)?;
        write_powershell_cargo_activation(&activate_dir, cargo_home)?;
        write_cmd_cargo_activation(&activate_dir, cargo_home)?;
    }
    Ok(())
}

fn write_posix_cargo_activation(activate_dir: &Path, cargo_home: &Path) -> Result<()> {
    let cargo_home = shell_single_quote(&cargo_home.to_string_lossy());
    let script = format!(
        "# generated by pixi-sandbox restore — keep Cargo on the restored vendor tree\n\
         export CARGO_HOME={cargo_home}\n\
         case \":$PATH:\" in\n\
         *\":$CARGO_HOME/bin:\"*) ;;\n\
         *) export PATH=\"$CARGO_HOME/bin:$PATH\" ;;\n\
         esac\n"
    );
    let path = activate_dir.join("pixi-sandbox-cargo-home.sh");
    fs::write(&path, script).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_powershell_cargo_activation(activate_dir: &Path, cargo_home: &Path) -> Result<()> {
    let cargo_home = powershell_single_quote(&cargo_home.to_string_lossy());
    let script = format!(
        "# generated by pixi-sandbox restore — keep Cargo on the restored vendor tree\n\
         $env:CARGO_HOME = {cargo_home}\n\
         $bin = Join-Path $env:CARGO_HOME 'bin'\n\
         if (-not (($env:PATH -split [IO.Path]::PathSeparator) -contains $bin)) {{\n\
             $env:PATH = \"$bin$([IO.Path]::PathSeparator)$env:PATH\"\n\
         }}\n"
    );
    let path = activate_dir.join("pixi-sandbox-cargo-home.ps1");
    fs::write(&path, script).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn write_cmd_cargo_activation(activate_dir: &Path, cargo_home: &Path) -> Result<()> {
    let cargo_home = cargo_home.to_string_lossy().replace('%', "%%");
    let script = format!(
        "@echo off\r\n\
         rem generated by pixi-sandbox restore — keep Cargo on the restored vendor tree\r\n\
         set \"CARGO_HOME={cargo_home}\"\r\n\
         set \"PATH=%CARGO_HOME%\\bin;%PATH%\"\r\n"
    );
    let path = activate_dir.join("pixi-sandbox-cargo-home.bat");
    fs::write(&path, script).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn shell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn powershell_single_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

/// Drop the pre-0.5 activation hook when a project is restored with a newer binary. The
/// supported contract is a registered `pixi` launcher plus `pixi run ...`; leaving the old file
/// behind would make a stale, unsupported PATH activation path look deliberate.
fn remove_legacy_sandbox_env(project: &Path) -> Result<()> {
    let path = project.join(".pixi").join("sandbox-env.sh");
    if path.exists() {
        fs::remove_file(&path).with_context(|| format!("removing {}", path.display()))?;
        println!("  removed legacy {}", path.display());
    }
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

#[cfg(test)]
mod tests {
    use super::{clean_work_dir, relocate_text_prefixes, remove_legacy_sandbox_env};
    use proptest::prelude::*;
    use std::fs;
    use std::path::Path;

    /// The staged prefix as an absolute string, the way a subprocess would have been handed it.
    fn staged_prefix(staged: &Path) -> String {
        staged
            .canonicalize()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn relocation_changes_text_and_preserves_binary_prefixes() {
        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("stage.with dots/env");
        let final_prefix = temp.path().join("final path/env");
        fs::create_dir_all(&staged).unwrap();
        let old = staged_prefix(&staged);
        fs::write(staged.join("metadata.pc"), format!("prefix={old}\n")).unwrap();
        let binary = [b"\0binary:".as_slice(), old.as_bytes()].concat();
        fs::write(staged.join("binary"), &binary).unwrap();

        assert_eq!(relocate_text_prefixes(&staged, &final_prefix).unwrap(), 1);
        assert_eq!(
            fs::read_to_string(staged.join("metadata.pc")).unwrap(),
            format!("prefix={}\n", final_prefix.display())
        );
        assert_eq!(fs::read(staged.join("binary")).unwrap(), binary);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        /// Prefix relocation has a deliberately narrow contract: valid UTF-8 text without a
        /// NUL gets rewritten, while any NUL-bearing payload remains byte-for-byte unchanged.
        #[test]
        fn relocation_rewrites_only_nul_free_utf8(
            suffix in proptest::collection::vec(any::<u8>(), 0..96),
            binary_tail in proptest::collection::vec(any::<u8>(), 0..96),
        ) {
            let temp = tempfile::tempdir().expect("tempdir");
            let staged = temp.path().join("stage/env");
            let final_prefix = temp.path().join("final/env");
            fs::create_dir_all(&staged).expect("stage directory");
            let old = staged_prefix(&staged);
            let suffix = String::from_utf8_lossy(&suffix).replace('\0', "");

            let text_path = staged.join("text.pc");
            let text = format!("before={old};{suffix};after={old}\n");
            fs::write(&text_path, &text).expect("write text");

            let binary_path = staged.join("binary");
            let binary = [old.as_bytes(), b"\0", binary_tail.as_slice()].concat();
            fs::write(&binary_path, &binary).expect("write binary");

            prop_assert_eq!(relocate_text_prefixes(&staged, &final_prefix).unwrap(), 1);
            prop_assert_eq!(
                fs::read_to_string(&text_path).unwrap(),
                format!("before={};{suffix};after={}\n", final_prefix.display(), final_prefix.display())
            );
            prop_assert_eq!(fs::read(&binary_path).unwrap(), binary);
        }
    }

    #[test]
    #[cfg(unix)]
    fn relocation_handles_read_only_repeated_and_non_utf8_files() {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir().unwrap();
        let staged = temp.path().join("env");
        fs::create_dir_all(&staged).unwrap();
        let old = staged_prefix(&staged);
        let final_prefix = temp.path().join("final");
        let new = final_prefix.to_string_lossy().into_owned();

        // A read-only header, as shipped by some conda packages: the write needs the bit flipped
        // and the mode handed back, or a later `pixi install --offline` cannot own the file.
        let read_only = staged.join("config.h");
        fs::write(&read_only, format!("#define PREFIX \"{old}\"\n")).unwrap();
        fs::set_permissions(&read_only, fs::Permissions::from_mode(0o444)).unwrap();

        // One file, several occurrences: a libtool `.la` names the prefix per section.
        let repeated = staged.join("libfoo.la");
        fs::write(&repeated, format!("prefix='{old}' {old}")).unwrap();

        // Valid bytes, invalid UTF-8: a latin-1 comment in a `.pc` file must survive intact.
        let latin1 = staged.join("latin1.pc");
        let latin1_bytes = [b"# caf\xe9 {".as_slice(), old.as_bytes(), b"}".as_slice()].concat();
        fs::write(&latin1, &latin1_bytes).unwrap();

        assert_eq!(relocate_text_prefixes(&staged, &final_prefix).unwrap(), 2);
        assert_eq!(
            fs::read_to_string(&read_only).unwrap(),
            format!("#define PREFIX \"{new}\"\n")
        );
        assert_eq!(
            fs::metadata(&read_only).unwrap().permissions().mode() & 0o777,
            0o444
        );
        assert_eq!(
            fs::read_to_string(&repeated).unwrap(),
            format!("prefix='{new}' {new}")
        );
        assert_eq!(fs::read(&latin1).unwrap(), latin1_bytes);
    }

    #[test]
    fn legacy_sandbox_env_is_removed_during_restore_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join(".pixi/sandbox-env.sh");
        fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        fs::write(&legacy, b"export PATH=legacy\n").unwrap();

        remove_legacy_sandbox_env(temp.path()).unwrap();

        assert!(!legacy.exists());
        remove_legacy_sandbox_env(temp.path()).unwrap();
    }

    #[test]
    fn cleanup_removes_this_restores_scratch_and_only_its_own() {
        let temp = tempfile::tempdir().unwrap();
        let work = temp.path().join(".pixi/.restore-work");
        for scratch in [
            "tmp",
            "pack-default",
            "stage-default",
            "stage-docs",
            "vendor-stage",
        ] {
            fs::create_dir_all(work.join(scratch)).unwrap();
        }

        // Without a vendor stage there is nothing to remove, and the borrowed directory is
        // otherwise empty, so it goes too.
        clean_work_dir(&work, &["default".to_string(), "docs".to_string()], false).unwrap();
        assert!(!work.join("tmp").exists());
        assert!(!work.join("pack-default").exists());
        assert!(!work.join("stage-default").exists());
        assert!(!work.join("stage-docs").exists());
        assert!(work.join("vendor-stage").exists());
        assert!(work.exists());

        clean_work_dir(&work, &[], true).unwrap();
        assert!(!work.exists());
    }

    #[test]
    fn cleanup_leaves_a_work_dir_that_holds_something_else() {
        let temp = tempfile::tempdir().unwrap();
        let work = temp.path().join(".restore-work");
        fs::create_dir_all(work.join("pack-default")).unwrap();
        fs::write(work.join("operator-notes.txt"), "keep me\n").unwrap();

        clean_work_dir(&work, &["default".to_string()], false).unwrap();
        assert!(!work.join("pack-default").exists());
        assert!(work.join("operator-notes.txt").is_file());
    }
}
