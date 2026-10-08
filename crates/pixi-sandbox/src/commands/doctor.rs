//! `doctor` — read-only inspection of a transport directory or an extracted branch.
//!
//! This is the one fully implemented command in the scaffold, because it is the one an
//! airlock operator needs *before* anything is written and the one CI runs against its own
//! output. It never writes into the transport or the project (the standalone probe's scratch
//! lives beside the branch and is removed before returning), never touches the network, and
//! (with `--verify`) reports every failure instead of the first one. `--verify-restored
//! <project>` extends the same honesty to the *output* of a restore: the tree that came out
//! the other end is checked against the manifest's per-file digests (D13), because an intact
//! branch plus verified writes still does not prove the prefix is right.

use crate::cli::DoctorArgs;
use crate::commands::support;
use anyhow::{Context, Result, bail};
use pixi_sandbox::host_probe::SystemHostProbe;
use pixi_sandbox_core::host_requirements::{HostReport, evaluate};
use pixi_sandbox_core::manifest::{MANIFEST_DIR, Manifest};
use pixi_sandbox_core::platform::Platform;
use pixi_sandbox_core::sandbox_config::SandboxConfig;
use pixi_sandbox_core::transport_budget::{self, BudgetReport};
use pixi_sandbox_core::verify::{self, Report, RestoredReport};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub fn run(args: DoctorArgs) -> Result<()> {
    crate::diagnostics::phase("load-manifest", "locating and parsing manifest.json");
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
    let report = (args.verify || project.is_some()).then(|| {
        crate::diagnostics::phase("verify-transport", "checking every declared transport byte");
        verify::verify(&manifest, &args.branch_location, only)
    });
    let restored = project.as_deref().map(|project| {
        crate::diagnostics::phase(
            "verify-restored",
            "checking restored files against the oracle",
        );
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

    // The standalone probe (issue #81, AC#3). Hashes prove the embedded tool is the
    // declared file; only executing it — under the empty environment an airlock has —
    // proves it *runs*. The order is the trust boundary: probe only what verified green,
    // on a host that matches the manifest platform, and never write into the branch.
    crate::diagnostics::phase(
        "standalone-probe",
        "checking the embedded pixi-sandbox helper when verified and host-compatible",
    );
    let standalone = standalone_probe(&args.branch_location, &manifest, report.as_ref());

    // Host requirements (issue #109, TASK-75). Probed only when the transport declares some,
    // and only against a host of the transport's own family: a Linux libc floor says nothing
    // about a macOS machine, and `evaluate` answers `not applicable` there without querying
    // anything. The probes are read-only and never install, start or launch what they look for.
    let host = manifest
        .host_requirements
        .as_ref()
        .filter(|declared| !declared.is_empty())
        .map(|declared| {
            crate::diagnostics::phase(
                "probe-host",
                "classifying the transport's host requirements on this machine",
            );
            let platform_family = Platform::from_str(&manifest.platform)
                .ok()
                .map(Platform::host_family);
            evaluate(declared, platform_family, &SystemHostProbe::new())
        });

    let budget = match &args.budget_config {
        Some(config) => {
            crate::diagnostics::phase(
                "check-budgets",
                "checking transport size budgets before publish",
            );
            let config_path = support::absolute(config)?;
            let config = SandboxConfig::load(&config_path).with_context(|| {
                format!("loading sandbox budget config {}", config_path.display())
            })?;
            let branch_root = transport_root_from_manifest(&path);
            let snapshot_bytes = repository_snapshot_bytes(&branch_root)?;
            Some(transport_budget::check(
                &manifest,
                Some(snapshot_bytes),
                config.budgets.to_transport_budgets()?,
            ))
        }
        None => None,
    };

    crate::diagnostics::phase(
        "render-report",
        "writing the requested human or JSON report",
    );
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
                restored_section,
                standalone.as_ref(),
                host.as_ref(),
                budget.as_ref()
            ))?
        );
    } else {
        print_human(
            &path,
            &manifest,
            report.as_ref(),
            restored_section,
            standalone.as_ref(),
            host.as_ref(),
            budget.as_ref(),
        );
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
    if let Some(StandaloneProbe::Refused { detail }) = &standalone {
        bail!("standalone probe failed:\n{detail}");
    }
    if let Some(budget) = &budget {
        if !budget.ok() {
            bail!(
                "transport budget exceeded: {} threshold(s)",
                budget.violations.len()
            );
        }
    }
    if args.require_host_requirements {
        if let Some(host) = &host {
            if !host.ok() {
                bail!(
                    "host requirements not met: {} missing ({} unknown, never a failure)",
                    host.missing(),
                    host.unknown()
                );
            }
        }
    }
    Ok(())
}

/// The probe's single outcome: skipped (with the honest reason), proven to run, or refused
/// with the rendered evidence and remedy. `None` means the check does not apply at all —
/// no `--verify`, or the transport embeds no pixi-sandbox tool (a consumer transport that
/// restores with an installed CLI).
enum StandaloneProbe {
    Skipped(String),
    Runs { version: String },
    Refused { detail: String },
}

/// Probe the embedded pixi-sandbox tool exactly as a restore will execute it.
///
/// Why these guards, in this order: probing is *executing fetched bytes*, so it runs only
/// behind `--verify` (a bare look at a branch executes nothing — the same trust shape
/// doctor always had) and only after the hash report is green; and a foreign-platform
/// binary cannot be judged here, so cross-platform hosts skip — the skip is printed, never
/// silent, because an airlock operator must know *who* still owes this proof.
fn standalone_probe(
    branch_location: &Path,
    manifest: &Manifest,
    report: Option<&Report>,
) -> Option<StandaloneProbe> {
    let report = report?;
    let tool = manifest.tools.get("pixi-sandbox")?;
    let relative = tool.path.clone()?;

    let skipped = match pixi_sandbox::standalone::host_platform() {
        Some(host) if host != manifest.platform => Some(format!(
            "manifest platform {} ≠ this host ({host}); the probe runs on the target host's doctor",
            manifest.platform
        )),
        None => Some("this host has no pixi platform in the release matrix".to_string()),
        Some(_) => None,
    };
    if let Some(reason) = skipped {
        return Some(StandaloneProbe::Skipped(reason));
    }
    if !report.ok() {
        return Some(StandaloneProbe::Skipped(
            "the hash report is not green — unverified bytes are never executed".to_string(),
        ));
    }

    let tool_path = branch_location.join(MANIFEST_DIR).join(&relative);
    let anchor = branch_location
        .with_file_name(format!(".pixi-sandbox-doctor-probe-{}", std::process::id()));
    let outcome = fs::create_dir_all(anchor.join("tmp"))
        .with_context(|| format!("creating {}", anchor.display()))
        .and_then(|()| {
            pixi_sandbox::standalone::probe(
                &tool_path,
                &anchor,
                &pixi_sandbox::standalone::CommandRunner::new(),
            )
            .map_err(|refusal| anyhow::anyhow!(refusal.render(&tool_path)))
        });
    let scratch = support::remove_path(&anchor);
    Some(match (outcome, scratch) {
        (Ok(()), Ok(())) => StandaloneProbe::Runs {
            version: tool.version.clone(),
        },
        (Ok(()), Err(scratch)) => StandaloneProbe::Refused {
            detail: format!(
                "the tool runs standalone, but the probe scratch was not removed: {scratch:#}"
            ),
        },
        (Err(failure), _) => StandaloneProbe::Refused {
            detail: format!("{failure:#}"),
        },
    })
}

/// Accept either the manifest itself or the directory that contains it.
fn locate(branch_location: &Path) -> PathBuf {
    if branch_location.is_file() {
        branch_location.to_path_buf()
    } else {
        verify::manifest_path(branch_location)
    }
}

fn transport_root_from_manifest(manifest_path: &Path) -> PathBuf {
    manifest_path
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| manifest_path.to_path_buf())
}

fn repository_snapshot_bytes(root: &Path) -> Result<u64> {
    fn walk(root: &Path, dir: &Path, total: &mut u64) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
            let entry = entry.with_context(|| format!("reading {}", dir.display()))?;
            let path = entry.path();
            let name = entry.file_name();
            if path == root.join(".git") || name == ".git" {
                continue;
            }
            let metadata = fs::symlink_metadata(&path)
                .with_context(|| format!("reading metadata for {}", path.display()))?;
            if metadata.is_dir() {
                walk(root, &path, total)?;
            } else if metadata.is_file() {
                *total = total
                    .checked_add(metadata.len())
                    .ok_or_else(|| anyhow::anyhow!("snapshot size overflow"))?;
            }
        }
        Ok(())
    }

    let mut total = 0u64;
    walk(root, root, &mut total)?;
    Ok(total)
}

fn print_human(
    path: &Path,
    manifest: &Manifest,
    report: Option<&Report>,
    restored: Option<(&Path, &RestoredReport)>,
    standalone: Option<&StandaloneProbe>,
    host: Option<&HostReport>,
    budget: Option<&BudgetReport>,
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

    // What the transport expects from the *host*, straight from the manifest (issue #109,
    // TASK-75). Printed as declared data: this build reports it without probing the machine, so
    // the line must not read as a verdict about the host it is running on.
    if let Some(host) = host {
        labelled(
            "host",
            &format!("declared requirements: {}", host.declared.summary()),
        );
        labelled(
            "conda",
            "conda-provided libraries travel with the transport and are not probed here",
        );
        for finding in &host.findings {
            let mut line = format!(
                "{} {}: {} ({})",
                finding.kind.as_str(),
                finding.name,
                finding.status,
                finding.detail
            );
            if let Some(remedy) = &finding.remedy {
                line.push_str(" — ");
                line.push_str(remedy);
            }
            println!("  {line}");
        }
        labelled(
            "host",
            &format!(
                "{} of {} satisfied · {} missing · {} unknown",
                host.satisfied(),
                host.findings.len(),
                host.missing(),
                host.unknown()
            ),
        );
        if !host.applicable() {
            labelled(
                "host",
                "not applicable on this host — nothing was probed (see the findings above)",
            );
        }
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

    if let Some(standalone) = standalone {
        match standalone {
            StandaloneProbe::Skipped(reason) => {
                labelled("probe", &format!("skipped — {reason}"));
            }
            StandaloneProbe::Runs { version } => {
                labelled(
                    "probe",
                    &format!(
                        "tool pixi-sandbox v{version}: runs standalone (--version, empty environment)"
                    ),
                );
            }
            StandaloneProbe::Refused { detail } => {
                labelled(
                    "probe",
                    "FAILED — the embedded tool does not run standalone",
                );
                println!("  {detail}");
            }
        }
    }

    if let Some(budget) = budget {
        if budget.ok() {
            labelled(
                "budget",
                &format!(
                    "OK — blob {} MiB, transport {} MiB, push {} MiB, restore preflight {} MiB",
                    mib(budget.measurements.largest_blob_bytes),
                    mib(budget.measurements.transport_bytes),
                    budget
                        .measurements
                        .repository_push_bytes
                        .map(mib)
                        .unwrap_or_else(|| "unknown".to_string()),
                    mib(budget.measurements.restore_required_bytes),
                ),
            );
        } else {
            labelled(
                "budget",
                &format!("FAILED — {} threshold(s) exceeded", budget.violations.len()),
            );
            for violation in &budget.violations {
                println!(
                    "  {}: {} MiB > {} MiB — {}",
                    violation.field,
                    mib(violation.measured_bytes),
                    mib(violation.limit_bytes),
                    violation.remedy
                );
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
    standalone: Option<&StandaloneProbe>,
    host: Option<&HostReport>,
    budget: Option<&BudgetReport>,
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

    if let Some(host) = host {
        // One object under the key the manifest spells: the declaration that travelled in the
        // transport, what this host turned out to be, and the counts a CI job should branch on
        // (`ok` is exactly `missing == 0`).
        let findings: Vec<Value> = host
            .findings
            .iter()
            .map(|finding| {
                let mut value = json!({
                    "kind": finding.kind,
                    "name": finding.name,
                    "status": finding.status,
                    "detail": finding.detail,
                });
                if let Some(remedy) = &finding.remedy {
                    value["remedy"] = json!(remedy);
                }
                value
            })
            .collect();
        out["host_requirements"] = json!({
            "declared": host.declared,
            "platform_family": host.platform_family,
            "host_family": host.host_family,
            "distro": host.distro,
            "applicable": host.applicable(),
            "ok": host.ok(),
            "satisfied": host.satisfied(),
            "missing": host.missing(),
            "unknown": host.unknown(),
            "findings": findings,
        });
    }

    if let Some(budget) = budget {
        out["budget"] = serde_json::to_value(budget).expect("budget report serialises");
    }

    if let Some(standalone) = standalone {
        out["standalone"] = match standalone {
            StandaloneProbe::Skipped(reason) => json!({ "status": "skipped", "reason": reason }),
            StandaloneProbe::Runs { version } => {
                json!({ "status": "ok", "tool": "pixi-sandbox", "version": version })
            }
            StandaloneProbe::Refused { detail } => {
                json!({ "status": "failed", "tool": "pixi-sandbox", "detail": detail })
            }
        };
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
