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
use pixi_sandbox::pack::{build_files_oracle, fingerprint_of, record_tree, sum_files};
use pixi_sandbox::vendor::{VendorInfo, VendorMode, validate_vendorable_lockfile, vendor_tree};
use pixi_sandbox_core::host_requirements::HostRequirementSet;
use pixi_sandbox_core::manifest::{
    Env, MANIFEST_DIR, MANIFEST_FILE, Manifest, SCHEMA_VERSION, Source, ToolEntry, ToolInfo, Vendor,
};
use pixi_sandbox_core::shard;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

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

        // Before anything is created: a lockfile cargo cannot vendor is a property of the
        // project, not of this run, and it is the one `--cargo-vendor` failure an operator
        // cannot act on without being told what to change (doc-2 §11). Detecting it here
        // rather than inside the vendor step means a refused pack leaves no output directory
        // behind — otherwise the next attempt fails on the stale-transport guard and the real
        // error is one run in the dark.
        if args.cargo_vendor {
            validate_vendorable_lockfile(&root)?;
        }

        // Issue #157: a non-wheel PyPI dependency cannot be carried by the native writer.
        // Refuse it with the selected environment/package named, before any acquisition or
        // output creation. This keeps the other preflight guards' contract: refusal leaves no
        // output directory behind and says exactly which dependency to move out.
        pixi_sandbox::pixi_lock::refuse_non_wheel_pypi(
            &root.join("pixi.lock"),
            &args.envs,
            &args.platform,
        )?;

        for environment in &args.envs {
            pixi_sandbox::native_pack::preflight(&root, environment, &args.platform)?;
        }

        // Resolved here, with the other pre-flight guards: a config that cannot be read or
        // does not validate is a refusal that must leave no output directory behind.
        let host_requirements =
            pixi_sandbox::pack::resolve_host_requirements(args.config.as_deref(), &args.platform)?;

        Ok(Self {
            args,
            root,
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
        for name in &self.args.envs {
            self.pack_environment(name, &mut envs)?;
        }

        println!("embed tools");
        let mut entries = BTreeMap::new();
        self.embed_pixi(&mut entries)?;

        self.embed_self_bin(&mut entries)?;

        let (vendor, vendor_info) = if self.args.cargo_vendor {
            println!("vendor cargo dependencies");
            vendor_tree(
                &self.root,
                &self.out,
                &self.payload,
                match self.args.cargo_vendor_mode {
                    VendorModeArg::Loose => VendorMode::Loose,
                    VendorModeArg::Tarballs => VendorMode::Tarballs,
                },
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

    /// Pack locked artifacts with the native writer, then install a scratch copy to record
    /// the per-file oracle for the tree the airlock will receive. The oracle also provides
    /// the honest `unpacked_size_bytes`: measured on the
    /// unpacked tree, not parsed from a version-dependent log line.
    fn pack_environment(&self, name: &str, envs: &mut BTreeMap<String, Env>) -> Result<()> {
        let target = self.payload.join("envs").join(name).join("pack");
        let parent = target
            .parent()
            .expect("pack target always has an env parent");
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;

        pixi_sandbox::native_pack::write(&self.root, name, &self.args.platform, &target)?;

        let files = shard::files_under(&target)
            .with_context(|| format!("reading native pack output for environment {name}"))?;
        if files.is_empty() {
            bail!("native pack produced no files for environment {name}");
        }
        let packed_size = sum_files(&files)?;

        let (oracle, unpacked_size) = build_files_oracle(
            &self.out,
            &self.payload,
            name,
            &target,
            None,
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
            name.to_string(),
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
