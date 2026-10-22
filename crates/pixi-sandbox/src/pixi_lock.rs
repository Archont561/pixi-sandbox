//! Read `pixi.lock` for the one preflight `pack` owes: whether each selected environment's
//! locked `PyPI` dependencies are transportable as wheels (issue #157).
//!
//! Native packing carries `PyPI` dependencies only as locked wheels: no solving, source builds
//! or dependency omission. This preflight names every offending environment/package before
//! output creation, rather than waiting for acquisition or installation (doc-2 §11).
//!
//! This diagnostic scan retains a small indentation reader for Pixi's environment selections
//! and top-level location/name map. The separate native writer performs authoritative v7
//! lock/record validation with the scoped YAML parser; this scan is not a general YAML parser.

use anyhow::{Context, Result, bail};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

/// One `PyPI` dependency native packing cannot carry because its locked location is not a wheel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NonWheelPypi {
    /// The environment that pulls the dependency in.
    pub environment: String,
    /// The package name from the lock's top-level `packages` block, or the location itself when
    /// the lock carries no matching entry.
    pub package: String,
    /// The location exactly as the environment's `packages` entry spells it.
    pub location: String,
}

/// Every non-wheel `PyPI` dependency under `environments.<env>.packages.<platform>`, for the
/// requested environments only, in lock order.
#[must_use]
pub fn non_wheel_pypi(lock: &str, envs: &[String], platform: &str) -> Vec<NonWheelPypi> {
    let scan = Scan::parse(lock);
    let mut findings = Vec::new();
    for environment in envs {
        let Some(by_platform) = scan.environments.get(environment) else {
            continue;
        };
        let Some(locations) = by_platform.get(platform) else {
            continue;
        };
        for location in locations {
            if is_wheel(location) {
                continue;
            }
            let package = scan
                .names
                .get(location)
                .cloned()
                .unwrap_or_else(|| location.clone());
            findings.push(NonWheelPypi {
                environment: environment.clone(),
                package,
                location: location.clone(),
            });
        }
    }
    findings
}

/// The two things the preflight reads out of a lock: the `PyPI` locations each environment lists
/// per platform, and the package name each location maps to.
#[derive(Debug, Default)]
struct Scan {
    /// environment → platform → locked `PyPI` locations.
    environments: BTreeMap<String, BTreeMap<String, Vec<String>>>,
    /// locked `PyPI` location → package name (from the top-level `packages` block).
    names: BTreeMap<String, String>,
}

/// Identify the wheel-shaped location for the diagnostic preflight: the filename ends in
/// `whl`. The native writer additionally parses the wheel filename. A URL fragment or query
/// is stripped first — `…#sha256=…` still names the wheel.
fn is_wheel(location: &str) -> bool {
    let without_fragment = location.split(['#', '?']).next().unwrap_or(location);
    let file_name = without_fragment
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(without_fragment);
    file_name.ends_with("whl")
}

/// The scalar after a `key:` — trimmed, with one layer of matching quotes removed.
fn scalar(rest: &str) -> String {
    let value = rest.trim();
    let unquoted = value
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .unwrap_or(value);
    unquoted.to_string()
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start_matches(' ').len()
}

impl Scan {
    fn parse(lock: &str) -> Self {
        let lines: Vec<&str> = lock.lines().collect();
        let mut scan = Self::default();
        let mut index = 0;
        while index < lines.len() {
            let line = lines[index];
            let trimmed = line.trim_end();
            if indent_of(line) == 0 {
                if trimmed == "environments:" {
                    index = scan.parse_environments(&lines, index + 1);
                    continue;
                }
                if trimmed == "packages:" {
                    index = scan.parse_packages(&lines, index + 1);
                    continue;
                }
            }
            index += 1;
        }
        scan
    }

    /// Read `environments:` until the next top-level key. Env names sit at indent 2, the
    /// `packages:` table at indent 4, each platform at indent 6, and its `- conda:` / `- pypi:`
    /// entries at indent 6 (the block sequence is aligned with its key).
    fn parse_environments(&mut self, lines: &[&str], start: usize) -> usize {
        let mut index = start;
        let mut environment: Option<String> = None;
        let mut platform: Option<String> = None;
        let mut in_packages = false;
        while index < lines.len() {
            let line = lines[index];
            let content = line.trim_start();
            if content.is_empty() || content.starts_with('#') {
                index += 1;
                continue;
            }
            let indent = indent_of(line);
            if indent == 0 {
                break;
            }
            match indent {
                2 => {
                    if let Some(name) = content.strip_suffix(':') {
                        self.environments.entry(name.to_string()).or_default();
                        environment = Some(name.to_string());
                        platform = None;
                        in_packages = false;
                    }
                }
                4 => {
                    in_packages = content == "packages:";
                    platform = None;
                }
                6 if in_packages => {
                    if let Some(name) = content.strip_suffix(':') {
                        platform = Some(name.to_string());
                    } else if let (Some(rest), Some(environment), Some(platform)) = (
                        content
                            .strip_prefix("- ")
                            .and_then(|entry| entry.strip_prefix("pypi:")),
                        environment.as_ref(),
                        platform.as_ref(),
                    ) {
                        self.environments
                            .get_mut(environment)
                            .expect("environment was just inserted")
                            .entry(platform.clone())
                            .or_default()
                            .push(scalar(rest));
                    }
                }
                _ => {}
            }
            index += 1;
        }
        index
    }

    /// Read the top-level `packages:` block. Each entry starts at indent 0 with `- `; a `pypi:`
    /// entry's following indent-2 `name:` gives the package name for that location.
    fn parse_packages(&mut self, lines: &[&str], start: usize) -> usize {
        let mut index = start;
        let mut location: Option<String> = None;
        while index < lines.len() {
            let line = lines[index];
            let content = line.trim_start();
            if content.is_empty() {
                index += 1;
                continue;
            }
            match indent_of(line) {
                0 => {
                    let Some(entry) = content.strip_prefix("- ") else {
                        break;
                    };
                    location = entry.strip_prefix("pypi:").map(scalar);
                }
                2 => {
                    if let (Some(current), Some(name)) =
                        (location.as_ref(), content.strip_prefix("name:"))
                    {
                        self.names.insert(current.clone(), scalar(name));
                        location = None;
                    }
                }
                _ => {}
            }
            index += 1;
        }
        index
    }
}

/// Refuse a `pack` whose selected environments lock a non-wheel `PyPI` dependency.
///
/// # Errors
///
/// Returns an error naming every offending dependency if the lock cannot be read or holds one,
/// so pack fails before any output directory exists.
pub fn refuse_non_wheel_pypi(lock_path: &Path, envs: &[String], platform: &str) -> Result<()> {
    let lock = std::fs::read_to_string(lock_path)
        .with_context(|| format!("reading {}", lock_path.display()))?;
    let findings = non_wheel_pypi(&lock, envs, platform);
    if findings.is_empty() {
        return Ok(());
    }
    let mut message = String::from(
        "native packing can carry a PyPI dependency only as a wheel, but the selected environments \
         lock dependencies that are not wheels:\n",
    );
    for finding in &findings {
        let _ = writeln!(
            message,
            "  - environment `{}` locks `{}` from `{}`",
            finding.environment, finding.package, finding.location
        );
    }
    message.push_str(
        "keep each such dependency out of published environments, or build it into a wheel",
    );
    bail!("{message}");
}
