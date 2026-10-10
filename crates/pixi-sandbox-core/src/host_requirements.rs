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

    /// One line naming every declared entry, in a fixed kind order — the spelling `pack` writes
    /// into a transport's branch README and `doctor` prints for a restored branch, so both
    /// surfaces describe the same set the same way. An empty set renders an empty string
    /// rather than a line of headings with nothing under them.
    #[must_use]
    pub fn summary(&self) -> String {
        let mut parts = Vec::new();
        if let Some(spec) = &self.libc {
            parts.push(format!("libc {spec}"));
        }
        if !self.packages.is_empty() {
            parts.push(format!("packages {}", self.packages.join(", ")));
        }
        if !self.services.is_empty() {
            parts.push(format!("services {}", self.services.join(", ")));
        }
        if !self.capabilities.is_empty() {
            let names = self
                .capabilities
                .iter()
                .map(|capability| capability.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("capabilities {names}"));
        }
        if !self.headless.is_empty() {
            parts.push(format!("headless {}", self.headless.join(", ")));
        }
        parts.join(" · ")
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

    /// The canonical spelling of a *floor*: always with the `>=` and without a redundant `.0`
    /// patch.
    #[must_use]
    pub fn canonical(self) -> String {
        format!(">={}", self.version())
    }

    /// The bare version, as a host reports it — `2.39`, not `>=2.39`. Compared with a floor, a
    /// host's own version is an observation, and printing an operator in front of it reads as a
    /// requirement the host never made.
    #[must_use]
    pub fn version(self) -> String {
        match self.patch {
            Some(patch) => format!("{}.{}.{}", self.major, self.minor, patch),
            None => format!("{}.{}", self.major, self.minor),
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

// ---------------------------------------------------------------------------------------------
// Probing: observations in, classification out
// ---------------------------------------------------------------------------------------------
//
// The split here is the whole reason this feature is testable without a host: a [`HostProbe`]
// reports what it can see — nothing else — and [`evaluate`] turns those observations into a
// report against the declared set. The probes themselves live in the CLI crate (they spawn
// `ldd`, `dpkg-query`, `systemctl`), while everything that decides what a *satisfied* or
// *missing* requirement means is pure data in this module, exercised by fixtures in a tempdir.

/// The package manager this host's distribution uses, for install guidance and for the
/// `unknown` a host without one earns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PackageManager {
    Apt,
    Dnf,
    Pacman,
    Unknown,
}

impl PackageManager {
    /// The spelling used in JSON output and in printed remedies.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            PackageManager::Apt => "apt",
            PackageManager::Dnf => "dnf",
            PackageManager::Pacman => "pacman",
            PackageManager::Unknown => "unknown",
        }
    }

    /// The one-line install command for a package name, or `None` when this host has no
    /// supported manager — which is the case that must degrade to manual guidance rather than
    /// to silence.
    #[must_use]
    pub fn install_command(self, package: &str) -> Option<String> {
        match self {
            PackageManager::Apt => Some(format!("sudo apt install {package}")),
            PackageManager::Dnf => Some(format!("sudo dnf install {package}")),
            PackageManager::Pacman => Some(format!("sudo pacman -S {package}")),
            PackageManager::Unknown => None,
        }
    }
}

impl fmt::Display for PackageManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What `doctor` observed about this host's distribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Distro {
    /// The `ID` from `/etc/os-release` — `ubuntu`, `fedora`, … — or `unknown`.
    pub id: String,
    pub manager: PackageManager,
}

impl Distro {
    /// What a host looks like when nothing could be read.
    #[must_use]
    pub fn unknown() -> Self {
        Distro {
            id: "unknown".to_string(),
            manager: PackageManager::Unknown,
        }
    }

    /// Parse `/etc/os-release` (or any file with its shape).
    ///
    /// `ID` decides, and for the distributions that only advertise a family through `ID_LIKE`
    /// (Linux Mint is `ID=linuxmint`, `ID_LIKE=ubuntu`) the second line is consulted rather
    /// than guessed at. Anything unrecognised is [`PackageManager::Unknown`], which later
    /// produces manual guidance instead of a wrong install command.
    #[must_use]
    pub fn parse_os_release(text: &str) -> Distro {
        let mut id = String::new();
        let mut id_like = String::new();
        for line in text.lines() {
            let line = line.trim();
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            // Values may be quoted; shell-escaped values (with backslashes) are out of scope
            // for the three fields this reads.
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key.trim() {
                "ID" => id = value.to_ascii_lowercase(),
                "ID_LIKE" => id_like = value.to_ascii_lowercase(),
                _ => {}
            }
        }
        let manager = classify_distribution(&id, &id_like);
        Distro {
            id: if id.is_empty() {
                "unknown".to_string()
            } else {
                id
            },
            manager,
        }
    }
}

/// The distribution families whose package names the remedies below are written for.
fn classify_distribution(id: &str, id_like: &str) -> PackageManager {
    let family = |candidates: &[&str]| {
        [id, id_like].iter().any(|field| {
            field
                .split_whitespace()
                .any(|word| candidates.contains(&word))
        })
    };
    if family(&[
        "debian",
        "ubuntu",
        "raspbian",
        "linuxmint",
        "pop",
        "elementary",
        "kali",
        "devuan",
    ]) {
        PackageManager::Apt
    } else if family(&[
        "fedora",
        "rhel",
        "centos",
        "rocky",
        "almalinux",
        "ol",
        "amzn",
        "oracle",
    ]) {
        PackageManager::Dnf
    } else if family(&["arch", "manjaro", "endeavouros", "garuda"]) {
        PackageManager::Pacman
    } else {
        PackageManager::Unknown
    }
}

/// What a probe could see about one requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub outcome: Outcome,
    /// What was observed, in the probe's words — printed verbatim in both reports.
    pub detail: String,
}

/// A probe's three answers. `Unknown` is not a failure: it means this host cannot be asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Satisfied,
    Missing,
    Unknown,
}

impl Observation {
    #[must_use]
    pub fn satisfied(detail: impl Into<String>) -> Self {
        Observation {
            outcome: Outcome::Satisfied,
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn missing(detail: impl Into<String>) -> Self {
        Observation {
            outcome: Outcome::Missing,
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn unknown(detail: impl Into<String>) -> Self {
        Observation {
            outcome: Outcome::Unknown,
            detail: detail.into(),
        }
    }

    #[must_use]
    pub fn is_satisfied(&self) -> bool {
        self.outcome == Outcome::Satisfied
    }
}

/// What a probe could see about the host's C runtime: the version when it could read one, and
/// always a sentence saying where that came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibcObservation {
    pub version: Option<LibcFloor>,
    pub detail: String,
}

/// Everything `evaluate` is allowed to know about the machine it is classifying for.
///
/// Read-only by construction: the trait has no method that changes anything, and no method
/// that reaches the network. A probe is expected to answer `Unknown`, never to guess — a wrong
/// `Satisfied` is the only outcome that can lose a user's afternoon.
pub trait HostProbe {
    /// This host's OS family, or `None` when it cannot be determined.
    fn host_family(&self) -> Option<HostFamily>;
    fn distro(&self) -> Distro;
    fn libc(&self) -> LibcObservation;
    /// Is this distro package installed?
    fn package(&self, name: &str) -> Observation;
    /// Is this service available to the invoking user?
    fn service(&self, name: &str) -> Observation;
    /// Is there a display this process can use (`DISPLAY`/`WAYLAND_DISPLAY`)? Nothing else —
    /// the headless fallback is [`evaluate`]'s to combine, because only it knows which
    /// providers were declared.
    fn display(&self) -> Observation;
    fn gpu(&self) -> Observation;
    /// Is this program on `PATH`? Never launched.
    fn program(&self, name: &str) -> Observation;
}

/// The kind of requirement a finding is about; the label both reports group by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequirementKind {
    Libc,
    Package,
    Service,
    Capability,
    Headless,
}

impl RequirementKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            RequirementKind::Libc => "libc",
            RequirementKind::Package => "package",
            RequirementKind::Service => "service",
            RequirementKind::Capability => "capability",
            RequirementKind::Headless => "headless",
        }
    }
}

/// How one declared requirement came out on this host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostStatus {
    Satisfied,
    Missing,
    Unknown,
    /// The transport declares requirements for a different OS family than this host's, so
    /// judging them here would be a guess. Never a failure.
    NotApplicable,
}

impl HostStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            HostStatus::Satisfied => "satisfied",
            HostStatus::Missing => "missing",
            HostStatus::Unknown => "unknown",
            HostStatus::NotApplicable => "not applicable",
        }
    }
}

impl fmt::Display for HostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One requirement, its outcome, and the remedy when there is one.
///
/// Not `Serialize` as a unit: the JSON report is built in `doctor` alongside its other
/// sections, so the field set lives in one place rather than in a derive plus a patch-up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFinding {
    pub kind: RequirementKind,
    /// The declared entry, as written: `>=2.34`, `fontconfig`, `dbus`, `display`, `xvfb-run`.
    pub name: String,
    pub status: HostStatus,
    /// What was observed (or why nothing could be).
    pub detail: String,
    /// The one-line thing to do about it, when something can be done on this host.
    pub remedy: Option<String>,
}

/// The full answer: what the transport declares, what this host is, and every finding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostReport {
    pub declared: HostRequirementSet,
    /// The family the transport was packed for, when this build recognises its platform.
    pub platform_family: Option<HostFamily>,
    pub host_family: Option<HostFamily>,
    pub distro: Distro,
    pub findings: Vec<HostFinding>,
}

impl HostReport {
    /// Findings that were found absent. The only status that enforcement fails on.
    #[must_use]
    pub fn missing(&self) -> usize {
        self.count(HostStatus::Missing)
    }

    #[must_use]
    pub fn unknown(&self) -> usize {
        self.count(HostStatus::Unknown)
    }

    #[must_use]
    pub fn satisfied(&self) -> usize {
        self.count(HostStatus::Satisfied)
    }

    fn count(&self, status: HostStatus) -> usize {
        self.findings
            .iter()
            .filter(|finding| finding.status == status)
            .count()
    }

    /// True when this host could be judged at all — the transport's family and the host's match.
    #[must_use]
    pub fn applicable(&self) -> bool {
        self.platform_family.is_some() && self.platform_family == self.host_family
    }

    /// The gate `--require-host-requirements` enforces: nothing is *missing*. `Unknown` never
    /// fails — a host without the queries to answer must not be treated as broken — and
    /// `NotApplicable` is not this host's business.
    #[must_use]
    pub fn ok(&self) -> bool {
        self.missing() == 0
    }
}

/// Classify every declared requirement against what `probe` can see.
///
/// When the transport's family and the host's do not match, nothing is probed at all: every
/// finding is `NotApplicable` with the reason, because a Linux libc floor says nothing about a
/// macOS host and a guessed verdict is worse than an explicit one.
#[must_use]
pub fn evaluate(
    declared: &HostRequirementSet,
    platform_family: Option<HostFamily>,
    probe: &impl HostProbe,
) -> HostReport {
    let host_family = probe.host_family();
    let distro = probe.distro();
    let mut findings = Vec::new();

    if platform_family.is_none() || platform_family != host_family {
        let detail = match (platform_family, host_family) {
            (None, _) => "the transport names a platform this build does not recognise".to_string(),
            (Some(declared), Some(host)) => {
                format!("declared for {declared} hosts; this host is {host}")
            }
            (Some(declared), None) => {
                format!("declared for {declared} hosts; this host's family is unknown")
            }
        };
        for (kind, name) in declared_requirements(declared) {
            findings.push(HostFinding {
                kind,
                name,
                status: HostStatus::NotApplicable,
                detail: detail.clone(),
                remedy: None,
            });
        }
        return HostReport {
            declared: declared.clone(),
            platform_family,
            host_family,
            distro,
            findings,
        };
    }

    let manager = distro.manager;
    if let Some(spec) = &declared.libc {
        findings.push(libc_finding(spec, probe));
    }

    for name in &declared.packages {
        let observation = probe.package(name);
        let status = status_of(observation.outcome);
        findings.push(HostFinding {
            kind: RequirementKind::Package,
            name: name.clone(),
            status,
            remedy: package_remedy(name, status, manager),
            detail: observation.detail,
        });
    }

    for name in &declared.services {
        let observation = probe.service(name);
        let status = status_of(observation.outcome);
        findings.push(HostFinding {
            kind: RequirementKind::Service,
            name: name.clone(),
            status,
            remedy: service_remedy(name, status),
            detail: observation.detail,
        });
    }

    // Headless providers are observed once and reused by the display capability below.
    let headless: Vec<(String, Observation)> = declared
        .headless
        .iter()
        .map(|name| (name.clone(), probe.program(name)))
        .collect();
    let available: Vec<&str> = headless
        .iter()
        .filter(|(_, observation)| observation.is_satisfied())
        .map(|(name, _)| name.as_str())
        .collect();

    findings.extend(capability_findings(declared, probe, &available, manager));

    for (name, observation) in headless {
        let status = status_of(observation.outcome);
        findings.push(HostFinding {
            kind: RequirementKind::Headless,
            name: name.clone(),
            status,
            remedy: package_remedy(&name, status, manager),
            detail: observation.detail,
        });
    }

    HostReport {
        declared: declared.clone(),
        platform_family,
        host_family,
        distro,
        findings,
    }
}

/// The declared libc floor as a finding: satisfied when the host reports a runtime that
/// meets it, missing when it reports one that does not, unknown when none can be read.
fn libc_finding(spec: &str, probe: &impl HostProbe) -> HostFinding {
    // A spec that reached here was validated at load; a parse failure would be a bug.
    let floor = LibcFloor::parse(spec).ok();
    let observed = probe.libc();
    let (status, detail) = match (floor, observed.version) {
        (Some(floor), Some(host)) if floor.is_met_by(host) => (
            HostStatus::Satisfied,
            format!("host reports {}; {}", host.version(), observed.detail),
        ),
        (Some(_), Some(host)) => (
            HostStatus::Missing,
            format!("host reports {}; {}", host.version(), observed.detail),
        ),
        _ => (
            HostStatus::Unknown,
            format!("no C runtime version could be read; {}", observed.detail),
        ),
    };
    HostFinding {
        kind: RequirementKind::Libc,
        name: spec.to_string(),
        status,
        remedy: match status {
            HostStatus::Missing => Some(format!(
                "the host C runtime cannot be installed per project — run this workload on a \
                 host with glibc {spec} (or newer), for example a newer base image"
            )),
            HostStatus::Unknown => {
                Some("check the host C runtime directly with `ldd --version`".to_string())
            }
            _ => None,
        },
        detail,
    }
}

/// Each declared capability as a finding. `Display` is satisfied by a live display or by
/// any available headless provider; `Gpu` is whatever the probe observed.
fn capability_findings(
    declared: &HostRequirementSet,
    probe: &impl HostProbe,
    available: &[&str],
    manager: PackageManager,
) -> Vec<HostFinding> {
    let mut findings = Vec::new();
    for capability in &declared.capabilities {
        let (status, detail) = match capability {
            HostCapability::Display => {
                let display = probe.display();
                match display.outcome {
                    Outcome::Satisfied => (HostStatus::Satisfied, display.detail),
                    _ if !available.is_empty() => (
                        HostStatus::Satisfied,
                        format!(
                            "{}; headless provider available: {}",
                            display.detail,
                            available.join(", ")
                        ),
                    ),
                    Outcome::Unknown => (HostStatus::Unknown, display.detail),
                    Outcome::Missing => (
                        HostStatus::Missing,
                        if declared.headless.is_empty() {
                            "no DISPLAY or WAYLAND_DISPLAY, and no headless provider is \
                             declared for this bundle"
                                .to_string()
                        } else {
                            format!(
                                "no DISPLAY or WAYLAND_DISPLAY, and none of the declared \
                                 headless providers is on PATH ({})",
                                declared.headless.join(", ")
                            )
                        },
                    ),
                }
            }
            HostCapability::Gpu => {
                let gpu = probe.gpu();
                (status_of(gpu.outcome), gpu.detail)
            }
        };
        findings.push(HostFinding {
            kind: RequirementKind::Capability,
            name: capability.as_str().to_string(),
            status,
            remedy: capability_remedy(*capability, status, manager),
            detail,
        });
    }
    findings
}

/// Every declared requirement as `(kind, spelling)`, in the order [`HostRequirementSet`]
/// documents — also the order [`evaluate`] reports in.
fn declared_requirements(declared: &HostRequirementSet) -> Vec<(RequirementKind, String)> {
    let mut out = Vec::new();
    if let Some(spec) = &declared.libc {
        out.push((RequirementKind::Libc, spec.clone()));
    }
    out.extend(
        declared
            .packages
            .iter()
            .map(|name| (RequirementKind::Package, name.clone())),
    );
    out.extend(
        declared
            .services
            .iter()
            .map(|name| (RequirementKind::Service, name.clone())),
    );
    out.extend(
        declared
            .capabilities
            .iter()
            .map(|capability| (RequirementKind::Capability, capability.as_str().to_string())),
    );
    out.extend(
        declared
            .headless
            .iter()
            .map(|name| (RequirementKind::Headless, name.clone())),
    );
    out
}

fn status_of(outcome: Outcome) -> HostStatus {
    match outcome {
        Outcome::Satisfied => HostStatus::Satisfied,
        Outcome::Missing => HostStatus::Missing,
        Outcome::Unknown => HostStatus::Unknown,
    }
}

/// The per-distribution package name for the names worth spelling out. Everything else falls
/// back to the declared name, with the caveat that names differ between distributions — a
/// wrong-but-confident `apt install` line is worse than an honest one.
fn curated_package(name: &str, manager: PackageManager) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    Some(match (lower.as_str(), manager) {
        ("fontconfig", PackageManager::Apt | PackageManager::Dnf | PackageManager::Pacman) => {
            "fontconfig"
        }
        (
            "fonts" | "fonts-dejavu" | "fonts-dejavu-core" | "dejavu-sans-fonts" | "ttf-dejavu",
            PackageManager::Apt,
        ) => "fonts-dejavu-core",
        (
            "fonts" | "fonts-dejavu" | "fonts-dejavu-core" | "dejavu-sans-fonts" | "ttf-dejavu",
            PackageManager::Dnf,
        ) => "dejavu-sans-fonts",
        (
            "fonts" | "fonts-dejavu" | "fonts-dejavu-core" | "dejavu-sans-fonts" | "ttf-dejavu",
            PackageManager::Pacman,
        ) => "ttf-dejavu",
        ("xvfb" | "xvfb-run" | "xorg-x11-server-xvfb", PackageManager::Apt) => "xvfb",
        ("xvfb" | "xvfb-run" | "xorg-x11-server-xvfb", PackageManager::Dnf) => {
            "xorg-x11-server-Xvfb"
        }
        ("xvfb" | "xvfb-run" | "xorg-x11-server-xvfb", PackageManager::Pacman) => {
            "xorg-server-xvfb"
        }
        ("dbus" | "dbus-daemon" | "dbus-broker", PackageManager::Apt | PackageManager::Pacman) => {
            "dbus"
        }
        ("dbus" | "dbus-daemon" | "dbus-broker", PackageManager::Dnf) => "dbus-daemon",
        ("libgtk-3" | "gtk3" | "libgtk-3-0" | "gtk3-devel", PackageManager::Apt) => "libgtk-3-0",
        (
            "libgtk-3" | "gtk3" | "libgtk-3-0" | "gtk3-devel",
            PackageManager::Dnf | PackageManager::Pacman,
        ) => "gtk3",
        (
            "webkit2gtk" | "libwebkit2gtk" | "libwebkit2gtk-4.1-0" | "webkit2gtk4.1",
            PackageManager::Apt,
        ) => "libwebkit2gtk-4.1-0",
        (
            "webkit2gtk" | "libwebkit2gtk" | "libwebkit2gtk-4.1-0" | "webkit2gtk4.1",
            PackageManager::Dnf,
        ) => "webkit2gtk4.1",
        (
            "webkit2gtk" | "libwebkit2gtk" | "libwebkit2gtk-4.1-0" | "webkit2gtk4.1",
            PackageManager::Pacman,
        ) => "webkit2gtk-4.1",
        _ => return None,
    })
}

/// The remedy for an unmet (or unanswerable) host package: the command for *this* host's
/// manager, or the manual guidance when there is no supported manager at all.
fn package_remedy(name: &str, status: HostStatus, manager: PackageManager) -> Option<String> {
    if status == HostStatus::Satisfied {
        return None;
    }
    if status == HostStatus::Unknown {
        return Some(format!(
            "no supported package-manager query on this host; verify that {name} is installed \
             and rerun"
        ));
    }
    let curated = curated_package(name, manager);
    let (package, caveat) = match curated {
        Some(package) => (package.to_string(), ""),
        None => (
            name.to_string(),
            " (package names differ between distributions — search your package manager if this \
             name is not found)",
        ),
    };
    match manager.install_command(&package) {
        Some(command) => Some(format!("install it: {command}{caveat}")),
        None => Some(format!(
            "install {package} with this host's package manager, or add it to the container \
             image{caveat}"
        )),
    }
}

fn service_remedy(name: &str, status: HostStatus) -> Option<String> {
    if status == HostStatus::Satisfied {
        return None;
    }
    let dbus_hint = if name.eq_ignore_ascii_case("dbus") {
        "; desktop applications need a session bus — start one with `dbus-run-session <command>` \
         when the host has no systemd user session"
    } else {
        ""
    };
    Some(match status {
        HostStatus::Missing => format!(
            "start it: `sudo systemctl enable --now {name}` (or the equivalent for this host){dbus_hint}"
        ),
        _ => format!(
            "no service manager on this host to query; verify that {name} is running{dbus_hint}"
        ),
    })
}

fn capability_remedy(
    capability: HostCapability,
    status: HostStatus,
    manager: PackageManager,
) -> Option<String> {
    if matches!(status, HostStatus::Satisfied | HostStatus::NotApplicable) {
        return None;
    }
    Some(match capability {
        HostCapability::Display => match status {
            HostStatus::Missing => {
                let install = curated_package("xvfb", manager)
                    .and_then(|package| manager.install_command(package))
                    .unwrap_or_else(|| {
                        "install a virtual framebuffer (Xvfb) with this host's package manager"
                            .to_string()
                    });
                format!(
                    "run the workload against a display (DISPLAY or WAYLAND_DISPLAY), or use a \
                     headless provider: {install}, then run it under `xvfb-run`"
                )
            }
            _ => "display availability could not be determined; check DISPLAY/WAYLAND_DISPLAY \
                  and any declared headless provider by hand"
                .to_string(),
        },
        HostCapability::Gpu => match status {
            HostStatus::Missing => "GPU access cannot be provided from inside the environment: \
                                    grant the workload device access on the host (for containers, \
                                    pass through /dev/dri and the driver mounts, and run as a user \
                                    in the `video`/`render` groups)"
                .to_string(),
            _ => "GPU availability could not be determined on this host; check device nodes \
                  (/dev/dri) and driver mounts by hand"
                .to_string(),
        },
    })
}
