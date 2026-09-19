use crate::error::Result;
use std::path::Path;

pub fn run(dir: &Path) -> Result<()> {
    println!("verify: dir={}", dir.display());
    let sums = dir.join("SHA256SUMS");
    if sums.exists() {
        println!("  checking SHA256SUMS...");
        crate::hash::verify_sha256sums(dir)?;
        println!("  integrity: sha256 verified ✅");
    } else {
        println!("  warning: no SHA256SUMS in kit");
    }

    let manifest = dir.join("dist-manifest.json");
    if manifest.exists() {
        println!("  dist-manifest.json present");
    } else {
        println!("  no dist-manifest.json");
    }

    println!("verify: done");
    Ok(())
}
