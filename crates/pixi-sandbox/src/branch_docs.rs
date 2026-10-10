//! Human and machine guides rendered from the authoritative transport manifest.
//!
//! Rendering is pure; writing is a separate boundary, so the guides can be compared against
//! pre-extraction bytes without a CLI process or an output directory.

use crate::pack::support;
use crate::vendor::VendorInfo;
use anyhow::{Context, Result};
use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::tools_lock::executable_filename;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchDocs {
    pub readme: String,
    pub agents: String,
}

/// Render exactly the given manifest; pack validates it before calling this boundary.
#[must_use]
pub fn render(manifest: &Manifest, vendor_info: Option<&VendorInfo>) -> BranchDocs {
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

    // The manifest is the machine-readable carrier; these two renderings are the human ones — a
    // reader of the branch learns what the machine must provide before launching anything.
    let declared_host = manifest
        .host_requirements
        .as_ref()
        .filter(|host| !host.is_empty());
    let host_section = declared_host
        .map(|host| {
            format!(
                "\nHost requirements (provided by the host, never installed by this transport): {}.\n",
                host.summary()
            )
        })
        .unwrap_or_default();
    let host_bullet = declared_host
        .map(|host| {
            format!(
                "- host requirements (provided by the host, never installed by this transport): {};\n",
                host.summary()
            )
        })
        .unwrap_or_default();

    let readme = format!(
        "# Offline sandbox (orphan branch)\n\n\
         Built {} from commit `{}` for platform `{}`.\n\
         `pixi.lock` sha256 `{}`.\n\n\
         {}\
         | env | platform | packed | unpacked | files |\n\
         | --- | --- | ---: | ---: | ---: |\n\
         {}\n\
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
        host_section,
        shell_language,
        restore_commands,
    );
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
         {}\
         - never download tools at restore time; bundled tools are: {};\n\
         - after restore, use pixi as the only entrypoint: `pixi install --frozen --offline` \
           (or `<project>/.pixi/tools/{}/{}` when user launchers were skipped) must be a no-op;\n\
         - no `.pixi/sandbox-env.sh` activation script is generated or supported.\n",
        manifest.schema,
        manifest.envs.keys().cloned().collect::<Vec<_>>().join(", "),
        manifest.platform,
        host_bullet,
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
    BranchDocs { readme, agents }
}

/// Write the two guides, preserving the original filenames, order and error context.
///
/// # Errors
///
/// Returns an error if either guide cannot be written into `out`.
pub fn write_branch_docs(
    out: &Path,
    manifest: &Manifest,
    vendor_info: Option<&VendorInfo>,
) -> Result<()> {
    let BranchDocs { readme, agents } = render(manifest, vendor_info);
    fs::write(out.join("README.md"), readme)
        .with_context(|| format!("writing {}/README.md", out.display()))?;
    fs::write(out.join("AGENTS.md"), agents)
        .with_context(|| format!("writing {}/AGENTS.md", out.display()))?;
    Ok(())
}
