//! Check 10 (task-39): `.github/workflows/relock.yml` is the committed render of the generator
//! `pixi-sandbox init` hands consumers, so the lane this repository runs is provably the lane
//! it ships; `xtask render-relock` is the only way to change it.

use super::Failure;
use super::support::rel;
use anyhow::Result;
use std::path::Path;

/// task-39: the relock lane is generated, and this repository commits its own render so the
/// artifact it runs is provably the artifact `pixi-sandbox init` writes for a consumer. Without
/// this check the committed copy would drift the moment the pixi pin moves or the template
/// changes, and the dogfooding claim would quietly become false — the v0.3.1 class of bug
/// (issue #37), where a generated workflow nobody ran was broken for a whole release.
pub(super) fn generated_relock_is_current(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let path = root.join(crate::workflow::RELOCK_PATH);
    let expected = crate::workflow::relock_render(root)?;
    let actual = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            failures.push(Failure::with(
                "the committed relock workflow is missing:",
                vec![format!("{}: {error}", rel(root, &path))],
                "run `pixi run xtask render-relock` to write it",
            ));
            return Ok(());
        }
    };
    if actual != expected {
        failures.push(Failure::with(
            "the committed relock workflow is not the generator's current render:",
            vec![format!(
                "{}: {} line(s) committed vs {} rendered",
                rel(root, &path),
                actual.lines().count(),
                expected.lines().count()
            )],
            "edit crates/pixi-sandbox/src/generated/relock_workflow.rs, never the workflow, then run `pixi run xtask render-relock`",
        ));
    }
    Ok(())
}
