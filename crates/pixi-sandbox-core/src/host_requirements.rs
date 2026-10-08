//! `[host_requirements]` — what a project's workloads need from the *host* (issue #109,
//! TASK-75).
//!
//! A transport restores a Pixi environment. It cannot restore fonts, an X server, D-Bus or GPU
//! access, because none of those live inside the environment: a GUI workload fails *after* a
//! restore that looks perfectly healthy, and the operator is left reading a stack trace inside
//! a sandbox that reported success. This module is the declaration half of the answer — the
//! project states, per host family, what the workloads need; `doctor` probes the machine and
//! reports each item as satisfied, missing, unknown or not applicable (the probes live in the
//! CLI crate; everything here is reviewed data and resolution, so it can be tested in a
//! tempdir on any platform).
//!
//! Three properties are the whole design:
//!
//! - **It never installs anything.** Every entry names something the host must already provide;
//!   the tool reports, it does not mutate the machine. Bundling a display server, a GPU driver
//!   or an OS service into a Pixi environment is explicitly not a supported outcome, and the
//!   documentation says so.
//! - **It is scoped to a host family.** A glibc floor or a package manager is an OS fact rather
//!   than a per-architecture one, so declarations are shared or overridden for
//!   [`HostFamily::Linux`], [`HostFamily::Osx`] or [`HostFamily::Windows`] and then apply to
//!   every Pixi platform in that family (see [`HostRequirements::resolved_for`]).
//! - **It travels in the transport.** The resolved set for a bundle's platform is written into
//!   `manifest.json`, so a restored branch can still say what it needs without the source
//!   checkout (TASK-75's manifest slice; this module is declaration and resolution only).
//!
//! Named `host_requirements` rather than pixi's `[system-requirements]` on purpose: pixi's
//! table constrains the *solver* (it declares virtual packages such as `__glibc` that conda
//! builds are selected against), while everything here is host state that the sandbox does not
//! own and never provides. The two tables live in different files and answer different
//! questions; sharing the name would blur exactly the boundary this feature exists to draw.

use crate::error::{Error, Result};
use crate::platform::HostFamily;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// A host capability a workload can require but a transport can never provide.
///
/// A closed set, enforced at load: `capabilities = ["dipslay"]` is a typo, not a requirement,
/// and a probe that silently skipped it would produce the false confidence this table exists to
/// remove. [`HostCapability::Display`] is satisfied by a display the process can actually use
/// (`DISPLAY` or `WAYLAND_DISPLAY`) or by one of the `headless` providers;
/// [`HostCapability::Gpu`] is only ever reported, never granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HostCapability {
    Display,
    Gpu,
}

impl HostCapability {
    /// Every capability this build knows, in documentation order.
    pub const ALL: [HostCapability; 2] = [HostCapability::Display, HostCapability::Gpu];

    /// The spelling used in the config and in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            HostCapability::Display => "display",
            HostCapability::Gpu => "gpu",
        }
    }

    /// One line an operator can act on, used by the human report.
    #[must_use]
    pub const fn expectation(self) -> &'static str {
        match self {
            HostCapability::Display => {
                "a usable display (DISPLAY or WAYLAND_DISPLAY) or a headless provider"
            }
            HostCapability::Gpu => "GPU access for the invoking user (a working driver)",
        }
    }
}

impl fmt::Display for HostCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One host family's requirements: the body of a `[host_requirements.<family>]` section, and —
/// after [`HostRequirements::resolved_for`] — the exact set a transport carries.
///
/// `libc` is a floor (`">=2.34"`, or a bare `"2.34"`), the three lists name host packages,
/// host services and host programs respectively, and `capabilities` names what the workload
/// needs from the machine itself. Nothing here is installed by pixi-sandbox.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRequirementSet {
    /// Minimum C runtime the workload needs, as `major.minor[.patch]` with an optional `>=`.
    /// Linux-only in practice, which is why the two non-Linux sections refuse it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub libc: Option<String>,
    /// Host packages (distro-level) the workload links against or loads at runtime.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    /// Host services that must be available (for example a D-Bus session bus).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<String>,
    /// Capabilities that must hold on the machine running the workload.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<HostCapability>,
    /// Headless display providers: programs on `PATH`, any *one* of which satisfies a display
    /// requirement when there is no physical display (`xvfb-run` is the canonical entry).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headless: Vec<String>,
}

impl HostRequirementSet {
    /// True when the set declares nothing at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.libc.is_none()
            && self.packages.is_empty()
            && self.services.is_empty()
            && self.capabilities.is_empty()
            && self.headless.is_empty()
    }

    /// The parsed `libc` floor, when one is declared.
    ///
    /// # Errors
    /// Returns an error for a spec [`LibcFloor::parse`] does not accept. A config that passed
    /// [`HostRequirements::validate`] can only fail here if the spec was left unvalidated, so
    /// callers that hold a loaded config may treat the failure as a bug.
    pub fn libc_floor(&self) -> Result<Option<LibcFloor>> {
        self.libc.as_deref().map(LibcFloor::parse).transpose()
    }

    /// Validate one requirement set, naming `field` (the TOML path) in every error.
    ///
    /// # Errors
    /// Returns an error for an empty or whitespace-bearing entry, a duplicate entry, or a
    /// `libc` spec outside [`LibcFloor::parse`]'s grammar.
    pub fn validate(&self, field: &str) -> Result<()> {
        if let Some(spec) = &self.libc {
            if let Err(error) = LibcFloor::parse(spec) {
                return Err(Error::Invalid(format!("[{field}] {error}")));
            }
        }
        for (key, entries) in [
            ("packages", &self.packages),
            ("services", &self.services),
            ("headless", &self.headless),
        ] {
            validate_entries(&format!("{field}.{key}"), entries)?;
        }
        validate_capabilities(field, &self.capabilities)?;
        Ok(())
    }
}

/// The `[host_requirements]` table: declarations shared by every family, plus optional
/// per-family sections that refine them.
///
/// The two levels are resolved by [`HostRequirements::resolved_for`], which is the only
/// resolution rule: the lists accumulate (shared entries first, duplicates dropped) and a
/// family's `libc` replaces the shared one. A family section cannot remove a shared entry —
/// a project that wants a requirement on one family only declares it in that family's section
/// alone, and that is the supported way to scope it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostRequirements {
    /// Minimum C runtime, shared by every family (Linux enforces it; see the sections below).
    #[serde(default)]
    pub libc: Option<String>,
    /// Host packages required on every family.
    #[serde(default)]
    pub packages: Vec<String>,
    /// Host services required on every family.
    #[serde(default)]
    pub services: Vec<String>,
    /// Host capabilities required on every family.
    #[serde(default)]
    pub capabilities: Vec<HostCapability>,
    /// Headless display providers accepted on every family.
    #[serde(default)]
    pub headless: Vec<String>,
    /// `[host_requirements.linux]` — Linux-specific requirements and overrides.
    #[serde(default)]
    pub linux: Option<HostRequirementSet>,
    /// `[host_requirements.osx]` — macOS-specific requirements and overrides.
    #[serde(default)]
    pub osx: Option<HostRequirementSet>,
    /// `[host_requirements.windows]` — Windows-specific requirements and overrides.
    #[serde(default)]
    pub windows: Option<HostRequirementSet>,
}

impl HostRequirements {
    /// The shared declarations, without any family section.
    #[must_use]
    pub fn shared(&self) -> HostRequirementSet {
        // Exhaustive destructuring on purpose: a requirement kind added to either struct stops
        // this compiling, so resolution can never silently drop a new kind.
        let HostRequirements {
            libc,
            packages,
            services,
            capabilities,
            headless,
            linux: _,
            osx: _,
            windows: _,
        } = self;
        HostRequirementSet {
            libc: libc.clone(),
            packages: packages.clone(),
            services: services.clone(),
            capabilities: capabilities.clone(),
            headless: headless.clone(),
        }
    }

    /// The section declared for `family`, when there is one.
    #[must_use]
    pub fn section(&self, family: HostFamily) -> Option<&HostRequirementSet> {
        match family {
            HostFamily::Linux => self.linux.as_ref(),
            HostFamily::Osx => self.osx.as_ref(),
            HostFamily::Windows => self.windows.as_ref(),
        }
    }

    /// The requirement set a transport for a platform in `family` carries.
    ///
    /// The shared declarations always apply; the family's section adds to them and wins on
    /// `libc`. Order is preserved (shared entries first, then the section's new ones) so the
    /// rendered report matches the reviewed config.
    #[must_use]
    pub fn resolved_for(&self, family: HostFamily) -> HostRequirementSet {
        let shared = self.shared();
        let Some(section) = self.section(family) else {
            return shared;
        };
        // Exhaustive destructuring again: a new kind must be resolved explicitly here.
        let HostRequirementSet {
            libc,
            packages,
            services,
            capabilities,
            headless,
        } = shared;
        HostRequirementSet {
            libc: section.libc.clone().or(libc),
            packages: union(&packages, &section.packages),
            services: union(&services, &section.services),
            capabilities: union(&capabilities, &section.capabilities),
            headless: union(&headless, &section.headless),
        }
    }

    /// True when the table declares nothing at all, sections included.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let sections_empty = HostFamily::ALL
            .iter()
            .all(|family| match self.section(*family) {
                Some(section) => section.is_empty(),
                None => true,
            });
        sections_empty && self.shared().is_empty()
    }

    /// Validate the whole table.
    ///
    /// # Errors
    /// Returns an error for an invalid requirement set, an empty table (a table that declares
    /// nothing is a typo, not a declaration), an empty family section, or a `libc` floor in the
    /// `osx`/`windows` sections, where it cannot mean anything.
    pub fn validate(&self) -> Result<()> {
        if self.is_empty() {
            return Err(Error::Invalid(
                "[host_requirements] declares no requirements; remove the table or list what the \
                 workloads need from the host (libc, packages, services, capabilities, headless)"
                    .to_string(),
            ));
        }
        self.shared().validate("host_requirements")?;
        for family in HostFamily::ALL {
            let Some(section) = self.section(family) else {
                continue;
            };
            let field = format!("host_requirements.{}", family.as_str());
            if section.is_empty() {
                return Err(Error::Invalid(format!(
                    "[{field}] declares nothing; remove the section or list what this family \
                     needs beyond the shared [host_requirements] entries"
                )));
            }
            section.validate(&field)?;
            if family != HostFamily::Linux && section.libc.is_some() {
                return Err(Error::Invalid(format!(
                    "[{field}] declares libc, but a libc floor only applies to Linux hosts: \
                     declare it in [host_requirements] or in [host_requirements.linux]"
                )));
            }
        }
        Ok(())
    }
}

/// Declaration order, duplicates dropped: the shared entries first, then whatever the section
/// adds.
fn union<T: Ord + Clone>(shared: &[T], extra: &[T]) -> Vec<T> {
    let mut seen = BTreeSet::new();
    shared
        .iter()
        .chain(extra)
        .filter(|entry| seen.insert((*entry).clone()))
        .cloned()
        .collect()
}

/// A host package, service or program name: one token, no whitespace, listed once.
fn validate_entries(field: &str, entries: &[String]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for entry in entries {
        if entry.is_empty() || entry.chars().any(char::is_whitespace) {
            return Err(Error::Invalid(format!(
                "[{field}] lists {entry:?}, which is not one host name: entries are single \
                 names without whitespace"
            )));
        }
        if !seen.insert(entry.as_str()) {
            return Err(Error::Invalid(format!("[{field}] lists {entry:?} twice")));
        }
    }
    Ok(())
}
/// Capabilities are a closed set, so an unknown value is named as such rather than skipped.
fn validate_capabilities(field: &str, capabilities: &[HostCapability]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for capability in capabilities {
        if !seen.insert(*capability) {
            return Err(Error::Invalid(format!(
                "[{field}] lists the capability {capability:?} twice"
            )));
        }
    }
    Ok(())
}

/// A parsed `libc` floor.
///
/// The grammar is deliberately one-sided: a floor states what the host must be *at least*, so
/// `"2.34"` and `">=2.34"` mean the same thing (the issue's example and the obvious spelling),
/// and every other operator is refused rather than approximated. `patch` is optional because
/// glibc releases are normally named `major.minor` (`2.39`), and a floor of `2.39` has to
/// compare equal to a host that reports `2.39.0`.
#[derive(Debug, Clone, Copy)]
pub struct LibcFloor {
    major: u32,
    minor: u32,
    patch: Option<u32>,
}

impl LibcFloor {
    /// Parse a floor spec: `major.minor` or `major.minor.patch`, optionally prefixed `>=`.
    ///
    /// # Errors
    /// Returns an error naming the accepted spellings for anything else — a missing component,
    /// a non-numeric component, a different operator (`<`, `==`, a range), or surrounding
    /// whitespace.
    pub fn parse(spec: &str) -> Result<LibcFloor> {
        let invalid = || {
            Error::Invalid(format!(
                "libc floor {spec:?} is not supported: write \"major.minor\" or \
                 \">=major.minor\" (a patch level is allowed, and only a minimum makes sense)"
            ))
        };
        if spec != spec.trim() {
            return Err(invalid());
        }
        let digits = spec.strip_prefix(">=").unwrap_or(spec);
        let mut parts = digits.split('.');
        let component = |part: Option<&str>| -> Result<u32> {
            part.filter(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
                .and_then(|part| part.parse().ok())
                .ok_or_else(invalid)
        };
        let major = component(parts.next())?;
        let minor = component(parts.next())?;
        let patch = match parts.next() {
            Some(part) => Some(component(Some(part))?),
            None => None,
        };
        if parts.next().is_some() {
            return Err(invalid());
        }
        Ok(LibcFloor {
            major,
            minor,
            patch,
        })
    }

    /// The canonical spelling, always with the `>=` and without a redundant `.0` patch.
    #[must_use]
    pub fn canonical(self) -> String {
        match self.patch {
            Some(patch) => format!(">={}.{}.{}", self.major, self.minor, patch),
            None => format!(">={}.{}", self.major, self.minor),
        }
    }

    /// True when a host reporting `host` satisfies this floor.
    #[must_use]
    pub fn is_met_by(self, host: LibcFloor) -> bool {
        host >= self
    }
}

impl fmt::Display for LibcFloor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.canonical())
    }
}

/// Equality is defined through the same key as [`Ord`] so `2.39` and `2.39.0` are one floor:
/// glibc names releases `major.minor`, and a host that reports the patch level must not look
/// like a different runtime.
impl PartialEq for LibcFloor {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other).is_eq()
    }
}

impl Eq for LibcFloor {}

impl Ord for LibcFloor {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (self.major, self.minor, self.patch.unwrap_or(0)).cmp(&(
            other.major,
            other.minor,
            other.patch.unwrap_or(0),
        ))
    }
}

impl PartialOrd for LibcFloor {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
