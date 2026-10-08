//! Check 5 (task-32 / task-49): connected hosts install one native package from the prefix.dev
//! ecosystem channel.

use super::Failure;
use anyhow::Result;
use std::path::Path;

const CANONICAL_INSTALL: &str = "pixi global install --channel https://prefix.dev/archont561/archont561 --channel conda-forge pixi-sandbox";
const CANONICAL_CHANNEL: &str = "https://prefix.dev/archont561/archont561";

/// TASK-32 / TASK-49: connected hosts install one native package from the prefix.dev
/// ecosystem channel (`archont561/archont561`, which carries every published Archont561
/// package). Keep the primary README, installation guide, and generated workflow source
/// aligned; release binaries remain a separate transport-bootstrap surface.
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
    if !generated.contains(CANONICAL_CHANNEL)
        || !generated.contains("pixi global install")
        || !generated.contains("pixi-sandbox")
    {
        failures.push(Failure::new(
            "the generated publishing workflow does not install pixi-sandbox from the canonical channel (task-32)",
        ));
    }
    Ok(())
}
