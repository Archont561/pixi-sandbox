//! `doctor` — read-only inspection of a transport directory or an extracted branch.
//!
//! This is the one fully implemented command in the scaffold, because it is the one an
//! airlock operator needs *before* anything is written and the one CI runs against its own
//! output. It never writes, never touches the network, and (with `--verify`) reports every
//! failure instead of the first one. `--verify-restored <project>` extends the same honesty
//! to the *output* of a restore: the tree that came out the other end is checked against the
//! manifest's per-file digests (D13), because an intact branch plus verified writes still
//! does not prove the prefix is right.

use crate::cli::DoctorArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox_core::manifest::Manifest;
use pixi_sandbox_core::verify::{self, Report, RestoredReport};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub fn run(args: DoctorArgs) -> Result<()> {
    let path = locate(&args.branch_location);
    let manifest = Manifest::load(&path).with_context(|| format!("loading {}", path.display()))?;
    let only = if args.envs.is_empty() {
        None
    } else {
        Some(args.envs.as_slice())
    };
    let project = match &args.verify_restored {
        Some(project) => Some(support::existing_dir(project, "--verify-restored")?),
        None => None,
    };

    // The restored-tree check trusts the manifest as its oracle, so it always verifies the
    // transport first — an unchecked oracle would turn the check into theater.
    let report = (args.verify || project.is_some())
        .then(|| verify::verify(&manifest, &args.branch_location, only));
    let restored = project.as_deref().map(|project| {
        (
            project,
            verify::verify_restored(
                &manifest,
                &args.branch_location,
                project,
                only,
                args.work_dir.as_deref(),
            ),
        )
    });

    let restored_section = restored
        .as_ref()
        .map(|(project, report)| (*project, report));
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&as_json(
                &path,
                &manifest,
                report.as_ref(),
                restored_section
            ))?
        );
    } else {
        print_human(&path, &manifest, report.as_ref(), restored_section);
    }

    if let Some(report) = &report {
        if !report.ok() {
            // Everything is already on stdout; the exit code is for scripts.
            bail!("verify failed: {} failure(s)", report.failures.len());
        }
    }
    if let Some((_, restored)) = &restored {
        if !restored.ok() {
            bail!(
                "restored-tree verification failed: {} failure(s)",
                restored.report.failures.len()
            );
        }
    }
    Ok(())
}

/// Accept either the manifest itself or the directory that contains it.
fn locate(branch_location: &Path) -> PathBuf {
    if branch_location.is_file() {
        branch_location.to_path_buf()
    } else {
        verify::manifest_path(branch_location)
    }
}

fn print_human(
    path: &Path,
    manifest: &Manifest,
    report: Option<&Report>,
    restored: Option<(&Path, &RestoredReport)>,
) {
    labelled("manifest", &path.display().to_string());

    let commit = manifest.source.commit.as_deref().unwrap_or("unknown");
    println!(
        "  platform {} · schema {} · created {} · commit {}",
        manifest.platform, manifest.schema, manifest.created_at, commit
    );

    for (name, env) in &manifest.envs {
        let fingerprint = env
            .pixi_environment_fingerprint
            .as_deref()
            .map(|f| format!(" · fingerprint {f}"))
            .unwrap_or_default();
        let files = env
            .files
            .as_ref()
            .map(|f| format!(" · {} file entries", f.entries))
            .unwrap_or_default();
        println!(
            "  env {name}: {} files packed / {} MiB packed → {} MiB unpacked{fingerprint}{files}",
            env.blobs.len(),
            mib(env.packed_size_bytes),
            mib(env.unpacked_size_bytes),
        );
    }

    for (name, tool) in &manifest.tools {
        let pinned = tool
            .pinned_sha256
            .as_deref()
            .map(|p| format!(" · pinned {}", &p[..p.len().min(12)]))
            .unwrap_or_default();
        println!(
            "  tool {name}: v{} {} {} MiB{pinned}",
            tool.version,
            tool.linkage,
            mib(tool.size_bytes),
        );
    }

    if let Some(vendor) = &manifest.vendor {
        let lock = vendor
            .cargo_lock_sha256
            .as_deref()
            .map(|s| format!(", Cargo.lock {}", &s[..s.len().min(12)]))
            .unwrap_or_default();
        println!(
            "  vendor: {} crates, {} MiB, {} mode{lock}",
            vendor.crates,
            mib(vendor.size_bytes),
            capitalise(&vendor.mode),
        );
    }

    let (envs, tools, vendor) = manifest.payload_split();
    let total = manifest.payload_bytes();
    let mut split = format!("envs {} MiB · tools {} MiB", mib(envs), mib(tools));
    if manifest.vendor.is_some() {
        let share = vendor.saturating_mul(100).checked_div(total).unwrap_or(0);
        split.push_str(&format!(" · vendor {} MiB, {share}% vendor", mib(vendor)));
    }
    println!("  payload {} MiB total ({split})", mib(total),);

    match report {
        None => labelled(
            "hint",
            "pass --verify to check every sha256 (nothing is written)",
        ),
        Some(report) => {
            for failure in &report.failures {
                println!(
                    "  {}: {}: {}",
                    failure.path,
                    failure.kind.as_str(),
                    failure.detail
                );
            }
            labelled(
                "verify",
                &format!(
                    "{} blob(s), {} MiB checked, {} failure(s)",
                    report.files,
                    mib(report.bytes),
                    report.failures.len()
                ),
            );
            if report.ok() {
                labelled("verify", "OK — every declared byte matches the manifest");
            } else {
                labelled("verify", "FAILED — do not restore from this branch");
            }
        }
    }

    if let Some((project, restored)) = restored {
        labelled("restored", &project.display().to_string());
        for name in &restored.verified {
            println!(
                "  env {name}: checked against the manifest's file list — {} entry(ies), 0 failure(s)",
                restored.report.files
            );
        }
        for name in &restored.unverifiable {
            println!(
                "  env {name}: no per-file digests in this manifest (schema 1 predates the oracle) — content not verified"
            );
        }
        for failure in &restored.report.failures {
            println!(
                "  {}: {}: {}",
                failure.path,
                failure.kind.as_str(),
                failure.detail
            );
        }
        if restored.ok() {
            if restored.verified.is_empty() {
                labelled(
                    "restored",
                    "nothing to verify — the selected envs predate the per-file oracle",
                );
            } else {
                labelled("restored", "OK — the restored tree matches the manifest");
            }
        } else {
            labelled(
                "restored",
                "FAILED — the restored project is not the tree the manifest describes",
            );
        }
    }
}

fn as_json(
    path: &Path,
    manifest: &Manifest,
    report: Option<&Report>,
    restored: Option<(&Path, &RestoredReport)>,
) -> Value {
    let envs: Vec<Value> = manifest
        .envs
        .iter()
        .map(|(name, env)| {
            json!({
                "name": name,
                "files": env.blobs.len(),
                "packed_bytes": env.packed_size_bytes,
                "unpacked_bytes": env.unpacked_size_bytes,
                "fingerprint": env.pixi_environment_fingerprint,
                "platform": env.platform,
                "file_entries": env.files.as_ref().map(|f| f.entries),
            })
        })
        .collect();

    let tools: Vec<Value> = manifest
        .tools
        .iter()
        .map(|(name, tool)| {
            json!({
                "name": name,
                "version": tool.version,
                "linkage": tool.linkage,
                "size_bytes": tool.size_bytes,
                "pinned_sha256": tool.pinned_sha256,
            })
        })
        .collect();

    let (env_bytes, tool_bytes, vendor_bytes) = manifest.payload_split();
    let mut out = json!({
        "manifest": path.display().to_string(),
        "schema": manifest.schema,
        "platform": manifest.platform,
        "created_at": manifest.created_at,
        "commit": manifest.source.commit,
        "envs": envs,
        "tools": tools,
        "vendor": manifest.vendor.as_ref().map(|v| json!({
            "mode": v.mode,
            "crates": v.crates,
            "size_bytes": v.size_bytes,
            "cargo_lock_sha256": v.cargo_lock_sha256,
        })),
        "payload_bytes": manifest.payload_bytes(),
        "payload_bytes_by_kind": { "envs": env_bytes, "tools": tool_bytes, "vendor": vendor_bytes },
    });

    if let Some(report) = report {
        let failures: Vec<Value> = report
            .failures
            .iter()
            .map(|f| json!({ "path": f.path, "kind": f.kind.as_str(), "detail": f.detail }))
            .collect();
        out["verify"] = json!({
            "files": report.files,
            "bytes": report.bytes,
            "ok": report.ok(),
            "failures": failures,
        });
    }

    if let Some((project, restored)) = restored {
        let failures: Vec<Value> = restored
            .report
            .failures
            .iter()
            .map(|f| json!({ "path": f.path, "kind": f.kind.as_str(), "detail": f.detail }))
            .collect();
        out["restored"] = json!({
            "project": project.display().to_string(),
            "verified": restored.verified,
            "unverifiable": restored.unverifiable,
            "entries": restored.report.files,
            "bytes": restored.report.bytes,
            "ok": restored.ok(),
            "failures": failures,
        });
    }
    out
}

fn labelled(label: &str, text: &str) {
    println!("{label:<10}{text}");
}

fn mib(bytes: u64) -> String {
    format!("{:.1}", bytes as f64 / (1024.0 * 1024.0))
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
