//! Check 2 (task-7): the README Platforms badge, `pixi.toml` and `.pixi-sandbox.toml` tell
//! one platform story, and the Windows gap is documented rather than advertised.

use super::Failure;
use anyhow::Result;
use regex::Regex;
use std::collections::BTreeSet;
use std::path::Path;

/// task-7: the README Platforms badge says exactly what `pixi.toml` declares, every platform
/// `.pixi-sandbox.toml` publishes is one of them, and the Windows gap (no conda-forge `bun`,
/// D11) is documented rather than advertised by the badge.
pub(super) fn platform_claims(root: &Path, failures: &mut Vec<Failure>) -> Result<()> {
    let declared = workspace_platforms(root)?;
    let readme = crate::util::read(&root.join("README.md"))?;

    match badge_platforms(&readme) {
        None => failures.push(Failure::new(
            "README.md has no parsable Platforms badge (task-7)",
        )),
        Some(badge) => {
            if badge != declared {
                failures.push(Failure::new(format!(
                    "the Platforms badge advertises '{}' but pixi.toml declares '{}' (task-7)",
                    join(&badge),
                    join(&declared)
                )));
            }
            if badge.contains("win-64") {
                failures.push(Failure::new(
                    "the Platforms badge claims win-64, which is neither declared nor proven (task-7)",
                ));
            }
        }
    }

    for platform in published_platforms(root)? {
        if !declared.contains(&platform) {
            failures.push(Failure::new(format!(
                ".pixi-sandbox.toml publishes {platform}, which pixi.toml does not declare (task-7)"
            )));
        }
    }

    // Windows is not dropped in silence: one line has to carry the whole explanation, or the
    // reader has to assemble it — the platform, the blocker (`bun` as a word; `bundle` does
    // not count) and the decision that gates it.
    let gap =
        Regex::new(r"win-64.*[^a-z]bun([^a-z]|$).*[^A-Za-z]D11([^0-9]|$)").expect("static regex");
    if !readme.lines().any(|l| gap.is_match(l)) {
        failures.push(Failure::new(
            "README.md does not document the Windows gap on one line: name win-64, the conda-forge bun blocker and D11 (task-7)",
        ));
    }
    Ok(())
}

fn join(set: &BTreeSet<String>) -> String {
    set.iter().cloned().collect::<Vec<_>>().join(" ")
}

fn workspace_platforms(root: &Path) -> Result<BTreeSet<String>> {
    let manifest: toml::Table = crate::util::read(&root.join("pixi.toml"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("parsing pixi.toml: {e}"))?;
    Ok(string_array(
        manifest.get("workspace").and_then(|w| w.get("platforms")),
    ))
}

/// Every platform any bundle in the publish plan targets, parsed as TOML rather than scraped
/// by line shape (the retired pipeline silently read only one bundle).
fn published_platforms(root: &Path) -> Result<BTreeSet<String>> {
    let plan: toml::Table = crate::util::read(&root.join(".pixi-sandbox.toml"))?
        .parse()
        .map_err(|e| anyhow::anyhow!("parsing .pixi-sandbox.toml: {e}"))?;
    let mut platforms = BTreeSet::new();
    if let Some(bundles) = plan.get("bundle").and_then(|b| b.as_array()) {
        for bundle in bundles {
            platforms.extend(string_array(bundle.get("platforms")));
        }
    }
    Ok(platforms)
}

fn string_array(value: Option<&toml::Value>) -> BTreeSet<String> {
    value
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// The badge encodes platforms as `Platforms-linux--64%20%7C%20osx--arm64-brightgreen.svg`:
/// `%20%7C%20` separates entries and shields.io doubles a literal hyphen.
fn badge_platforms(readme: &str) -> Option<BTreeSet<String>> {
    let badge =
        Regex::new(r"img\.shields\.io/badge/Platforms-(.*)-[A-Za-z]*\.svg").expect("static regex");
    let encoded = badge.captures(readme)?.get(1)?.as_str();
    let decoded = encoded
        .replace("%20%7C%20", " ")
        .replace("%20", " ")
        .replace("%7C", " ")
        .replace("--", "-");
    Some(decoded.split_whitespace().map(str::to_string).collect())
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{headlines, valid_fixture};
    use std::fs;

    #[test]
    fn a_badge_that_disagrees_with_the_workspace_fires_check_2() {
        let dir = valid_fixture();
        let readme = fs::read_to_string(dir.path().join("README.md")).expect("readme");
        fs::write(
            dir.path().join("README.md"),
            readme.replace("osx--arm64", "win--64"),
        )
        .expect("readme");
        let found = headlines(dir.path());
        assert!(
            found
                .iter()
                .any(|h| h.contains("Platforms badge advertises")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|h| h.contains("claims win-64")),
            "{found:?}"
        );
    }

    #[test]
    fn a_published_platform_the_workspace_lacks_fires_check_2() {
        let dir = valid_fixture();
        fs::write(
            dir.path().join(".pixi-sandbox.toml"),
            "schema = 1\n\n[[bundle]]\nname = \"developer\"\nplatforms = [\"linux-aarch64\"]\n",
        )
        .expect("plan");
        let found = headlines(dir.path());
        assert!(
            found.iter().any(|h| h.contains("publishes linux-aarch64")),
            "{found:?}"
        );
    }
}
