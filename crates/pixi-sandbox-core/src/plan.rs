use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub id: String,
    pub intent: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub mutates: bool,
    pub produces: Vec<PathBuf>,
    pub requires_hosts: Vec<String>,
}

pub fn run(
    envs: Vec<String>,
    targets: Vec<String>,
    components: &str,
    dry_run: bool,
    json: bool,
    cwd: Option<&Path>,
    manifest_path: Option<&Path>,
) -> Result<()> {
    let inv = crate::inventory::detect_inventory(cwd, manifest_path)?;
    let root = inv.root.clone();

    let envs_to_plan = if envs.is_empty() {
        if inv.environments.is_empty() {
            vec!["default".to_string()]
        } else {
            inv.environments.clone()
        }
    } else {
        envs
    };

    let platforms_to_plan = if targets.is_empty() {
        if inv.platforms.is_empty() {
            vec!["linux-64".to_string()]
        } else {
            inv.platforms.clone()
        }
    } else {
        targets.clone()
    };

    // Component selection derived from lockfiles per D13, auto unless overridden
    let components_list: Vec<String> = if components == "auto" {
        let mut comps = Vec::new();
        if inv.has_pixi_lock {
            comps.push("env".to_string());
        }
        if inv.has_cargo_lock {
            comps.push("vendor".to_string());
        }
        comps.push("self".to_string()); // pixi-sandbox binary itself per D14
        comps
    } else {
        components
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    let mut actions: Vec<Action> = Vec::new();

    // Pack actions: one per env x platform
    if components_list.contains(&"env".to_string()) {
        for env in &envs_to_plan {
            for platform in &platforms_to_plan {
                actions.push(Action {
                    id: format!("pack.{env}.{platform}"),
                    intent: format!("pack env `{env}` for {platform}"),
                    program: "pixi-pack".to_string(),
                    args: vec![
                        "--environment".to_string(),
                        env.clone(),
                        "--platform".to_string(),
                        platform.clone(),
                        "--output-file".to_string(),
                        format!("packs/{env}-{platform}.tar"),
                    ],
                    cwd: root.clone(),
                    mutates: true,
                    produces: vec![PathBuf::from(format!("packs/{env}-{platform}.tar"))],
                    requires_hosts: vec![
                        "prefix.dev".to_string(),
                        "conda.anaconda.org".to_string(),
                    ],
                });
            }
        }
    }

    // Vendor action
    if components_list.contains(&"vendor".to_string()) {
        actions.push(Action {
            id: "vendor.sync".to_string(),
            intent: "vendor Rust crates via cargo vendor --locked --versioned-dirs".to_string(),
            program: "cargo".to_string(),
            args: vec![
                "vendor".to_string(),
                "--locked".to_string(),
                "--versioned-dirs".to_string(),
                ".cargo/vendor".to_string(),
            ],
            cwd: root.clone(),
            mutates: true,
            produces: vec![
                PathBuf::from(".cargo/vendor"),
                PathBuf::from("vendor/vendor.tar.gz"),
            ],
            requires_hosts: vec!["index.crates.io".to_string()],
        });
    }

    // Self-hosting: ship pixi + pixi-unpack + pixi-sandbox binary (D14)
    if components_list.contains(&"self".to_string()) {
        for bin in ["pixi", "pixi-unpack", "pixi-sandbox"] {
            actions.push(Action {
                id: format!("selfhost.{bin}"),
                intent: format!("mirror {bin} binary into bin/ with sha256 (D14)"),
                program: "pixi-sandbox".to_string(),
                args: vec![
                    "mirror".to_string(),
                    "binary".to_string(),
                    "--as".to_string(),
                    bin.to_string(),
                ],
                cwd: root.clone(),
                mutates: true,
                produces: vec![PathBuf::from(format!(
                    "bin/{bin}-x86_64-unknown-linux-musl"
                ))],
                requires_hosts: vec!["github.com".to_string()],
            });
        }
    }

    // Kit assembly + transport
    actions.push(Action {
        id: "kit.assemble".to_string(),
        intent: "assemble kit branch with SHA256SUMS + dist-manifest.json + workspace/".to_string(),
        program: "pixi-sandbox".to_string(),
        args: vec!["kit".to_string(), "build".to_string()],
        cwd: root.clone(),
        mutates: true,
        produces: vec![
            PathBuf::from("SHA256SUMS"),
            PathBuf::from("dist-manifest.json"),
            PathBuf::from("workspace/pixi.toml"),
        ],
        requires_hosts: vec![],
    });

    actions.push(Action {
        id: "transport.push".to_string(),
        intent: format!("push kit to orphan branch {}", "pixi-sandbox-dist"),
        program: "git".to_string(),
        args: vec![
            "push".to_string(),
            "origin".to_string(),
            "pixi-sandbox-dist".to_string(),
        ],
        cwd: root.clone(),
        mutates: true,
        produces: vec![],
        requires_hosts: vec!["github.com".to_string()],
    });

    if json {
        let j = serde_json::json!({
            "root": root,
            "manifest": inv.manifest_path,
            "environments": envs_to_plan,
            "platforms": platforms_to_plan,
            "components": components_list,
            "has_pixi_lock": inv.has_pixi_lock,
            "has_cargo_lock": inv.has_cargo_lock,
            "has_vendor": inv.has_vendor,
            "tier": "auto",
            "dry_run": dry_run,
            "actions": actions,
        });
        println!("{}", serde_json::to_string_pretty(&j).unwrap());
    } else {
        println!("workspace   {} (tier L0)", root.display());
        if let Some(m) = &inv.manifest_path {
            println!("  manifest  {} (tier L1)", m.display());
        }
        println!("environments ({} detected)", envs_to_plan.len());
        for e in &envs_to_plan {
            println!("  {e} tier L1");
        }
        println!("platforms: {}", platforms_to_plan.join(" "));
        println!("components  {} (auto-derived)", components_list.join(","));
        if inv.has_pixi_lock {
            println!("  env     on pixi.lock present (L0) → pixi-pack per (env × platform)");
        }
        if inv.has_cargo_lock {
            println!(
                "  vendor  on Cargo.lock present (L0) → cargo vendor --locked --versioned-dirs"
            );
        }
        println!("  self    on package pixi-sandbox (L1) → bin/* into kit + dist branch");
        println!();
        println!("plan ({} actions, dry_run={dry_run}):", actions.len());
        for a in &actions {
            let mutates_flag = if a.mutates { "mut" } else { "read" };
            println!(
                "  {} [{}] {} → {} {}",
                a.id,
                mutates_flag,
                a.intent,
                a.program,
                a.args.join(" ")
            );
            if !a.requires_hosts.is_empty() {
                println!("    requires: {}", a.requires_hosts.join(", "));
            }
            if !a.produces.is_empty() {
                println!(
                    "    produces: {}",
                    a.produces
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
            }
        }
    }

    Ok(())
}
