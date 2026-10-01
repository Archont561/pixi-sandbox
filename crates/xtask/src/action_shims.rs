//! Render `setup/action.yml` and `publish/action.yml` from the single implementation in the
//! repository-root `action.yml`.
//!
//! Why a renderer and not a two-line shim that calls the root action (task-12, issue #37):
//! `setup/action.yml` and `publish/action.yml` used to contain one step, `uses: ../action.yml`.
//! That cannot work from a remote ref. A relative `uses:` inside a composite action is resolved
//! against the *workflow run's workspace*, not against the repository that defines the action,
//! and a `../` prefix is rejected outright by the runner's reference parser, so every
//! `Archont561/pixi-sandbox/setup@<ref>` caller died during job setup. Hardcoding
//! `Archont561/pixi-sandbox@<sha>` inside the shim defeats the caller's own SHA pin and cannot
//! be tested before it is tagged.
//!
//! That leaves one honest option: each published action path is a complete composite action.
//! Duplication is the mechanism, not the design — the copies are generated here and
//! `xtask check-repository` re-renders them and fails on any drift, exactly as
//! `templates/install.sh` + `render-install` make a stale release asset unconstructible.
//!
//! The transform is deliberately tiny, so a reviewer can hold it in their head:
//!   1. the root's own header (name/description/author/branding + its explanatory comment) is
//!      replaced by the variant's header, because the marketplace metadata is the one thing
//!      that legitimately differs between the three entry points;
//!   2. everything from `inputs:` to EOF is copied byte for byte;
//!   3. for `publish`, the `subpath` input's default flips from `setup` to `publish`, which is
//!      what selects the behaviour at run time.

use anyhow::{Result, bail};
use std::fmt;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Setup,
    Publish,
}

impl Variant {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "setup" => Ok(Self::Setup),
            "publish" => Ok(Self::Publish),
            other => bail!("unknown variant '{other}' (expected setup or publish)"),
        }
    }
}

impl fmt::Display for Variant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Setup => "setup",
            Self::Publish => "publish",
        })
    }
}

/// The banner names the source and the renderer so that an editor who opens the generated
/// file first is told where to go, and the lint that enforces it.
fn header(variant: Variant) -> String {
    let banner = format!(
        "# GENERATED FILE — do not edit. Source: ../action.yml (rendered by `pixi run render-action-shims`).\n\
         # Regenerate with: pixi run render-action-shims\n\
         #\n\
         # This is a full copy of the root action, not a shim that calls it: a relative `uses:` inside a\n\
         # remote composite action resolves against the caller's workspace, so `uses: ../action.yml` made\n\
         # every `Archont561/pixi-sandbox/{variant}@<ref>` caller fail at job setup (issue #37). The repository\n\
         # consistency gate (`pixi run lint-repo-consistency`) re-renders this file and fails if it has\n\
         # drifted from ../action.yml.\n"
    );
    let metadata = match variant {
        Variant::Setup => {
            "name: Setup pixi-sandbox\n\
             description: Download, checksum-verify, and expose a standalone pixi-sandbox release binary. Use as Archont561/pixi-sandbox/setup@vX (or root Archont561/pixi-sandbox@vX).\n\
             author: pixi-sandbox\n\
             branding:\n\
             \x20 icon: package\n\
             \x20 color: orange\n"
        }
        Variant::Publish => {
            "name: Publish Pixi sandbox bundle\n\
             description: Install requested Pixi environments, pack with a verified release binary, verify, and publish one orphan branch. Use as Archont561/pixi-sandbox/publish@vX.\n\
             author: pixi-sandbox\n\
             branding:\n\
             \x20 icon: upload-cloud\n\
             \x20 color: blue\n"
        }
    };
    format!("{banner}{metadata}\n")
}

/// Pure transform: variant header + the root action's `inputs:`-to-EOF body, with the publish
/// copy's `subpath` default flipped. Refuses a root action it cannot carve up.
pub fn render(root_action: &str, variant: Variant) -> Result<String> {
    let Some(body_start) = body_offset(root_action) else {
        bail!("action.yml has no top-level inputs: block");
    };

    let mut body = root_action[body_start..].to_string();
    if variant == Variant::Publish {
        const SETUP_DEFAULT: &str = "\n    default: setup\n";
        let Some(at) = body.find(SETUP_DEFAULT) else {
            bail!("action.yml has no `default: setup` for the subpath input");
        };
        body.replace_range(at..at + SETUP_DEFAULT.len(), "\n    default: publish\n");
    }

    let rendered = format!("{}{}", header(variant), body);
    check(&rendered, variant)?;
    Ok(rendered)
}

/// Byte offset of the `inputs:` line (column 0), from which the body is copied verbatim.
fn body_offset(root_action: &str) -> Option<usize> {
    if root_action.starts_with("inputs:\n") {
        return Some(0);
    }
    root_action.find("\ninputs:\n").map(|at| at + 1)
}

/// A generated action that is not self-contained, or that lost the mode switch, is the bug
/// this renderer exists to prevent: refuse to emit it.
fn check(rendered: &str, variant: Variant) -> Result<()> {
    if !rendered.lines().any(|l| l == "  using: composite") {
        bail!("rendered {variant} action is not a composite action");
    }
    if !rendered
        .lines()
        .any(|l| l == format!("    default: {variant}"))
    {
        bail!("rendered {variant} action does not default to subpath={variant}");
    }
    if let Some(relative) = relative_uses(rendered) {
        bail!(
            "rendered {variant} action contains a relative uses:, which cannot resolve from a remote ref: {relative}"
        );
    }
    Ok(())
}

/// The first `uses:` whose target starts with a dot — the exact shape that cannot resolve
/// from a remote ref. Shared with `check-repository`'s published-action check.
pub fn relative_uses(action: &str) -> Option<String> {
    for line in action.lines() {
        let trimmed = line.trim_start();
        let step = trimmed
            .strip_prefix("- ")
            .map(str::trim_start)
            .unwrap_or(trimmed);
        if let Some(target) = step.strip_prefix("uses:") {
            if target.trim_start().starts_with('.') {
                return Some(line.to_string());
            }
        }
    }
    None
}

/// `xtask render-action-shims [variant] [output]`: no arguments rewrites both generated
/// copies in place; a variant prints to stdout or writes `output`.
pub fn emit(root: &Path, variant: Option<&str>, output: Option<&Path>) -> Result<()> {
    let root_action_path = root.join("action.yml");
    if !root_action_path.is_file() {
        bail!("no root action.yml at {}", root_action_path.display());
    }
    let root_action = crate::util::read(&root_action_path)?;

    let emit_one = |variant: Variant, output: Option<&Path>| -> Result<()> {
        let rendered = render(&root_action, variant)?;
        match output {
            None => print!("{rendered}"),
            Some(path) => {
                crate::util::write_atomic(path, &rendered)?;
                eprintln!("→ {} (subpath default: {variant})", path.display());
            }
        }
        Ok(())
    };

    match variant {
        None => {
            emit_one(Variant::Setup, Some(&root.join("setup/action.yml")))?;
            emit_one(Variant::Publish, Some(&root.join("publish/action.yml")))
        }
        Some(name) => emit_one(Variant::parse(name)?, output),
    }
}

#[cfg(test)]
mod tests {
    use super::{Variant, relative_uses, render};

    fn root_action() -> &'static str {
        "name: root\ndescription: the one implementation\n\n# commentary that must not survive\ninputs:\n  subpath:\n    required: false\n    default: setup\nruns:\n  using: composite\n  steps:\n    - shell: bash\n      run: echo hi\n"
    }

    #[test]
    fn the_setup_copy_keeps_the_body_and_swaps_the_header() {
        let rendered = render(root_action(), Variant::Setup).expect("render");
        assert!(rendered.starts_with("# GENERATED FILE"));
        assert!(rendered.contains("name: Setup pixi-sandbox"));
        assert!(!rendered.contains("commentary that must not survive"));
        assert!(rendered.contains("    default: setup\n"));
        assert!(rendered.ends_with("      run: echo hi\n"));
    }

    #[test]
    fn the_publish_copy_flips_exactly_the_subpath_default() {
        let rendered = render(root_action(), Variant::Publish).expect("render");
        assert!(rendered.contains("    default: publish\n"));
        assert!(!rendered.contains("    default: setup\n"));
        assert!(rendered.contains("name: Publish Pixi sandbox bundle"));
    }

    #[test]
    fn a_root_without_inputs_or_without_the_default_is_refused() {
        assert!(render("name: x\nruns: {}\n", Variant::Setup).is_err());
        let no_default = "inputs:\n  subpath:\n    required: false\nruns:\n  using: composite\n";
        assert!(render(no_default, Variant::Publish).is_err());
    }

    #[test]
    fn a_relative_uses_is_spotted_in_both_step_spellings() {
        assert!(relative_uses("steps:\n  - uses: ../action.yml\n").is_some());
        assert!(relative_uses("runs:\n  uses: ./local\n").is_some());
        assert!(relative_uses("steps:\n  - uses: owner/repo@abc\n").is_none());
    }
}
