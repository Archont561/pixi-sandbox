use crate::error::Result;
use std::path::Path;

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

    let envs_to_plan = if envs.is_empty() {
        inv.environments.clone()
    } else {
        envs
    };

    if json {
        let j = serde_json::json!({
            "environments": envs_to_plan,
            "targets": targets,
            "components": components,
            "dry_run": dry_run,
            "has_pixi_lock": inv.has_pixi_lock,
            "has_cargo_lock": inv.has_cargo_lock,
        });
        println!("{}", serde_json::to_string_pretty(&j).unwrap());
    } else {
        println!(
            "plan for envs {:?}, targets {:?}, components {}",
            envs_to_plan, targets, components
        );
        println!("  dry_run: {dry_run}");
        println!("  has_pixi_lock: {}", inv.has_pixi_lock);
        println!("  has_cargo_lock: {}", inv.has_cargo_lock);
    }
    Ok(())
}
