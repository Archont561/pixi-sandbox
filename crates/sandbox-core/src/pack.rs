use crate::error::{Result, SandboxError};
use std::path::Path;

pub fn run(
    envs: Vec<String>,
    targets: Vec<String>,
    out: Option<&Path>,
    all_envs: bool,
    dry_run: bool,
    cwd: Option<&Path>,
    manifest_path: Option<&Path>,
) -> Result<()> {
    let inv = crate::inventory::detect_inventory(cwd, manifest_path)?;

    let envs_to_pack = if all_envs {
        inv.environments.clone()
    } else if envs.is_empty() {
        vec!["default".to_string()]
    } else {
        envs
    };

    let out_dir = out.unwrap_or(Path::new("dist")).to_path_buf();

    println!(
        "plan: would pack envs {:?} for targets {:?} into {}",
        envs_to_pack,
        targets,
        out_dir.display()
    );
    println!("  manifest: {:?}", inv.manifest_path);
    println!("  has_pixi_lock: {}", inv.has_pixi_lock);

    if dry_run {
        println!("dry-run: no artifacts written");
        return Ok(());
    }

    // For M1, we implement a minimal pack that uses pixi-pack if available, otherwise stubs
    // Check for pixi-pack binary
    let pixi_pack_present = inv.tools.iter().any(|t| t.name == "pixi-pack" && t.present);
    if !pixi_pack_present {
        return Err(SandboxError::Unavailable(
            "pixi-pack not found on PATH — install via pixi or ship bin/pixi-pack-<triple> from branch (D14)".to_string(),
        ));
    }

    // In real implementation, we would call pixi-pack for each env x platform
    // For now, we create a placeholder
    std::fs::create_dir_all(&out_dir).map_err(|e| SandboxError::Other(e.into()))?;
    for env in &envs_to_pack {
        let pack_path = out_dir.join(format!("{env}-linux-64.tar"));
        println!("  packing {env} -> {}", pack_path.display());
        // Placeholder: create empty file
        std::fs::write(
            &pack_path,
            b"placeholder pack - real pixi-pack would run here",
        )
        .map_err(|e| SandboxError::Other(e.into()))?;
    }

    println!("pack: done (placeholder, M1 will call pixi-pack)");
    Ok(())
}
