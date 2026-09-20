//! `pack` — build a self-contained transport on the connected side.
//!
//! The ordering below is intentional: gather packages and tools into a fresh output tree,
//! record its hashes (splitting only oversized payload blobs), then write the manifest and the
//! human/machine guides. Nothing on the airlock needs to discover or download a tool.

use crate::cli::{PackArgs, VendorModeArg};
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{
    Env, MANIFEST_DIR, MANIFEST_FILE, Manifest, SCHEMA_VERSION, Source, ToolEntry, ToolInfo, Vendor,
};
use pixi_sandbox_core::shard;
use pixi_sandbox_core::tools_lock::{ToolsLock, executable_filename};
use pixi_sandbox_core::verify::{self, Linkage};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

const TOOL_NAME: &str = env!("CARGO_PKG_NAME");
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(args: PackArgs) -> Result<()> {
    let root = support::existing_dir(&args.repo_root, "--repo-root")?;
    let out = support::absolute(&args.output_dir)?;
    let payload = out.join(MANIFEST_DIR);
    let shard_limit = shard_limit_bytes(args.shard_limit_mib)?;

    if !root.join("pixi.lock").is_file() {
        bail!(
            "{} has no pixi.lock — run `pixi install` before packing",
            root.display()
        );
    }
    if out.exists() {
        bail!(
            "{} already exists — remove it first rather than packing into a stale transport",
            out.display()
        );
    }
    validate_env_names(&args.envs)?;

    // Fetching is fully pinned. The default pins are compiled into the released CLI, while an
    // explicit `--tools-lock` is a deliberately reviewable per-project override. The non-fetch
    // fallback is for a developer who consciously supplies pixi-pack/pixi/pixi-unpack on PATH.
    let lock = if args.fetch_tools {
        let lock = match args.tools_lock.as_deref() {
            Some(path) => {
                let path = if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    root.join(path)
                };
                ToolsLock::load(&path)
                    .with_context(|| format!("reading tool-pin override {}", path.display()))?
            }
            None => ToolsLock::embedded().context("loading embedded helper-tool pins")?,
        };
        Some(lock)
    } else {
        None
    };
    let cache = lock
        .is_some()
        .then(|| tools_cache(args.tools_cache.as_deref()))
        .transpose()?;

    let packer = match &lock {
        Some(lock) => {
            fetch_tool(
                lock,
                "pixi-pack",
                &args.platform,
                cache.as_deref().expect("a fetched tool always has a cache"),
            )?
            .path
        }
        None => support::find_executable("pixi-pack").ok_or_else(|| {
            anyhow::anyhow!("pixi-pack is not on PATH (pass --fetch-tools to use embedded pins)")
        })?,
    };

    fs::create_dir_all(&payload)
        .with_context(|| format!("creating transport payload at {}", payload.display()))?;

    println!(
        "pack environments: {} · {}",
        args.envs.join(", "),
        args.platform
    );
    let mut envs = BTreeMap::new();
    for name in &args.envs {
        let target = payload.join("envs").join(name).join("pack");
        let parent = target
            .parent()
            .expect("pack target always has an env parent");
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

        let mut command = Command::new(&packer);
        command
            .current_dir(&root)
            .arg(&root)
            .arg("-e")
            .arg(name)
            .arg("-p")
            .arg(&args.platform)
            .arg("-o")
            .arg(&target)
            .arg("--directory-only");
        let output =
            support::run(&mut command).with_context(|| format!("pixi-pack environment {name}"))?;

        let files = shard::files_under(&target)
            .with_context(|| format!("reading pixi-pack output for environment {name}"))?;
        if files.is_empty() {
            bail!("pixi-pack produced no files for environment {name}");
        }
        let packed_size = sum_files(&files)?;
        // pixi-pack 0.7 prints an unpacked-size hint on some releases but not all. The
        // installed prefix is authoritative when it is available, and gives restore a useful
        // disk-space estimate instead of recording a misleading zero.
        let unpacked_size = parse_unpacked_size(&output)
            .or(installed_environment_size(&root, name)?)
            .unwrap_or(0);
        println!(
            "  {name}: {} files · {} MiB packed · {} MiB unpacked",
            files.len(),
            support::mib(packed_size),
            support::mib(unpacked_size)
        );

        envs.insert(
            name.clone(),
            Env {
                platform: args.platform.clone(),
                pack_path: format!("{MANIFEST_DIR}/envs/{name}/pack"),
                packed_size_bytes: packed_size,
                unpacked_size_bytes: unpacked_size,
                pixi_environment_fingerprint: fingerprint_of(&root, name),
                blobs: Vec::new(),
            },
        );
    }

    println!("embed tools");
    let mut tools = BTreeMap::new();
    for name in ["pixi", "pixi-unpack"] {
        let source = match &lock {
            Some(lock) => {
                let fetched = fetch_tool(
                    lock,
                    name,
                    &args.platform,
                    cache.as_deref().expect("a fetched tool always has a cache"),
                )?;
                ToolSource {
                    path: fetched.path,
                    version: fetched.version,
                    url: Some(fetched.url),
                    pinned_sha256: Some(fetched.sha256),
                }
            }
            None => {
                let path = support::find_executable(name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "{name} is not on PATH (pass --fetch-tools to use embedded pins)"
                    )
                })?;
                ToolSource {
                    version: reported_version(&path)?,
                    path,
                    url: None,
                    pinned_sha256: None,
                }
            }
        };
        let entry = embed_tool(&payload, &args.platform, name, &source, shard_limit)?;
        println!(
            "  {name} {} · {} · {} MiB",
            entry.version,
            entry.linkage,
            support::mib(entry.size_bytes)
        );
        tools.insert(name.to_string(), entry);
    }

    if let Some(self_bin) = &args.self_bin {
        let source = support::absolute(self_bin)?;
        if !source.is_file() {
            bail!("--self-bin is not a file: {}", source.display());
        }
        let entry = embed_tool(
            &payload,
            &args.platform,
            "pixi-sandbox",
            &ToolSource {
                path: source,
                version: TOOL_VERSION.to_string(),
                url: None,
                pinned_sha256: None,
            },
            shard_limit,
        )?;
        println!(
            "  pixi-sandbox {} · {} · {} MiB",
            entry.version,
            entry.linkage,
            support::mib(entry.size_bytes)
        );
        tools.insert("pixi-sandbox".to_string(), entry);
    }

    let (mut vendor, vendor_info) = if args.cargo_vendor {
        println!("vendor cargo dependencies");
        vendor_tree(&root, &out, &payload, args.cargo_vendor_mode)?
    } else {
        (None, None)
    };

    println!("record manifest and shard oversized files");
    for (name, environment) in &mut envs {
        let root = payload.join("envs").join(name).join("pack");
        environment.blobs = record_tree(&payload, &root, shard_limit)?;
    }
    if let Some(vendor) = &mut vendor {
        let root = payload.join("vendor");
        vendor.blobs = record_tree(&payload, &root, shard_limit)?;
    }

    let manifest = Manifest {
        schema: SCHEMA_VERSION,
        tool: ToolInfo {
            name: TOOL_NAME.to_string(),
            version: TOOL_VERSION.to_string(),
        },
        created_at: support::now_rfc3339(),
        platform: args.platform.clone(),
        shard_limit_bytes: shard_limit,
        source: Source {
            commit: git_commit(&root),
            lock_sha256: Some(shard::sha256_file(&root.join("pixi.lock"))?),
        },
        tools,
        envs,
        vendor,
    };
    manifest.validate()?;

    let manifest_path = payload.join(MANIFEST_FILE);
    let encoded = serde_json::to_vec_pretty(&manifest).context("serialising manifest")?;
    fs::write(&manifest_path, encoded)
        .with_context(|| format!("writing {}", manifest_path.display()))?;
    // Keep text files predictable in tools that expect a final newline.
    let mut manifest_text = fs::read_to_string(&manifest_path)?;
    manifest_text.push('\n');
    fs::write(&manifest_path, manifest_text)?;

    write_branch_docs(&out, &manifest, vendor_info.as_ref())?;

    let recorded = manifest
        .envs
        .values()
        .map(|env| env.blobs.len())
        .sum::<usize>()
        + manifest.tools.len()
        + manifest
            .vendor
            .as_ref()
            .map_or(0, |vendor| vendor.blobs.len());
    let files = shard::files_under(&payload)?;
    let (env_bytes, tool_bytes, vendor_bytes) = manifest.payload_split();
    println!(
        "packed transport: {} blobs · {} files · envs {} MiB · tools {} MiB · vendor {} MiB",
        recorded,
        files.len(),
        support::mib(env_bytes),
        support::mib(tool_bytes),
        support::mib(vendor_bytes)
    );
    println!("  manifest: {}", manifest_path.display());
    println!(
        "  next: pixi-sandbox publish --input-dir {} --branch-name <name>",
        out.display()
    );
    Ok(())
}

#[derive(Debug)]
struct ToolSource {
    path: PathBuf,
    version: String,
    url: Option<String>,
    pinned_sha256: Option<String>,
}

#[derive(Debug)]
struct FetchedTool {
    path: PathBuf,
    version: String,
    url: String,
    sha256: String,
}

#[derive(Debug)]
struct VendorInfo {
    cargo: String,
    rustc: String,
}

fn shard_limit_bytes(mebibytes: f64) -> Result<u64> {
    if !mebibytes.is_finite() || mebibytes <= 0.0 {
        bail!("--shard-limit-mib must be a positive finite number");
    }
    let bytes = mebibytes * 1024.0 * 1024.0;
    if bytes > u64::MAX as f64 {
        bail!("--shard-limit-mib is too large");
    }
    Ok(bytes as u64)
}

fn validate_env_names(envs: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for name in envs {
        let path = Path::new(name);
        if name.is_empty()
            || name == "."
            || name == ".."
            || path.is_absolute()
            || path.components().count() != 1
        {
            bail!("unsafe environment name {name:?}");
        }
        if !seen.insert(name) {
            bail!("environment {name:?} was requested more than once");
        }
    }
    Ok(())
}

fn tools_cache(explicit: Option<&Path>) -> Result<PathBuf> {
    match explicit {
        Some(path) => support::absolute(path),
        None => {
            let home = env::var_os("HOME")
                .or_else(|| env::var_os("USERPROFILE"))
                .ok_or_else(|| anyhow::anyhow!("cannot choose a tools cache: HOME is not set"))?;
            Ok(PathBuf::from(home)
                .join(".cache")
                .join("pixi-sandbox")
                .join("tools"))
        }
    }
}

fn fetch_tool(lock: &ToolsLock, name: &str, platform: &str, cache: &Path) -> Result<FetchedTool> {
    let tool = lock
        .tools
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("{name} is not pinned in the tools lock"))?;
    let pin = lock
        .pin(name, platform)
        .ok_or_else(|| anyhow::anyhow!("{name} has no pin for platform {platform}"))?;
    let url = lock
        .url(name, platform)
        .expect("pin and tool were checked above");

    fs::create_dir_all(cache)
        .with_context(|| format!("creating tools cache {}", cache.display()))?;
    let cached_name = executable_filename(&format!("{name}-{}-{platform}", tool.version), platform);
    let destination = cache.join(cached_name);
    let cached = destination.is_file()
        && shard::sha256_file(&destination)
            .map(|actual| actual == pin.sha256)
            .unwrap_or(false);

    if !cached {
        println!("  fetch {name} {} ({})", tool.version, pin.target);
        let temporary = destination.with_file_name(format!(
            ".{}.download-{}",
            destination
                .file_name()
                .and_then(|file| file.to_str())
                .unwrap_or(name),
            std::process::id()
        ));
        support::remove_path(&temporary)?;
        let result = (|| -> Result<()> {
            let response = ureq::get(&url)
                .call()
                .map_err(|error| anyhow::anyhow!("downloading {name} from {url}: {error}"))?;
            let mut source = response.into_reader();
            let mut output = File::create(&temporary)
                .with_context(|| format!("creating {}", temporary.display()))?;
            io::copy(&mut source, &mut output)
                .with_context(|| format!("writing download for {name}"))?;
            drop(output);
            let actual = shard::sha256_file(&temporary)?;
            if actual != pin.sha256 {
                bail!(
                    "integrity: {name} from {url} does not match the selected tool pins (expected {}, got {actual})",
                    pin.sha256
                );
            }
            support::remove_path(&destination)?;
            fs::rename(&temporary, &destination).with_context(|| {
                format!(
                    "moving verified {name} into the tools cache at {}",
                    destination.display()
                )
            })?;
            Ok(())
        })();
        if result.is_err() {
            let _ = support::remove_path(&temporary);
        }
        result?;
    }

    support::make_executable(&destination)?;
    let reported = reported_version(&destination)?;
    if !reported.contains(&tool.version) {
        bail!(
            "{name}: selected tool pins require {} but {} reports {reported:?}",
            tool.version,
            destination.display()
        );
    }
    Ok(FetchedTool {
        path: destination,
        version: tool.version.clone(),
        url,
        sha256: pin.sha256.clone(),
    })
}

fn reported_version(path: &Path) -> Result<String> {
    let mut command = Command::new(path);
    command.arg("--version");
    let output = support::run(&mut command)
        .with_context(|| format!("asking {} for its version", path.display()))?;
    Ok(output
        .split_whitespace()
        .last()
        .unwrap_or("unknown")
        .to_string())
}

fn embed_tool(
    payload: &Path,
    platform: &str,
    name: &str,
    source: &ToolSource,
    shard_limit: u64,
) -> Result<ToolEntry> {
    let file_name = executable_filename(name, platform);
    let relative = format!("tools/{platform}/{file_name}");
    let destination = payload.join(&relative);
    let parent = destination
        .parent()
        .expect("tool path always has a parent directory");
    fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    fs::copy(&source.path, &destination).with_context(|| {
        format!(
            "copying {name} from {} to {}",
            source.path.display(),
            destination.display()
        )
    })?;
    support::make_executable(&destination)?;

    let metadata = fs::metadata(&destination)?;
    if metadata.len() > shard_limit {
        bail!(
            "embedded tool {name} is {} bytes, above the {} byte shard limit; tools cannot be split",
            metadata.len(),
            shard_limit
        );
    }
    let actual = shard::sha256_file(&destination)?;
    if let Some(expected) = &source.pinned_sha256 {
        if &actual != expected {
            bail!(
                "integrity: embedded {name} does not match its pin (expected {expected}, got {actual})"
            );
        }
    }

    let linkage = verify::linkage_of(&destination);
    if linkage == Linkage::Dynamic {
        bail!("{name} is dynamically linked — ship the static release asset instead (decision D4)");
    }
    Ok(ToolEntry {
        version: source.version.clone(),
        url: source.url.clone(),
        pinned_sha256: source.pinned_sha256.clone(),
        linkage: linkage.as_str().to_string(),
        size_bytes: metadata.len(),
        path: Some(relative),
    })
}

fn vendor_tree(
    root: &Path,
    out: &Path,
    payload: &Path,
    mode: VendorModeArg,
) -> Result<(Option<Vendor>, Option<VendorInfo>)> {
    let cargo_lock = root.join("Cargo.lock");
    if !cargo_lock.is_file() {
        bail!(
            "--cargo-vendor needs a Cargo.lock at {}",
            cargo_lock.display()
        );
    }

    let vendor_dir = payload.join("vendor");
    let source_dir = match mode {
        VendorModeArg::Loose => vendor_dir.clone(),
        VendorModeArg::Tarballs => out.join(".cargo-vendor-tmp"),
    };
    support::remove_path(&source_dir)?;
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .arg("vendor")
        .arg("--locked")
        .arg("--versioned-dirs")
        .arg(&source_dir);
    support::run(&mut command).context("running cargo vendor")?;

    let crates = crate_directories(&source_dir)?;
    if crates.is_empty() {
        bail!("cargo vendor produced no crate directories");
    }
    if matches!(mode, VendorModeArg::Tarballs) {
        fs::create_dir_all(&vendor_dir)
            .with_context(|| format!("creating {}", vendor_dir.display()))?;
        for crate_dir in &crates {
            let name = crate_dir.file_name().ok_or_else(|| {
                anyhow::anyhow!("vendor directory without a name: {}", crate_dir.display())
            })?;
            let archive_path = vendor_dir.join(format!("{}.tar", name.to_string_lossy()));
            let archive = File::create(&archive_path)
                .with_context(|| format!("creating {}", archive_path.display()))?;
            let mut builder = tar::Builder::new(archive);
            builder
                .append_dir_all(name, crate_dir)
                .with_context(|| format!("archiving {}", crate_dir.display()))?;
            builder
                .finish()
                .with_context(|| format!("finishing {}", archive_path.display()))?;
        }
        support::remove_path(&source_dir)?;
    }

    let files = shard::files_under(&vendor_dir)?;
    let size = sum_files(&files)?;
    let mode_name = match mode {
        VendorModeArg::Loose => "loose",
        VendorModeArg::Tarballs => "tarballs",
    };
    println!(
        "  {} crates · {} files · {} MiB · {mode_name}",
        crates.len(),
        files.len(),
        support::mib(size)
    );
    Ok((
        Some(Vendor {
            mode: mode_name.to_string(),
            crates: crates.len() as u64,
            size_bytes: size,
            cargo_lock_sha256: Some(shard::sha256_file(&cargo_lock)?),
            directory: Some(format!("{MANIFEST_DIR}/vendor")),
            blobs: Vec::new(),
        }),
        Some(VendorInfo {
            cargo: command_version("cargo").unwrap_or_else(|_| "cargo (unknown)".to_string()),
            rustc: command_version("rustc").unwrap_or_else(|_| "rustc (unknown)".to_string()),
        }),
    ))
}

fn crate_directories(root: &Path) -> Result<Vec<PathBuf>> {
    let mut directories = fs::read_dir(root)
        .with_context(|| format!("reading cargo vendor output {}", root.display()))?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_dir())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    directories.sort();
    Ok(directories)
}

fn command_version(program: &str) -> Result<String> {
    let mut command = Command::new(program);
    command.arg("--version");
    Ok(support::run(&mut command)?.trim().to_string())
}

fn record_tree(
    payload: &Path,
    root: &Path,
    shard_limit: u64,
) -> Result<Vec<pixi_sandbox_core::manifest::Blob>> {
    let mut blobs = Vec::new();
    for path in shard::files_under(root)? {
        let relative = path
            .strip_prefix(payload)
            .with_context(|| format!("{} is outside {}", path.display(), payload.display()))?
            .to_string_lossy()
            .replace('\\', "/");
        blobs.push(shard::record_file(payload, &relative, shard_limit)?);
    }
    Ok(blobs)
}

fn sum_files(paths: &[PathBuf]) -> Result<u64> {
    paths.iter().try_fold(0u64, |sum, path| {
        let size = fs::metadata(path)
            .with_context(|| format!("reading metadata for {}", path.display()))?
            .len();
        sum.checked_add(size)
            .ok_or_else(|| anyhow::anyhow!("size overflow while reading {}", path.display()))
    })
}

fn installed_environment_size(root: &Path, env: &str) -> Result<Option<u64>> {
    let prefix = root.join(".pixi").join("envs").join(env);
    if !prefix.is_dir() {
        return Ok(None);
    }
    Ok(Some(sum_files(&shard::files_under(&prefix)?)?))
}

fn fingerprint_of(root: &Path, env: &str) -> Option<String> {
    let marker = root
        .join(".pixi")
        .join("envs")
        .join(env)
        .join("conda-meta")
        .join(".pixi-environment-fingerprint");
    fs::read_to_string(marker)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn git_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty())
}

fn parse_unpacked_size(output: &str) -> Option<u64> {
    for line in output.lines() {
        if !line.to_ascii_lowercase().contains("unpacked") {
            continue;
        }
        let tokens = line.split_whitespace().collect::<Vec<_>>();
        for (index, token) in tokens.iter().enumerate() {
            let unit = token.trim_matches(|character: char| !character.is_ascii_alphabetic());
            let scale = match unit.to_ascii_lowercase().as_str() {
                "gib" => 1024_f64.powi(3),
                "mib" => 1024_f64.powi(2),
                "kib" => 1024_f64,
                "b" => 1.0,
                _ => continue,
            };
            let number = tokens
                .get(index.checked_sub(1)?)?
                .trim_matches(|character: char| !(character.is_ascii_digit() || character == '.'));
            if let Ok(number) = number.parse::<f64>() {
                return Some((number * scale) as u64);
            }
        }
    }
    None
}

fn write_branch_docs(
    out: &Path,
    manifest: &Manifest,
    vendor_info: Option<&VendorInfo>,
) -> Result<()> {
    let rows = manifest
        .envs
        .iter()
        .map(|(name, env)| {
            format!(
                "| `{name}` | {} | {} MiB | {} MiB | {} |",
                env.platform,
                support::mib(env.packed_size_bytes),
                support::mib(env.unpacked_size_bytes),
                env.blobs.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let commit = manifest.source.commit.as_deref().unwrap_or("unknown");
    let lock = manifest.source.lock_sha256.as_deref().unwrap_or("unknown");
    let pixi_file = executable_filename("pixi", &manifest.platform);
    let vendor = manifest
        .vendor
        .as_ref()
        .map(|vendor| {
            let toolchain = vendor_info.map_or_else(
                || "unknown cargo/rustc".to_string(),
                |info| format!("{}; {}", info.cargo, info.rustc),
            );
            format!(
                "\nCargo dependencies: **{} crates**, {} MiB ({}) from `Cargo.lock` sha256 `{}…`; \
             restore materialises them to `.pixi-sandbox/vendor/`. Built with {toolchain}.\n",
                vendor.crates,
                support::mib(vendor.size_bytes),
                vendor.mode,
                vendor
                    .cargo_lock_sha256
                    .as_deref()
                    .unwrap_or("unknown")
                    .chars()
                    .take(12)
                    .collect::<String>(),
            )
        })
        .unwrap_or_default();

    let readme = format!(
        "# Offline sandbox (orphan branch)\n\n\
         Built {} from commit `{commit}` for platform `{}`.\n\
         `pixi.lock` sha256 `{lock}`.\n\n\
         | env | platform | packed | unpacked | files |\n\
         | --- | --- | ---: | ---: | ---: |\n\
         {rows}\n\
         {vendor}\n\
         ## Restore on the disconnected machine\n\n\
         ```bash\n\
         pixi-sandbox restore --branch-location <extracted-branch> \\\n                              --path-to-main-repo-code .\n\
         # then, with no network:\n\
         .pixi/tools/{}/{pixi_file} install --frozen --offline\n\
         source .pixi/sandbox-env.sh\n\
         ```\n\n\
         Every manifest blob is verified before it is written into the working tree.\n",
        manifest.created_at, manifest.platform, manifest.platform
    );
    fs::write(out.join("README.md"), readme)
        .with_context(|| format!("writing {}/README.md", out.display()))?;

    let agents = format!(
        "# AGENTS.md — machine instructions for this bundle\n\n\
         This is an **offline pixi sandbox**, not source code to merge.\n\n\
         - authoritative manifest: `.pixi-sandbox/manifest.json` (schema {});\n\
         - environments: {} (platform {});\n\
         - restore with `pixi-sandbox restore --branch-location <dir> --path-to-main-repo-code <project>`;\n\
         - never download tools at restore time; bundled tools are: {};\n\
         - after restore, `.pixi/tools/{}/{pixi_file} install --frozen --offline` must be a no-op.\n",
        manifest.schema,
        manifest.envs.keys().cloned().collect::<Vec<_>>().join(", "),
        manifest.platform,
        manifest
            .tools
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        manifest.platform,
    );
    fs::write(out.join("AGENTS.md"), agents)
        .with_context(|| format!("writing {}/AGENTS.md", out.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{ToolSource, embed_tool};
    use std::fs;

    #[test]
    fn windows_transport_tools_keep_the_exe_suffix() {
        let directory = tempfile::tempdir().expect("temporary payload");
        let source = directory.path().join("source.exe");
        // `MZ` is deliberately recognised as a native/system executable by the linkage probe.
        fs::write(&source, b"MZ test binary").expect("write test executable");
        let entry = embed_tool(
            directory.path(),
            "win-64",
            "pixi",
            &ToolSource {
                path: source,
                version: "test".to_string(),
                url: None,
                pinned_sha256: None,
            },
            u64::MAX,
        )
        .expect("embed Windows tool");

        assert_eq!(entry.path.as_deref(), Some("tools/win-64/pixi.exe"));
        assert!(directory.path().join("tools/win-64/pixi.exe").is_file());
    }
}
