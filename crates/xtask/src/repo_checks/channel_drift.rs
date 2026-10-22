//! Check 5: interactive installs use the canonical channel (TASK-32/49); generated CI
//! bootstraps verified standalone releases instead (ADR-30 / TASK-126).

use super::Failure;
use anyhow::Result;
use std::path::Path;

const CANONICAL_INSTALL: &str = "pixi global install --channel https://prefix.dev/archont561/archont561 --channel conda-forge pixi-sandbox";

/// TASK-32 / TASK-49: connected hosts install one native package from the prefix.dev
/// ecosystem channel (`archont561/archont561`, which carries every published Archont561
/// package). Keep the primary README, installation guide, and generated workflow source
/// aligned. ADR-30 checks the generated workflow's distinct standalone bootstrap below.
pub(super) fn canonical_channel_install(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    for path in ["README.md", "docs/src/content/docs/installation.mdx"] {
        let text = crate::util::read(&root.join(path))?;
        if !text.contains(CANONICAL_INSTALL) || !text.contains("pixi-sandbox init") {
            failures.push(Failure::new(format!(
                "{path} must show the canonical channel install followed by pixi-sandbox init (task-32)"
            )));
        }
    }
    let generated =
        crate::util::read(&root.join("crates/pixi-sandbox/src/generated/github_workflow.rs"))?;
    // Interactive installation still uses the canonical channel; generated CI has no
    // package-manager bootstrap any more (ADR-30 / TASK-126).
    if !generated.contains("crate::release::PIXI_SANDBOX_REPO")
        || !generated.contains("sha256sum --check selected.sha256")
        || !generated.contains("SHA256SUMS")
    {
        failures.push(Failure::new(
            "the generated publishing workflow must bootstrap from the canonical standalone release with SHA256SUMS verification (ADR-30)",
        ));
    }
    Ok(())
}
