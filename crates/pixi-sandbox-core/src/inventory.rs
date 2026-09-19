use crate::error::{Result, SandboxError};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Inventory {
    pub root: PathBuf,
    pub manifest_path: Option<PathBuf>,
    pub environments: Vec<String>,
    pub platforms: Vec<String>,
    pub has_pixi_lock: bool,
    pub has_cargo_lock: bool,
    pub has_vendor: bool,
    pub tools: Vec<ToolInfo>,
}

#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub name: String,
    pub version: Option<String>,
    pub present: bool,
}

pub fn detect_inventory(cwd: Option<&Path>, manifest_path: Option<&Path>) -> Result<Inventory> {
    let root = if let Some(p) = manifest_path {
        p.parent().unwrap_or(Path::new(".")).to_path_buf()
    } else if let Some(c) = cwd {
        c.to_path_buf()
    } else {
        std::env::current_dir().map_err(|e| SandboxError::Other(e.into()))?
    };

    let manifest_path_buf = if let Some(p) = manifest_path {
        Some(p.to_path_buf())
    } else {
        let pixi_toml = root.join("pixi.toml");
        if pixi_toml.exists() {
            Some(pixi_toml)
        } else {
            None
        }
    };

    let mut envs = Vec::new();
    let mut platforms = Vec::new();
    if let Some(ref mp) = manifest_path_buf
        && let Ok(content) = std::fs::read_to_string(mp)
        && let Ok(value) = toml::from_str::<toml::Value>(&content)
    {
        if let Some(env_table) = value.get("environments").and_then(|v| v.as_table()) {
            envs.extend(env_table.keys().cloned());
        }
        if let Some(ws) = value.get("workspace").and_then(|v| v.as_table())
            && let Some(plats) = ws.get("platforms").and_then(|v| v.as_array())
        {
            for p in plats {
                if let Some(s) = p.as_str() {
                    platforms.push(s.to_string());
                }
            }
        }
    }
    if envs.is_empty() {
        envs.push("default".to_string());
    }

    let has_pixi_lock = root.join("pixi.lock").exists();
    let has_cargo_lock = root.join("Cargo.lock").exists();
    let has_vendor = root.join(".cargo").join("vendor").exists() || root.join("vendor").exists();

    let tool_names = [
        "pixi",
        "pixi-pack",
        "pixi-unpack",
        "cargo",
        "rustc",
        "bun",
        "git",
        "tar",
    ];
    let mut tools = Vec::new();
    for name in tool_names {
        let present = which(name).is_some();
        let version = if present {
            get_tool_version(name)
        } else {
            None
        };
        tools.push(ToolInfo {
            name: name.to_string(),
            version,
            present,
        });
    }

    Ok(Inventory {
        root,
        manifest_path: manifest_path_buf,
        environments: envs,
        platforms,
        has_pixi_lock,
        has_cargo_lock,
        has_vendor,
        tools,
    })
}

fn which(name: &str) -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(name);
            if candidate.exists() {
                return Some(candidate);
            }
            let candidate_exe = dir.join(format!("{name}.exe"));
            if candidate_exe.exists() {
                return Some(candidate_exe);
            }
        }
    }
    None
}

fn get_tool_version(name: &str) -> Option<String> {
    let output = std::process::Command::new(name)
        .arg("--version")
        .output()
        .ok()?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
        let s = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

pub fn run(tier: &str, json: bool, cwd: Option<&Path>, manifest_path: Option<&Path>) -> Result<()> {
    let inv = detect_inventory(cwd, manifest_path)?;

    if json {
        let json_val = serde_json::json!({
            "root": inv.root,
            "manifest": inv.manifest_path,
            "environments": inv.environments,
            "platforms": inv.platforms,
            "has_pixi_lock": inv.has_pixi_lock,
            "has_cargo_lock": inv.has_cargo_lock,
            "has_vendor": inv.has_vendor,
            "tier": tier,
            "tools": inv.tools.iter().map(|t| serde_json::json!({
                "name": t.name,
                "present": t.present,
                "version": t.version
            })).collect::<Vec<_>>()
        });
        println!("{}", serde_json::to_string_pretty(&json_val).unwrap());
    } else {
        println!("workspace   {} (tier L0)", inv.root.display());
        if let Some(mp) = &inv.manifest_path {
            println!("  manifest  {} (tier L1)", mp.display());
        }
        println!(
            "environments ({} detected) tier {}",
            inv.environments.len(),
            tier
        );
        for env in &inv.environments {
            println!("  {env} tier L1");
        }
        if !inv.platforms.is_empty() {
            println!("platforms: {}", inv.platforms.join(" "));
        }
        println!("components auto-derived");
        println!(
            "  env     {} pixi.lock present (L0)",
            if inv.has_pixi_lock { "on" } else { "off" }
        );
        println!(
            "  vendor  {} Cargo.lock present (L0)",
            if inv.has_cargo_lock { "on" } else { "off" }
        );
        println!("toolchains");
        for tool in &inv.tools {
            if tool.present {
                println!(
                    "  {} {} ✅",
                    tool.name,
                    tool.version.as_deref().unwrap_or("")
                );
            } else {
                println!("  {} not-found ⛔", tool.name);
            }
        }
    }
    Ok(())
}
