//! `pack` — build a self-contained transport on the connected side.
//!
//! The ordering below is intentional: gather packages and tools into a fresh output tree,
//! record its hashes (splitting only oversized payload blobs), then write the manifest and the
//! human/machine guides. Nothing on the airlock needs to discover or download a tool.

use crate::cli::{PackArgs, VendorModeArg};
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::{
    Env, EnvFiles, MANIFEST_DIR, MANIFEST_FILE, Manifest, SCHEMA_VERSION, Source, ToolEntry,
    ToolInfo, Vendor,
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
    crate::diagnostics::phase(
        "validate-plan",
        "checking lockfiles, paths, and environment names",
    );
    let plan = PackPlan::from_args(args)?;
    crate::diagnostics::phase("resolve-tools", "resolving pixi-pack and pixi-unpack");
    let tools = plan.resolve_tools()?;
    crate::diagnostics::phase(
        "helpers-ready",
        format!(
            "pixi-pack={} pixi-unpack={}",
            tools.packer.display(),
            tools.unpacker.display()
        ),
    );
    crate::diagnostics::phase(
        "assemble",
        "packing environments, tools, and optional Cargo vendor",
    );
    let artifacts = plan.assemble_artifacts(&tools)?;
    crate::diagnostics::phase("write-manifest", "recording hashes and transport guides");
    plan.write_manifest_and_docs(artifacts)
}

/// Everything `run` decides before any output exists: path resolution plus the guards that
/// must refuse before a transport directory is created (design.md §11) — a missing
/// `pixi.lock`, a stale `--output-dir`, an unsafe or duplicate `--envs` name, or (when
/// `--cargo-vendor` is set) a `Cargo.lock` cargo cannot vendor. Named `PackPlan`, not
/// `PackArgs`: by the time [`PackPlan::from_args`] returns, every field has been validated or
/// resolved to an absolute path, which is what makes the later steps infallible on these
/// inputs.
#[derive(Debug)]
struct PackPlan {
    args: PackArgs,
    root: PathBuf,
    out: PathBuf,
    payload: PathBuf,
    shard_limit: u64,
}

/// The tools one `pack` run executes: the packer/unpacker invoked for every environment, plus
/// whatever pinned [`ToolsLock`] and download cache back them when `--fetch-tools` is set. A
/// `None` lock means `--fetch-tools` was not asked for — `packer`/`unpacker` were resolved from
/// PATH instead, and every tool [`PackPlan::assemble_artifacts`] embeds carries no provenance
/// (its `url`/`pinned_sha256` are `None`).
#[derive(Debug)]
struct ResolvedTools {
    lock: Option<ToolsLock>,
    cache: Option<PathBuf>,
    packer: PathBuf,
    unpacker: PathBuf,
}

/// What a `pack` run actually produces before anything is written: the per-environment file
/// oracle (D13), every embedded tool (including an optional `--self-bin`), and the vendored
/// cargo tree. Kept distinct from [`PackPlan::write_manifest_and_docs`] so the artifacts exist
/// as plain data before the manifest is assembled around them.
struct PackedArtifacts {
    envs: BTreeMap<String, Env>,
    tools: BTreeMap<String, ToolEntry>,
    vendor: Option<Vendor>,
    vendor_info: Option<VendorInfo>,
}

impl PackPlan {
    fn from_args(args: PackArgs) -> Result<Self> {
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

        // Before anything is created: a lockfile cargo cannot vendor is a property of the
        // project, not of this run, and it is the one `--cargo-vendor` failure an operator
        // cannot act on without being told what to change (design.md §11). Detecting it here
        // rather than inside the vendor step means a refused pack leaves no output directory
        // behind — otherwise the next attempt fails on the stale-transport guard and the real
        // error is one run in the dark.
        if args.cargo_vendor {
            validate_vendorable_lockfile(&root)?;
        }

        Ok(Self {
            args,
            root,
            out,
            payload,
            shard_limit,
        })
    }

    /// Fetching is fully pinned. The default pins are compiled into the released CLI, while an
    /// explicit `--tools-lock` is a deliberately reviewable per-project override. The
    /// non-fetch fallback is for a developer who consciously supplies
    /// pixi-pack/pixi/pixi-unpack on PATH.
    fn resolve_tools(&self) -> Result<ResolvedTools> {
        let lock = if self.args.fetch_tools {
            let lock = match self.args.tools_lock.as_deref() {
                Some(path) => {
                    let path = if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        self.root.join(path)
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
            .then(|| tools_cache(self.args.tools_cache.as_deref()))
            .transpose()?;

        let packer = match &lock {
            Some(lock) => {
                fetch_tool(
                    lock,
                    "pixi-pack",
                    &self.args.platform,
                    cache.as_deref().expect("a fetched tool always has a cache"),
                )?
                .path
            }
            None => support::find_executable("pixi-pack").ok_or_else(|| {
                anyhow::anyhow!(
                    "pixi-pack is not on PATH (pass --fetch-tools to use embedded pins)"
                )
            })?,
        };
        // The per-file oracle (D13) is built by unpacking each environment once more, with the
        // same pinned unpacker the airlock will use, so it must be resolved up here rather than
        // in the embed step below. The embed step re-resolves it from the same verified cache.
        let unpacker = match &lock {
            Some(lock) => {
                fetch_tool(
                    lock,
                    "pixi-unpack",
                    &self.args.platform,
                    cache.as_deref().expect("a fetched tool always has a cache"),
                )?
                .path
            }
            None => support::find_executable("pixi-unpack").ok_or_else(|| {
                anyhow::anyhow!(
                    "pixi-unpack is not on PATH (pass --fetch-tools to use embedded pins)"
                )
            })?,
        };

        Ok(ResolvedTools {
            lock,
            cache,
            packer,
            unpacker,
        })
    }

    /// Pack every requested environment, build its file oracle, embed the transport tools (and
    /// an optional `--self-bin`), and vendor cargo dependencies when asked. Nothing here is
    /// written to the manifest yet — [`PackPlan::write_manifest_and_docs`] owns that.
    fn assemble_artifacts(&self, tools: &ResolvedTools) -> Result<PackedArtifacts> {
        fs::create_dir_all(&self.payload)
            .with_context(|| format!("creating transport payload at {}", self.payload.display()))?;

        println!(
            "pack environments: {} · {}",
            self.args.envs.join(", "),
            self.args.platform
        );
        let mut envs = BTreeMap::new();
        for name in &self.args.envs {
            let target = self.payload.join("envs").join(name).join("pack");
            let parent = target
                .parent()
                .expect("pack target always has an env parent");
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

            let mut command = Command::new(&tools.packer);
            command
                .current_dir(&self.root)
                .arg(&self.root)
                .arg("-e")
                .arg(name)
                .arg("-p")
                .arg(&self.args.platform)
                .arg("-o")
                .arg(&target)
                .arg("--directory-only");
            support::run(&mut command).with_context(|| format!("pixi-pack environment {name}"))?;

            let files = shard::files_under(&target)
                .with_context(|| format!("reading pixi-pack output for environment {name}"))?;
            if files.is_empty() {
                bail!("pixi-pack produced no files for environment {name}");
            }
            let packed_size = sum_files(&files)?;

            // The oracle: unpack the pack once more (a copy — pixi-unpack writes a cache into
            // the pack dir it reads from) and record every file of the tree the airlock will
            // actually get. This is also the honest `unpacked_size_bytes`: measured on the
            // unpacked tree, not parsed from a version-dependent log line.
            let (oracle, unpacked_size) = build_files_oracle(
                &self.out,
                &self.payload,
                name,
                &target,
                &tools.unpacker,
                self.shard_limit,
            )
            .with_context(|| format!("building the per-file oracle for environment {name}"))?;
            println!(
                "  {name}: {} files · {} MiB packed · {} MiB unpacked · {} file entries recorded",
                files.len(),
                support::mib(packed_size),
                support::mib(unpacked_size),
                oracle.entries
            );

            envs.insert(
                name.clone(),
                Env {
                    platform: self.args.platform.clone(),
                    pack_path: format!("{MANIFEST_DIR}/envs/{name}/pack"),
                    packed_size_bytes: packed_size,
                    unpacked_size_bytes: unpacked_size,
                    pixi_environment_fingerprint: fingerprint_of(&self.root, name),
                    blobs: Vec::new(),
                    files: Some(oracle),
                },
            );
        }

        println!("embed tools");
        let mut entries = BTreeMap::new();
        for name in ["pixi", "pixi-unpack"] {
            let source = match &tools.lock {
                Some(lock) => {
                    let fetched = fetch_tool(
                        lock,
                        name,
                        &self.args.platform,
                        tools
                            .cache
                            .as_deref()
                            .expect("a fetched tool always has a cache"),
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
            let entry = embed_tool(
                &self.payload,
                &self.args.platform,
                name,
                &source,
                self.shard_limit,
            )?;
            println!(
                "  {name} {} · {} · {} MiB",
                entry.version,
                entry.linkage,
                support::mib(entry.size_bytes)
            );
            entries.insert(name.to_string(), entry);
        }

        if let Some(self_bin) = &self.args.self_bin {
            let source = support::absolute(self_bin)?;
            if !source.is_file() {
                bail!("--self-bin is not a file: {}", source.display());
            }
            // Structural fast-path (issue #81): the shapes whose brokenness path provenance
            // already decides — a `pixi global` trampoline, a managed launcher script — are
            // refused before anything is copied, with the remedy that packs a runnable binary.
            if let Some(refusal) = pixi_sandbox::standalone::pack_refusal_for_ownership(&source) {
                bail!("{refusal}");
            }
            let entry = embed_tool(
                &self.payload,
                &self.args.platform,
                "pixi-sandbox",
                &ToolSource {
                    path: source,
                    version: TOOL_VERSION.to_string(),
                    url: None,
                    pinned_sha256: None,
                },
                self.shard_limit,
            )?;
            println!(
                "  pixi-sandbox {} · {} · {} MiB",
                entry.version,
                entry.linkage,
                support::mib(entry.size_bytes)
            );

            // The behavioural oracle: execute exactly the bytes that will ship, under an empty
            // environment. A self-bin's content is not pinned into the manifest (only its size
            // is), and a static trampoline passes the linkage check — so neither declared check
            // can see a launcher that dies without its global prefix; only running it can.
            // Cross-platform packs skip with a reason: probing foreign bytes would be a lie,
            // and doctor on the target host is the same guard there.
            if pixi_sandbox::standalone::host_platform() == Some(self.args.platform.as_str()) {
                let Some(relative) = entry.path.clone() else {
                    bail!("embedded pixi-sandbox recorded no tool path");
                };
                let embedded_path = self.payload.join(relative);
                let anchor = self.out.join(".pixi-sandbox-standalone-probe");
                let outcome = fs::create_dir_all(anchor.join("tmp"))
                    .with_context(|| format!("creating {}", anchor.display()))
                    .and_then(|()| {
                        pixi_sandbox::standalone::probe(
                            &embedded_path,
                            &anchor,
                            &pixi_sandbox::standalone::CommandRunner::new(),
                        )
                        .map_err(|refusal| anyhow::anyhow!(refusal.render(&embedded_path)))
                    });
                support::remove_path(&anchor)?;
                outcome?;
                println!("  self-bin: runs standalone (--version, empty environment)");
            } else {
                println!(
                    "  self-bin: standalone probe skipped — packing {} on a {} host; doctor --verify on the target host is the guard",
                    self.args.platform,
                    pixi_sandbox::standalone::host_platform().unwrap_or("unsupported"),
                );
            }
            entries.insert("pixi-sandbox".to_string(), entry);
        }

        let (vendor, vendor_info) = if self.args.cargo_vendor {
            println!("vendor cargo dependencies");
            vendor_tree(
                &self.root,
                &self.out,
                &self.payload,
                self.args.cargo_vendor_mode,
            )?
        } else {
            (None, None)
        };

        Ok(PackedArtifacts {
            envs,
            tools: entries,
            vendor,
            vendor_info,
        })
    }

    /// Shard oversized blobs, assemble and validate the manifest, write it (and the branch
    /// docs built from it) under `--output-dir`, and print the summary an operator reads.
    fn write_manifest_and_docs(&self, artifacts: PackedArtifacts) -> Result<()> {
        let PackedArtifacts {
            mut envs,
            tools,
            mut vendor,
            vendor_info,
        } = artifacts;

        println!("record manifest and shard oversized files");
        for (name, environment) in &mut envs {
            let root = self.payload.join("envs").join(name).join("pack");
            environment.blobs = record_tree(&self.payload, &root, self.shard_limit)?;
        }
        if let Some(vendor) = &mut vendor {
            let root = self.payload.join("vendor");
            vendor.blobs = record_tree(&self.payload, &root, self.shard_limit)?;
        }

        let manifest = Manifest {
            schema: SCHEMA_VERSION,
            tool: ToolInfo {
                name: TOOL_NAME.to_string(),
                version: TOOL_VERSION.to_string(),
            },
            created_at: support::now_rfc3339(),
            platform: self.args.platform.clone(),
            shard_limit_bytes: self.shard_limit,
            source: Source {
                commit: pixi_sandbox_git::current_commit(&self.root),
                lock_sha256: Some(shard::sha256_file(&self.root.join("pixi.lock"))?),
            },
            tools,
            envs,
            vendor,
        };
        manifest.validate()?;

        let manifest_path = self.payload.join(MANIFEST_FILE);
        let encoded = serde_json::to_vec_pretty(&manifest).context("serialising manifest")?;
        fs::write(&manifest_path, encoded)
            .with_context(|| format!("writing {}", manifest_path.display()))?;
        // Keep text files predictable in tools that expect a final newline.
        let mut manifest_text = fs::read_to_string(&manifest_path)?;
        manifest_text.push('\n');
        fs::write(&manifest_path, manifest_text)?;

        write_branch_docs(&self.out, &manifest, vendor_info.as_ref())?;

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
        let files = shard::files_under(&self.payload)?;
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
            self.out.display()
        );
        Ok(())
    }
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

/// Build the per-file oracle for one environment (D13): unpack the finished pack with the
/// pinned unpacker — exactly what `restore` will do on the airlock — and record the tree it
/// produces, with every prefix-path spelling canonicalised away (see `files_manifest`).
///
/// The unpack runs on a *copy* of the pack: pixi-unpack writes its extraction cache into the
/// pack directory it reads from, and the payload tree must stay exactly what pixi-pack
/// produced. The scratch lives inside `out` (never `/tmp`, invariant 3) and is removed before
/// returning, so a successful pack leaves no trace of it.
fn build_files_oracle(
    out: &Path,
    payload: &Path,
    env: &str,
    pack: &Path,
    unpacker: &Path,
    shard_limit: u64,
) -> Result<(EnvFiles, u64)> {
    let scratch = out.join(format!(".pixi-sandbox-verify-{env}"));
    support::remove_path(&scratch)?;
    let pack_copy = scratch.join("pack");
    let stage = scratch.join("stage");
    fs::create_dir_all(scratch.join("tmp"))
        .with_context(|| format!("creating {}", scratch.join("tmp").display()))?;
    fs::create_dir_all(&pack_copy).with_context(|| format!("creating {}", pack_copy.display()))?;
    fs::create_dir_all(&stage).with_context(|| format!("creating {}", stage.display()))?;

    for file in shard::files_under(pack)? {
        let relative = file
            .strip_prefix(pack)
            .with_context(|| format!("{} is outside {}", file.display(), pack.display()))?;
        let destination = pack_copy.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
        fs::copy(&file, &destination)
            .with_context(|| format!("copying {} for the verification unpack", file.display()))?;
    }

    let mut command = Command::new(unpacker);
    command
        .arg(&pack_copy)
        .arg("-o")
        .arg(&stage)
        .arg("-e")
        .arg(env);
    support::use_work_tmp(&mut command, &scratch);
    support::run(&mut command).with_context(|| {
        format!(
            "verification-unpacking environment {env} with {}",
            unpacker.display()
        )
    })?;

    let prefix = stage.join(env);
    if !prefix.is_dir() {
        bail!(
            "the verification unpack completed but did not create {}",
            prefix.display()
        );
    }

    // The paths this side must neutralise: the stage prefix the unpacker stamped in, and the
    // pack copy it installed from — in both literal and canonical form, so a symlinked
    // scratch directory cannot defeat the canonicalisation.
    let mut candidates: Vec<Vec<u8>> = Vec::new();
    let mut push_candidate = |path: &Path| {
        let bytes = path.to_string_lossy().into_owned().into_bytes();
        if !bytes.is_empty() && !candidates.contains(&bytes) {
            candidates.push(bytes);
        }
    };
    push_candidate(&prefix);
    if let Ok(canonical) = prefix.canonicalize() {
        push_candidate(&canonical);
    }
    push_candidate(&pack_copy);
    if let Ok(canonical) = pack_copy.canonicalize() {
        push_candidate(&canonical);
    }

    let (doc, unpacked_bytes) =
        pixi_sandbox_core::files_manifest::scan_prefix(&prefix, &candidates)
            .context("scanning the verification-unpacked environment")?;
    support::remove_path(&scratch)?;

    let relative = pixi_sandbox_core::files_manifest::list_rel_path(env);
    let list_path = payload.join(&relative);
    let encoded = doc.to_bytes()?;
    fs::write(&list_path, &encoded).with_context(|| format!("writing {}", list_path.display()))?;
    let blob = shard::record_file(payload, &relative, shard_limit)
        .with_context(|| format!("recording {}", list_path.display()))?;
    Ok((
        EnvFiles {
            blob,
            entries: doc.entries() as u64,
        },
        unpacked_bytes,
    ))
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
            let mut source = response.into_body().into_reader();
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
    // Already validated in `run` before the output tree was created; re-checked here so this
    // function stays correct if it is ever called on its own.
    validate_vendorable_lockfile(root)?;

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

/// A `--cargo-vendor` preflight: the lockfile must exist, and no crate+version may be
/// reachable from two sources.
fn validate_vendorable_lockfile(root: &Path) -> Result<()> {
    let cargo_lock = root.join("Cargo.lock");
    if !cargo_lock.is_file() {
        bail!(
            "--cargo-vendor needs a Cargo.lock at {}",
            cargo_lock.display()
        );
    }
    reject_duplicate_crate_sources(&cargo_lock)
}

/// Refuse a lockfile in which one crate+version is reachable from two different sources.
///
/// `cargo vendor` cannot represent this: it maps each crate to a `<name>-<version>` directory
/// under the vendor root, so two entries for the same pair collide and cargo aborts with
/// "found duplicate version of package ... vendored from two sources" and no remedy (design.md
/// §11, known upstream). Left alone that surfaces as a pack failure on the connected side at
/// best, and at worst as a transport that packs "successfully" around a missing crate and only
/// breaks on the airlock, where cargo reports nothing useful.
///
/// The remedy is a maintainer decision, not something to guess at, so the error names the
/// crates and the two sources and states the two real ways out. Path (workspace) members have
/// no `source` in the lockfile and are not vendored, so they are ignored; two entries for the
/// same crate+version from the *same* source are a normal lockfile artefact, not this bug.
fn reject_duplicate_crate_sources(cargo_lock: &Path) -> Result<()> {
    let text = fs::read_to_string(cargo_lock)
        .with_context(|| format!("reading {}", cargo_lock.display()))?;
    let lock: toml::Value =
        toml::from_str(&text).with_context(|| format!("parsing {}", cargo_lock.display()))?;

    let Some(packages) = lock.get("package").and_then(toml::Value::as_array) else {
        // A lockfile with no [[package]] entries cannot have duplicates.
        return Ok(());
    };

    // (name, version) -> the distinct sources it is reachable from, in first-seen order.
    let mut origins: BTreeMap<(String, String), Vec<String>> = BTreeMap::new();
    for package in packages {
        let (Some(name), Some(version)) = (
            package.get("name").and_then(toml::Value::as_str),
            package.get("version").and_then(toml::Value::as_str),
        ) else {
            continue;
        };
        // No `source` means a path/workspace member: not vendored, so not a duplicate source.
        let Some(source) = package.get("source").and_then(toml::Value::as_str) else {
            continue;
        };
        let seen = origins
            .entry((name.to_string(), version.to_string()))
            .or_default();
        if !seen.iter().any(|existing| existing == source) {
            seen.push(source.to_string());
        }
    }

    let duplicates = origins
        .into_iter()
        .filter(|(_, sources)| sources.len() > 1)
        .collect::<Vec<_>>();
    if duplicates.is_empty() {
        return Ok(());
    }

    let detail = duplicates
        .iter()
        .map(|((name, version), sources)| {
            let list = sources
                .iter()
                .map(|source| format!("\n      - {source}"))
                .collect::<String>();
            format!(
                "    {name} {version} is reachable from {} sources:{list}",
                sources.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    bail!(
        "cannot vendor: {} crate(s) are reachable from more than one source in {}:\n{detail}\n\n\
         `cargo vendor` stores every crate as <name>-<version> under one vendor root, so two \
         sources for the same pair collide and it aborts with no remedy. An airlock would then \
         find a crate missing with nothing pointing at the cause.\n\
         Fix it on the connected side, before packing:\n  \
           - make the versions differ, so each source provides a distinct pair; or\n  \
           - drop one of the two dependencies, if the crate is reachable from the other \
         source anyway.\n\
         Patching the vendored tree is not a fix: the next `cargo update` reintroduces the \
         collision.",
        duplicates.len(),
        cargo_lock.display()
    );
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
    let self_file = executable_filename("pixi-sandbox", &manifest.platform);
    let has_self = manifest.tools.contains_key("pixi-sandbox");
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
    let bootstrap = if has_self {
        format!(
            "The verified self-bootstrap binary is stored at `.pixi-sandbox/tools/{}/{}`. \
             The branch root intentionally contains documentation only.\n\n",
            manifest.platform, self_file
        )
    } else {
        "This transport has no embedded self-bootstrap binary; use an installed `pixi-sandbox` to restore it.\n\n".to_string()
    };
    let (shell_language, restore_commands) = if !has_self {
        (
            if manifest.platform.starts_with("win-") {
                "powershell"
            } else {
                "bash"
            },
            "pixi-sandbox restore --branch-location <extracted-branch> --output-path <project>"
                .to_string(),
        )
    } else if manifest.platform.starts_with("win-") {
        (
            "powershell",
            format!(
                ".\\.pixi-sandbox\\tools\\{}\\{} doctor --branch-location . --verify\n.\\.pixi-sandbox\\tools\\{}\\{} restore --branch-location . --output-path <project> --force",
                manifest.platform, self_file, manifest.platform, self_file
            ),
        )
    } else {
        (
            "bash",
            format!(
                "./.pixi-sandbox/tools/{}/{} doctor --branch-location . --verify\n./.pixi-sandbox/tools/{}/{} restore --branch-location . --output-path <project> --force",
                manifest.platform, self_file, manifest.platform, self_file
            ),
        )
    };

    let readme = format!(
        "# Offline sandbox (orphan branch)\n\n\
         Built {} from commit `{}` for platform `{}`.\n\
         `pixi.lock` sha256 `{}`.\n\n\
         {}\
         | env | platform | packed | unpacked | files |\n\
         | --- | --- | ---: | ---: | ---: |\n\
         {}\n\
         {}\n\
         ## Restore on the disconnected machine\n\n\
         ```{}\n\
         {}\n\
         # then, from <project> with no network, use pixi as the sole entrypoint:\n\
         pixi install --frozen --offline\n\
         pixi run --frozen -- cargo build --offline\n\
         ```\n\n\
         Every manifest blob is verified before it is written into the working tree.\n",
        manifest.created_at,
        commit,
        manifest.platform,
        lock,
        bootstrap,
        rows,
        vendor,
        shell_language,
        restore_commands,
    );
    fs::write(out.join("README.md"), readme)
        .with_context(|| format!("writing {}/README.md", out.display()))?;

    let agents_bootstrap = if has_self {
        format!(
            "- verified bootstrap: `.pixi-sandbox/tools/{}/{}`; the branch root contains documentation only;\n",
            manifest.platform, self_file
        )
    } else {
        "- restore with an installed `pixi-sandbox`; this transport does not contain a self-bootstrap binary;\n".to_string()
    };
    let agents = format!(
        "# AGENTS.md — machine instructions for this bundle\n\n\
         This is an **offline pixi sandbox**, not source code to merge.\n\n\
         - authoritative manifest: `.pixi-sandbox/manifest.json` (schema {});\n\
         - environments: {} (platform {});\n\
         {}\
         - never download tools at restore time; bundled tools are: {};\n\
         - after restore, use pixi as the only entrypoint: `pixi install --frozen --offline` \
           (or `<project>/.pixi/tools/{}/{}` when user launchers were skipped) must be a no-op;\n\
         - no `.pixi/sandbox-env.sh` activation script is generated or supported.\n",
        manifest.schema,
        manifest.envs.keys().cloned().collect::<Vec<_>>().join(", "),
        manifest.platform,
        agents_bootstrap,
        manifest
            .tools
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", "),
        manifest.platform,
        pixi_file,
    );
    fs::write(out.join("AGENTS.md"), agents)
        .with_context(|| format!("writing {}/AGENTS.md", out.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MANIFEST_DIR, PackPlan, ToolSource, embed_tool};
    use std::fs;
    use std::path::{Path, PathBuf};

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

    /// A `PackArgs` with every optional field at its CLI default, so each test below only
    /// states the field it means to exercise.
    fn args(repo_root: &Path, output_dir: &Path, envs: &[&str]) -> crate::cli::PackArgs {
        crate::cli::PackArgs {
            diagnostics: crate::cli::DiagnosticsArgs::default(),
            repo_root: repo_root.to_path_buf(),
            envs: envs.iter().map(|&s| s.to_string()).collect(),
            output_dir: output_dir.to_path_buf(),
            platform: "linux-64".to_string(),
            shard_limit_mib: 95.0,
            cargo_vendor: false,
            cargo_vendor_mode: crate::cli::VendorModeArg::Loose,
            fetch_tools: false,
            tools_lock: None,
            tools_cache: None,
            self_bin: None,
        }
    }

    #[test]
    fn a_missing_pixi_lock_is_refused_before_anything_is_created() {
        let repo = tempfile::tempdir().expect("repo");
        let output_dir = repo.path().join("out");
        let error = PackPlan::from_args(args(repo.path(), &output_dir, &["default"]))
            .expect_err("no pixi.lock");
        assert!(format!("{error:#}").contains("pixi.lock"), "{error:#}");
        assert!(!output_dir.exists(), "a refused plan must create nothing");
    }

    #[test]
    fn an_existing_output_dir_is_refused() {
        let repo = tempfile::tempdir().expect("repo");
        fs::write(repo.path().join("pixi.lock"), "").expect("lockfile");
        let output_dir = repo.path().join("out");
        fs::create_dir_all(&output_dir).expect("pre-existing output dir");
        let error = PackPlan::from_args(args(repo.path(), &output_dir, &["default"]))
            .expect_err("stale output dir");
        assert!(format!("{error:#}").contains("already exists"), "{error:#}");
    }

    #[test]
    fn unsafe_and_duplicate_env_names_are_refused() {
        let repo = tempfile::tempdir().expect("repo");
        fs::write(repo.path().join("pixi.lock"), "").expect("lockfile");
        let output_dir = repo.path().join("out");

        let error = PackPlan::from_args(args(repo.path(), &output_dir, &[".."]))
            .expect_err("unsafe env name");
        assert!(
            format!("{error:#}").contains("unsafe environment name"),
            "{error:#}"
        );

        let error = PackPlan::from_args(args(repo.path(), &output_dir, &["default", "default"]))
            .expect_err("duplicate env name");
        assert!(
            format!("{error:#}").contains("requested more than once"),
            "{error:#}"
        );
    }

    #[test]
    fn valid_args_resolve_to_absolute_paths_and_a_byte_shard_limit() {
        let repo = tempfile::tempdir().expect("repo");
        fs::write(repo.path().join("pixi.lock"), "").expect("lockfile");
        let output_dir = repo.path().join("out");

        let plan = PackPlan::from_args(args(repo.path(), &output_dir, &["default", "web"]))
            .expect("valid plan");
        assert!(plan.root.is_absolute(), "{}", plan.root.display());
        assert!(plan.out.is_absolute(), "{}", plan.out.display());
        assert_eq!(plan.payload, plan.out.join(MANIFEST_DIR));
        assert_eq!(plan.shard_limit, (95.0 * 1024.0 * 1024.0) as u64);
    }

    /// `resolve_tools` is otherwise network-bound (it downloads from the pins it loads), so
    /// this covers the one failure `--tools-lock` can hit entirely on the filesystem: an
    /// override path that does not exist. The PATH-fallback and embedded-pin branches stay
    /// covered by `tests/e2e.rs`'s full `pack` round trip.
    #[test]
    fn an_explicit_tools_lock_override_that_does_not_exist_is_refused() {
        let repo = tempfile::tempdir().expect("repo");
        fs::write(repo.path().join("pixi.lock"), "").expect("lockfile");
        let output_dir = repo.path().join("out");
        let mut pack_args = args(repo.path(), &output_dir, &["default"]);
        pack_args.fetch_tools = true;
        pack_args.tools_lock = Some(PathBuf::from("missing-tools-lock.toml"));

        let plan = PackPlan::from_args(pack_args).expect("validation alone does not fetch");
        let error = plan
            .resolve_tools()
            .expect_err("the override file does not exist");
        assert!(
            format!("{error:#}").contains("reading tool-pin override"),
            "{error:#}"
        );
    }
}
