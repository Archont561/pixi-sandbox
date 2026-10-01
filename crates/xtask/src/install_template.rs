//! Render `templates/install.sh` into the `install.sh` release asset.
//!
//! The install one-liner is *rendered* at the tag rather than copied from a committed
//! `install.sh`, because a committed copy carries a VERSION default that only the release
//! knows the right value for, so it silently lags: the published v0.3.0 one-liner installed
//! v0.2.0 binaries (task-2). Rendering means the default IS the tag being released, and
//! "downloads a vX.Y.Z script, installs vX.Y-1.Z binaries" cannot be constructed.
//!
//! The rendered asset is deliberately NOT added to SHA256SUMS — that file stays the binary
//! contract setup-pixi-sandbox reads, and install.sh verifies the binary it downloads against
//! it. install.sh itself travels over `curl | sh` (trust-on-first-use via TLS), so it carries
//! no checksum of its own.

use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

const PLACEHOLDER_LINE: &str = "VERSION=${PIXI_SANDBOX_VERSION:-__VERSION__}";
const PLACEHOLDER: &str = "__VERSION__";

/// Stamp `tag` into the template. Pure so a fixture can prove every refusal: the tag shape,
/// the placeholder's presence, the shebang, and that nothing but the VERSION line changes.
pub fn render(template: &str, tag: &str) -> Result<String> {
    // The version flows into a download URL; keep it a strict `vX.Y.Z`.
    let semver = tag.strip_prefix('v').unwrap_or("");
    if !crate::util::is_strict_semver(semver) {
        bail!("tag '{tag}' is not vX.Y.Z");
    }

    if !template.contains(PLACEHOLDER_LINE) {
        bail!("could not stamp {tag} — templates/install.sh has no `{PLACEHOLDER_LINE}` line");
    }
    let stamped_line = format!("VERSION=${{PIXI_SANDBOX_VERSION:-{tag}}}");
    let rendered = template.replace(PLACEHOLDER_LINE, &stamped_line);

    // Refuse to ship anything but a script that will actually do the right thing.
    if rendered.contains(PLACEHOLDER) {
        bail!(
            "{PLACEHOLDER} placeholder survived the render — a second placeholder sits outside the VERSION line"
        );
    }
    if rendered.lines().next() != Some("#!/bin/sh") {
        bail!("the rendered install.sh lost its #!/bin/sh shebang");
    }
    Ok(rendered)
}

/// `xtask render-install <TAG> <OUTPUT>`: read the template from the repository, render, and
/// write the executable asset. `sh -n` double-checks the result where an `sh` exists (every
/// runner that publishes a release has one; a local Windows box merely skips the parse check).
pub fn render_to_file(root: &Path, tag: &str, output: &Path) -> Result<()> {
    let template_path = root.join("templates/install.sh");
    if !template_path.is_file() {
        bail!(
            "templates/install.sh not found at {}",
            template_path.display()
        );
    }
    let rendered = render(&crate::util::read(&template_path)?, tag)?;
    crate::util::write_atomic(output, &rendered)?;

    match Command::new("sh").arg("-n").arg(output).status() {
        Ok(status) if !status.success() => {
            bail!("{} is not valid POSIX sh", output.display())
        }
        Ok(_) => {}
        Err(_) => eprintln!(
            "::notice::no `sh` on this host; skipping the syntax re-check of the rendered asset"
        ),
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))
            .with_context(|| format!("marking {} executable", output.display()))?;
    }

    eprintln!("→ {} (default VERSION={tag})", output.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{PLACEHOLDER_LINE, render};

    fn template() -> String {
        format!("#!/bin/sh\nset -eu\n{PLACEHOLDER_LINE}\necho \"$VERSION\"\n")
    }

    #[test]
    fn rendering_stamps_exactly_the_version_line() {
        let rendered = render(&template(), "v1.2.3").expect("render");
        assert!(rendered.contains("VERSION=${PIXI_SANDBOX_VERSION:-v1.2.3}"));
        // Everything but the VERSION line survives byte for byte.
        let keep = |s: &str| {
            s.lines()
                .filter(|l| !l.starts_with("VERSION="))
                .collect::<Vec<_>>()
                .join("\n")
        };
        assert_eq!(keep(&template()), keep(&rendered));
    }

    #[test]
    fn a_tag_that_is_not_strict_semver_is_refused() {
        for bad in ["1.2.3", "v1.2", "v1.2.3-rc1", "latest"] {
            assert!(render(&template(), bad).is_err(), "{bad} must be refused");
        }
    }

    #[test]
    fn a_template_without_the_placeholder_line_is_refused() {
        let err = render("#!/bin/sh\nVERSION=v0.0.0\n", "v1.2.3").expect_err("must fail");
        assert!(format!("{err:#}").contains("no `VERSION="));
    }

    #[test]
    fn a_surviving_placeholder_outside_the_version_line_is_refused() {
        let sneaky = format!("#!/bin/sh\n{PLACEHOLDER_LINE}\necho __VERSION__\n");
        assert!(render(&sneaky, "v1.2.3").is_err());
    }

    #[test]
    fn a_lost_shebang_is_refused() {
        let headless = format!("# no shebang\n{PLACEHOLDER_LINE}\n");
        assert!(render(&headless, "v1.2.3").is_err());
    }
}
