//! The committed payload declaration a `wrap-payload` run reads (TASK-78, ADR-28).
//!
//! A payload is a prebuilt file or directory that is fetched on a connected host and wrapped as
//! a conda package. This module is the *reviewed data* half: it parses the declaration, checks
//! everything that can be checked before any bytes move (names, platform, the source URL and its
//! declared SHA256, the relative paths), and hands the caller a [`PayloadSpec`] plus the exact
//! `prepare` command and output paths.
//!
//! The declaration is its own file — `pixi-sandbox-payload.toml`, passed to `wrap-payload
//! --config` — rather than a table inside `.pixi-sandbox.toml`. The sandbox config is
//! `deny_unknown_fields` and versioned: adding `[payload]` there would make every older binary
//! refuse an otherwise valid config with an opaque parse error unless the schema were bumped.
//! Keeping the payload declaration separate also matches the boundary the guide states — a
//! channel package carries one dependency, the transport branch carries a whole environment.
//!
//! The declared `source.sha256` is the SHA256 of the *fetched artifact* (the bytes a `prepare`
//! command downloads); `artifact` names where those bytes land, and `payload` names the file or
//! directory that is actually packaged. AC#13's gate is
//! [`verify_artifact`](crate::payload::verify_artifact): a changed upstream fails the run
//! instead of packing different bytes behind a green pipeline.

use crate::payload::{PayloadError, PayloadSpec, validate_source, validate_spec_identity};
use crate::platform::Platform;
use serde::Deserialize;
use std::path::Path;

/// Schema of [`PayloadConfig`]. Bumped only for incompatible changes; readers refuse anything
/// newer, mirroring `manifest::SCHEMA_VERSION` and `sandbox_config::CONFIG_SCHEMA`.
pub const PAYLOAD_CONFIG_SCHEMA: u32 = 1;

/// Default file name the `wrap-payload` verb reads when no `--config` is given.
pub const DEFAULT_FILE: &str = "pixi-sandbox-payload.toml";

/// A checked-in payload declaration: what to fetch, how, and what to wrap.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PayloadConfig {
    pub schema: u32,
    /// The conda package name.
    pub name: String,
    /// The conda package version.
    pub version: String,
    /// The one platform subdir, e.g. `linux-64`.
    pub platform: String,
    /// Declared `noarch` (refused for a payload that carries a native executable).
    #[serde(default)]
    pub noarch: bool,
    /// An optional payload-relative executable to expose on `PATH` as a launcher.
    #[serde(default)]
    pub entry_point: Option<String>,
    /// The config-declared command that fetches the payload. It does the network; pixi-sandbox
    /// never downloads the payload itself (AC#13).
    pub prepare: String,
    /// Directory the `prepare` command runs in, relative to the config file's directory.
    /// Defaults to `.pixi/payload-work`, keeping fetched inputs out of the project working tree.
    #[serde(default = "default_working_dir")]
    pub working_dir: String,
    /// The file the `prepare` command leaves the fetched bytes at, relative to `working_dir`.
    /// Its SHA256 is checked against `source.sha256`.
    pub artifact: String,
    /// The file or directory to package, relative to `working_dir`. Usually the artifact
    /// unpacked; may equal `artifact` for a payload that is a single file.
    pub payload: String,
    /// Upstream provenance: the source URL (recorded) and the artifact's expected SHA256 (checked).
    pub source: SourceDecl,
}

/// The upstream source of the artifact, as `[source]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceDecl {
    /// An `https://` URL, recorded in the package's provenance document.
    pub url: String,
    /// The expected SHA256 of the fetched artifact, lowercase hexadecimal.
    pub sha256: String,
}

fn default_working_dir() -> String {
    ".pixi/payload-work".to_string()
}

impl PayloadConfig {
    /// Read and validate a payload declaration.
    ///
    /// # Errors
    ///
    /// Returns [`PayloadError::Io`] when the file cannot be read, [`PayloadError::InvalidConfig`]
    /// when it is not valid TOML or carries an unusable relative path, the schema/digest/identity
    /// refusals, or [`PayloadError::InvalidPlatform`].
    pub fn load(path: &Path) -> Result<Self, PayloadError> {
        let text = std::fs::read_to_string(path).map_err(|error| PayloadError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        let config: PayloadConfig = toml::from_str(&text).map_err(|error| {
            PayloadError::InvalidConfig(format!(
                "{} is not a valid payload config: {error}",
                path.display()
            ))
        })?;
        config.validate()?;
        Ok(config)
    }

    /// Validate everything checkable without touching the network or the payload tree.
    ///
    /// # Errors
    ///
    /// A schema newer than this build, an empty `prepare`, a non-relative output path, an
    /// invalid package identity, an invalid platform, or an invalid source URL / SHA256.
    pub fn validate(&self) -> Result<(), PayloadError> {
        if self.schema == 0 || self.schema > PAYLOAD_CONFIG_SCHEMA {
            return Err(PayloadError::InvalidConfig(format!(
                "payload config schema {} is not supported by this build (understands 1..={PAYLOAD_CONFIG_SCHEMA})",
                self.schema
            )));
        }
        if self.prepare.trim().is_empty() {
            return Err(PayloadError::InvalidConfig(
                "`prepare` must be a non-empty command; it is the only thing that fetches the payload".to_string(),
            ));
        }
        for (field, value) in [
            ("working_dir", &self.working_dir),
            ("artifact", &self.artifact),
            ("payload", &self.payload),
        ] {
            validate_relative_output(field, value)?;
        }
        self.spec()?;
        validate_source(&self.source.url, &self.source.sha256)
    }

    /// The [`PayloadSpec`] the pure rules take, with the platform parsed and the identity checked.
    ///
    /// # Errors
    ///
    /// [`PayloadError::InvalidPlatform`] for an unknown platform, or the identity refusals from
    /// [`validate_spec_identity`].
    pub fn spec(&self) -> Result<PayloadSpec, PayloadError> {
        let platform = self
            .platform
            .parse::<Platform>()
            .map_err(|_| PayloadError::InvalidPlatform(self.platform.clone()))?;
        let spec = PayloadSpec {
            name: self.name.clone(),
            version: self.version.clone(),
            platform,
            noarch: self.noarch,
            entry_point: self.entry_point.clone(),
        };
        validate_spec_identity(&spec)?;
        Ok(spec)
    }
}

/// A config output path must be relative and stay inside `working_dir`: a committed file may not
/// carry a machine-specific absolute path, and `..` would make the config's meaning depend on
/// where it sits (AC#11's spirit, applied to the payload config's own paths).
fn validate_relative_output(field: &str, value: &str) -> Result<(), PayloadError> {
    let bare = value.trim();
    if bare.is_empty()
        || bare.starts_with('/')
        || bare.contains('\\')
        || bare.split('/').any(|component| component == "..")
    {
        return Err(PayloadError::InvalidConfig(format!(
            "`{field}` must be a non-empty path relative to working_dir with no `\\` or `..`, got {value:?}"
        )));
    }
    Ok(())
}
