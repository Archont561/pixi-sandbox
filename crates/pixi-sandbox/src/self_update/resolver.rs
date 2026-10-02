//! Which release to install, and why.
//!
//! Two modes, and the difference is the whole point of decision-4: `latest` is *discovery*,
//! permitted while preparing a reviewed upgrade, while an exact `--version X.Y.Z` is what every
//! committed workflow and every published transport names. The resolver records which of the
//! two produced the tag so the caller can say it out loud.

use anyhow::{Context, Result, bail};

use crate::release::ReleaseSource;

/// The release this run will install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The tag as GitHub spells it, e.g. `v0.4.4`.
    pub tag: String,
    /// The bare version, e.g. `0.4.4`.
    pub version: String,
    /// How the tag was chosen.
    pub selection: Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// No `--version`: whatever the project currently calls latest.
    Latest,
    /// An exact version the caller named.
    Pinned,
}

impl Selection {
    pub fn describe(self) -> &'static str {
        match self {
            Selection::Latest => "latest",
            Selection::Pinned => "pinned",
        }
    }
}

/// Resolve `requested` (or latest) against `source`.
pub fn resolve(
    source: &dyn ReleaseSource,
    repo: &str,
    requested: Option<&str>,
) -> Result<Resolved> {
    match requested {
        Some(requested) => {
            let version = normalise_version(requested)?;
            Ok(Resolved {
                tag: format!("v{version}"),
                version,
                selection: Selection::Pinned,
            })
        }
        None => {
            let tag = source
                .latest_tag(repo)
                .with_context(|| format!("resolving the latest pixi-sandbox release of {repo}"))?;
            let version = normalise_version(&tag).with_context(|| {
                format!("{repo}'s latest release tag {tag:?} is not a vX.Y.Z release")
            })?;
            Ok(Resolved {
                tag,
                version,
                selection: Selection::Latest,
            })
        }
    }
}

/// Accept `X.Y.Z` or `vX.Y.Z` and return the bare `X.Y.Z`.
///
/// Strict on purpose: a loose parser turns a typo into a 404 from the download step, where the
/// error no longer says which input was wrong.
pub fn normalise_version(input: &str) -> Result<String> {
    let bare = input.strip_prefix('v').unwrap_or(input);
    let parts: Vec<&str> = bare.split('.').collect();
    let valid = parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part.len() == 1 || !part.starts_with('0'))
        });
    if !valid {
        bail!("{input:?} is not an exact version; expected X.Y.Z (for example 0.4.4)");
    }
    Ok(bare.to_string())
}
