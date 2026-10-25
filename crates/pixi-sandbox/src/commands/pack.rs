//! `pack` — build a self-contained transport on the connected side.
//!
//! The ordering below is intentional: gather packages and tools into a fresh output tree,
//! record its hashes (splitting only oversized payload blobs), then write the manifest and the
//! human/machine guides. Nothing on the airlock needs to discover or download a tool.

use crate::cli::{PackArgs, VendorModeArg};
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::branch_docs::write_branch_docs;
use pixi_sandbox::embedded_tools::{ToolSource, embed_tool, reported_version};
use pixi_sandbox::pack::{
    PackTarget, build_files_oracle, fingerprint_of, record_tree, resolve_targets, sum_files,
};
use pixi_sandbox::vendor::{VendorInfo, VendorMode, validate_vendorable_lockfile, vendor_tree};
use pixi_sandbox_core::host_requirements::HostRequirementSet;
use pixi_sandbox_core::manifest::{
    Env, MANIFEST_DIR, MANIFEST_FILE, Manifest, SCHEMA_VERSION, Source, ToolEntry, ToolInfo,
    Vendor, WorkspaceEntry, WorkspaceRef, WorkspaceVendor,
};
use pixi_sandbox_core::sandbox_config::SandboxConfig;
use pixi_sandbox_core::shard;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const TOOL_NAME: &str = env!("CARGO_PKG_NAME");
const TOOL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run(args: PackArgs) -> Result<()> {
    crate::diagnostics::phase(
        "validate-plan",
        "checking lockfiles, paths, and environment names",
    );
    let plan = PackPlan::from_args(args)?;
    crate::diagnostics::phase(
        "assemble",
        "packing environments, tools, and optional Cargo vendor",
    );
    let artifacts = plan.assemble_artifacts()?;
    crate::diagnostics::phase("write-manifest", "recording hashes and transport guides");
    plan.write_manifest_and_docs(artifacts)
}

/// Everything `run` decides before any output exists: path resolution plus the guards that
/// must refuse before a transport directory is created (doc-2 §11) — a missing
/// `pixi.lock`, a stale `--output-dir`, an unsafe or duplicate `--envs` name, or (when
/// `--cargo-vendor` is set) a `Cargo.lock` cargo cannot vendor. Named `PackPlan`, not
/// `PackArgs`: by the time [`PackPlan::from_args`] returns, every field has been validated or
/// resolved to an absolute path, which is what makes the later steps infallible on these
/// inputs.
#[derive(Debug)]
struct PackPlan {
    args: PackArgs,
    root: PathBuf,
    /// Every environment this run packs, each resolved to the project root that owns its lock
    /// (TASK-129). A root-only run resolves to one target per `--envs` entry with `root`, which
    /// is exactly what this command did before the type existed.
    targets: Vec<PackTarget>,
    out: PathBuf,
    payload: PathBuf,
    shard_limit: u64,
    /// Resolved `[host_requirements]` for `--platform`, exactly as the manifest will carry it
    /// (issue #109, TASK-75). `None` when no config was given, when it declares no table, or
    /// when nothing resolves for this platform's host family.
    host_requirements: Option<HostRequirementSet>,
}

/// What a `pack` run actually produces before anything is written: the per-environment file
/// oracle (ADR-13), every embedded tool (including an optional `--self-bin`), and the vendored
/// cargo tree. Kept distinct from [`PackPlan::write_manifest_and_docs`] so the artifacts exist
/// as plain data before the manifest is assembled around them.
struct PackedArtifacts {
    envs: BTreeMap<String, Env>,
    tools: BTreeMap<String, ToolEntry>,
    vendor: Option<Vendor>,
    workspace_vendors: Vec<WorkspaceVendor>,
    vendor_info: Option<VendorInfo>,
}

impl PackPlan {
    fn from_args(args: PackArgs) -> Result<Self> {
        let pixi_sandbox::pack::PackLayout {
            root,
            out,
            payload,
            shard_limit,
        } = pixi_sandbox::pack::plan_layout(
            &args.repo_root,
            &args.output_dir,
            &args.envs,
            args.shard_limit_mib,
        )?;

        // Loaded once, before any preflight that depends on it: the config resolves which
        // project root each `--envs` id belongs to, and (when it declares one) the host
        // requirements the manifest will carry.
        let config = args
            .config
            .as_deref()
            .map(|path| {
                let path = support::absolute(path)?;
                SandboxConfig::load(&path)
                    .with_context(|| format!("loading sandbox config {}", path.display()))
            })
            .transpose()?;
        let targets = resolve_targets(&root, &args.envs, config.as_ref())?;

        if args.cargo_vendor {
            // Before anything is created: a lockfile cargo cannot vendor is a property of the
            // project, not of this run, and it is the one `--cargo-vendor` failure an operator
            // cannot act on without being told what to change (doc-2 §11). The root and every
            // selected workspace are checked independently — one transport now carries one tree
            // per project with its own Cargo.lock (TASK-131) — and a project without a lock is
            // simply not vendored. A run with no lock anywhere is the old refusal, named.
            let roots = Self::vendorable_roots(&root, &targets);
            let with_locks = roots
                .iter()
                .filter(|project_root| project_root.join("Cargo.lock").is_file())
                .collect::<Vec<_>>();
            if with_locks.is_empty() {
                let searched = roots
                    .iter()
                    .map(|project_root| project_root.join("Cargo.lock").display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                bail!("--cargo-vendor needs a Cargo.lock, but none exists at: {searched}");
            }
            for project_root in with_locks {
                validate_vendorable_lockfile(project_root)?;
            }
        }

        for target in &targets {
            // Issue #157: a non-wheel PyPI dependency cannot be carried by the native writer.
            // Refuse it with the selected environment/package named, before any acquisition or
            // output creation. A nested workspace gets the same guard against its *own* lock,
            // which is the whole point of resolving a project root per environment.
            pixi_sandbox::pixi_lock::refuse_non_wheel_pypi(
                &target.root.join("pixi.lock"),
                std::slice::from_ref(&target.env),
                &args.platform,
            )?;
            pixi_sandbox::native_pack::preflight(&target.root, &target.env, &args.platform)?;
        }

        // Resolved here, with the other pre-flight guards: a config that cannot be read or
        // does not validate is a refusal that must leave no output directory behind.
        let host_requirements = match &config {
            None => None,
            Some(config) => config
                .host_requirements_for(&args.platform)?
                .filter(|set| !set.is_empty()),
        };

        Ok(Self {
            args,
            root,
            targets,
            out,
            payload,
            shard_limit,
            host_requirements,
        })
    }

    /// Pack every requested environment, build its file oracle, embed the transport tools (and
    /// an optional `--self-bin`), and vendor cargo dependencies when asked. Nothing here is
    /// written to the manifest yet — [`PackPlan::write_manifest_and_docs`] owns that.
    fn assemble_artifacts(&self) -> Result<PackedArtifacts> {
        fs::create_dir_all(&self.payload)
            .with_context(|| format!("creating transport payload at {}", self.payload.display()))?;

        println!(
            "pack environments: {} · {}",
            self.args.envs.join(", "),
            self.args.platform
        );
        let mut envs = BTreeMap::new();
        for target in &self.targets {
            self.pack_environment(target, &mut envs)?;
        }

        println!("embed tools");
        let mut entries = BTreeMap::new();
        self.embed_pixi(&mut entries)?;

        self.embed_self_bin(&mut entries)?;

        let (vendor, workspace_vendors, vendor_info) = if self.args.cargo_vendor {
            println!("vendor cargo dependencies");
            self.vendor_cargo_trees(match self.args.cargo_vendor_mode {
                VendorModeArg::Loose => VendorMode::Loose,
                VendorModeArg::Tarballs => VendorMode::Tarballs,
            })?
        } else {
            (None, Vec::new(), None)
        };

        Ok(PackedArtifacts {
            envs,
            tools: entries,
            vendor,
            workspace_vendors,
            vendor_info,
        })
    }

    /// Distinct project roots a `--cargo-vendor` run may vendor: the repository root first,
    /// then each nested workspace in first-seen order, de-duplicated so a workspace with
    /// several selected environments is still preflighted once.
    fn vendorable_roots(repo_root: &Path, targets: &[PackTarget]) -> Vec<PathBuf> {
        let mut roots = vec![repo_root.to_path_buf()];
        for target in targets {
            if target.workspace.is_some() && !roots.iter().any(|root| root == &target.root) {
                roots.push(target.root.clone());
            }
        }
        roots
    }

    /// Vendor every project root that carries a `Cargo.lock`, each into its own qualified
    /// payload directory. The root keeps `vendor/`; a workspace lands under
    /// `vendor/workspaces/<slug>/`, matching the env convention `envs/workspaces/<slug>/`.
    /// Toolchain provenance is recorded once: the same cargo/rustc builds every tree.
    fn vendor_cargo_trees(
        &self,
        mode: VendorMode,
    ) -> Result<(Option<Vendor>, Vec<WorkspaceVendor>, Option<VendorInfo>)> {
        let mut vendor_info = None;
        let root_vendor = if self.root.join("Cargo.lock").is_file() {
            let (vendor, info) = vendor_tree(
                &self.root,
                &self.out,
                &self.payload,
                Path::new("vendor"),
                mode,
            )?;
            vendor_info = info;
            vendor
        } else {
            None
        };

        let mut workspace_vendors = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for target in &self.targets {
            let Some(workspace) = &target.workspace else {
                continue;
            };
            if !seen.insert(workspace.slug.clone()) || !target.root.join("Cargo.lock").is_file() {
                continue;
            }
            let vendor_rel = PathBuf::from("vendor/workspaces").join(&workspace.slug);
            println!("vendor cargo dependencies: workspace {}", workspace.slug);
            let (vendor, info) =
                vendor_tree(&target.root, &self.out, &self.payload, &vendor_rel, mode)?;
            vendor_info = vendor_info.or(info);
            workspace_vendors.push(WorkspaceVendor {
                slug: workspace.slug.clone(),
                manifest: workspace.manifest.clone(),
                directory: workspace.directory.clone(),
                vendor: vendor.context("cargo vendor produced no tree")?,
            });
        }
        Ok((root_vendor, workspace_vendors, vendor_info))
    }

    /// Pack locked artifacts with the native writer, then install a scratch copy to record
    /// the per-file oracle for the tree the airlock will receive. The oracle also provides
    /// the honest `unpacked_size_bytes`: measured on the
    /// unpacked tree, not parsed from a version-dependent log line.
    fn pack_environment(
        &self,
        target: &PackTarget,
        envs: &mut BTreeMap<String, Env>,
    ) -> Result<()> {
        let id = target.id.as_str();
        // Where this environment's blobs live inside the transport. A nested one sits under the
        // reserved `workspaces/` segment, so a prefix-anchored blob match can never confuse it
        // with a root environment called `subproject`.
        let relative = match &target.workspace {
            None => PathBuf::from("envs").join(&target.env).join("pack"),
            Some(workspace) => PathBuf::from("envs")
                .join(pixi_sandbox_core::sandbox_config::WORKSPACES_SEGMENT)
                .join(&workspace.slug)
                .join(&target.env)
                .join("pack"),
        };
        let destination = self.payload.join(&relative);
        let parent = destination
            .parent()
            .expect("pack target always has an env parent");
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

        pixi_sandbox::native_pack::write(
            &target.root,
            &target.env,
            &self.args.platform,
            &destination,
        )?;

        let files = shard::files_under(&destination)
            .with_context(|| format!("reading native pack output for environment {id}"))?;
        if files.is_empty() {
            bail!("native pack produced no files for environment {id}");
        }
        let packed_size = sum_files(&files)?;

        let env_dir = relative.to_string_lossy().replace('\\', "/");
        let (oracle, unpacked_size) = build_files_oracle(
            &self.out,
            &self.payload,
            id,
            &env_dir,
            &destination,
            None,
            self.shard_limit,
        )
        .with_context(|| format!("building the per-file oracle for environment {id}"))?;
        println!(
            "  {id}: {} files · {} MiB packed · {} MiB unpacked · {} file entries recorded",
            files.len(),
            support::mib(packed_size),
            support::mib(unpacked_size),
            oracle.entries
        );

        envs.insert(
            id.to_string(),
            Env {
                platform: self.args.platform.clone(),
                pack_path: format!(
                    "{MANIFEST_DIR}/{}",
                    relative.to_string_lossy().replace('\\', "/")
                ),
                packed_size_bytes: packed_size,
                unpacked_size_bytes: unpacked_size,
                pixi_environment_fingerprint: fingerprint_of(&target.root, &target.env),
                blobs: Vec::new(),
                files: Some(oracle),
                workspace: target.workspace.as_ref().map(|workspace| WorkspaceRef {
                    manifest: workspace.manifest.clone(),
                    directory: workspace.directory.clone(),
                }),
            },
        );
        Ok(())
    }

    /// Explicitly bundle Pixi for the subsequent frozen/offline environment check.
    fn embed_pixi(&self, entries: &mut BTreeMap<String, ToolEntry>) -> Result<()> {
        let Some(path) = &self.args.pixi_bin else {
            return Ok(());
        };
        let path = if path.is_file() {
            support::absolute(path)?
        } else {
            support::find_executable(path.as_os_str()).context("--pixi-bin was not found")?
        };
        let entry = embed_tool(
            &self.payload,
            &self.args.platform,
            "pixi",
            &ToolSource {
                version: reported_version(&path)?,
                path,
                url: None,
                pinned_sha256: None,
            },
            self.shard_limit,
        )?;
        entries.insert("pixi".into(), entry);
        Ok(())
    }

    /// Embed the CLI binary itself (`--self-bin`) and probe it standalone on a matching host.
    /// No `--self-bin` means no-op.
    fn embed_self_bin(&self, entries: &mut BTreeMap<String, ToolEntry>) -> Result<()> {
        let Some(self_bin) = &self.args.self_bin else {
            return Ok(());
        };
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
        // environment. The manifest records the local digest, size and linkage, but a static
        // trampoline can pass all those checks and still die without its global prefix;
        // only running the embedded bytes can prove that boundary.
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
        Ok(())
    }

    /// The declared workspaces this run actually carries, one entry per slug.
    ///
    /// Only workspaces whose environments were selected appear, in declaration order, so a
    /// `--envs` subset does not advertise a workspace the transport has no environment for.
    fn workspace_entries(&self) -> Vec<WorkspaceEntry> {
        let mut entries: Vec<WorkspaceEntry> = Vec::new();
        for target in &self.targets {
            let Some(workspace) = &target.workspace else {
                continue;
            };
            match entries
                .iter_mut()
                .find(|entry| entry.slug == workspace.slug)
            {
                Some(entry) => entry.environments.push(target.env.clone()),
                None => entries.push(WorkspaceEntry {
                    slug: workspace.slug.clone(),
                    manifest: workspace.manifest.clone(),
                    directory: workspace.directory.clone(),
                    environments: vec![target.env.clone()],
                }),
            }
        }
        entries
    }

    /// Shard oversized blobs, assemble and validate the manifest, write it (and the branch
    /// docs built from it) under `--output-dir`, and print the summary an operator reads.
    fn write_manifest_and_docs(&self, artifacts: PackedArtifacts) -> Result<()> {
        let PackedArtifacts {
            mut envs,
            tools,
            mut vendor,
            mut workspace_vendors,
            vendor_info,
        } = artifacts;

        println!("record manifest and shard oversized files");
        for (id, environment) in &mut envs {
            // `pack_path` was computed from the same relative directory the writer used, so the
            // blob walk asks the manifest where the tree is instead of rebuilding the layout
            // from the environment's name (which is not a path: `subproject/default`).
            let relative = environment
                .pack_path
                .strip_prefix(MANIFEST_DIR)
                .with_context(|| format!("env {id}: pack path outside {MANIFEST_DIR}"))?
                .trim_start_matches('/');
            let root = self.payload.join(relative);
            environment.blobs = record_tree(&self.payload, &root, self.shard_limit, None)?;
        }
        if let Some(vendor) = &mut vendor {
            let root = self.payload.join("vendor");
            // Nested trees live under vendor/workspaces/ and carry their own blob lists, so
            // the root walk must not claim their files for the root Vendor.
            let nested = self.payload.join("vendor/workspaces");
            vendor.blobs = record_tree(&self.payload, &root, self.shard_limit, Some(&nested))?;
        }
        for tree in &mut workspace_vendors {
            // The same relative layout the producer wrote, so blob paths stay anchored at the
            // payload root and pass the manifest's `vendor/workspaces/<slug>/` prefix rule.
            let root = self.payload.join("vendor/workspaces").join(&tree.slug);
            tree.vendor.blobs = record_tree(&self.payload, &root, self.shard_limit, None)?;
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
            workspaces: self.workspace_entries(),
            vendor,
            workspace_vendors,
            host_requirements: self.host_requirements.clone(),
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
