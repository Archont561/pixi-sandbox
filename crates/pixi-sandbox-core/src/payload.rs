//! Prebuilt binary payloads as conda packages (TASK-78, decision ADR-28).
//!
//! Everything here is a pure decision over inputs the caller has already read from disk: the
//! payload tree as [`PayloadEntry`] values, a name, a version, and a platform. The filesystem
//! walk, the `rattler-build` subprocess and the channel indexer live behind [`PayloadToolchain`],
//! so the rules are tested with a fake and never touch the network or this checkout.
//!
//! The pieces, in the order [`plan_wrap`] applies them:
//!
//! 1. [`validate_payload`] — names, safe relative paths, link containment, native executables
//!    against `noarch` and against the platform, and the entry point;
//! 2. [`budget_preflight`] — the blob and transport budgets, judged on uncompressed sizes;
//! 3. [`provenance`] — the upstream URL and the SHA256 of the fetched artifact;
//! 4. [`render_recipe`] — the `rattler-build` recipe that installs the payload under
//!    `libexec/<name>/` and, when asked, a launcher in `bin/` that execs the real file in place.
//!
//! Nothing in this module names a particular upstream. The worked example lives in documentation
//! and tests only (AC#9).

use crate::platform::Platform;
use crate::transport_budget::{BudgetViolation, TransportBudgets, bytes_to_mib};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// Directory name the recipe copies the payload from. The payload tree is staged under it.
pub const PAYLOAD_DIR: &str = "payload";
/// File name of the provenance document, both in the staged recipe and inside the package.
pub const PROVENANCE_FILE: &str = "pixi-sandbox-payload.json";
/// Schema of [`Provenance`]. Bump it with the document, never silently.
pub const PROVENANCE_SCHEMA: u32 = 1;

/// The three executable formats a payload can carry that tie it to one operating system.
///
/// Universal (fat) Mach-O files are not detected: they share their magic with Java class files,
/// and a false positive would refuse a legitimate `noarch` package.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeFormat {
    Elf,
    MachO,
    Pe,
}

impl NativeFormat {
    /// The name an operator reads in a refusal.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            NativeFormat::Elf => "ELF",
            NativeFormat::MachO => "Mach-O",
            NativeFormat::Pe => "PE",
        }
    }

    /// Whether a file of this format can run on the platform's operating system. The
    /// architecture is not checked yet (ADR-28, open items).
    const fn runs_on(self, platform: Platform) -> bool {
        match self {
            NativeFormat::Elf => matches!(platform, Platform::Linux64 | Platform::LinuxAarch64),
            NativeFormat::MachO => matches!(platform, Platform::Osx64 | Platform::OsxArm64),
            NativeFormat::Pe => matches!(platform, Platform::Win64),
        }
    }
}

impl std::fmt::Display for NativeFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Recognise a native executable from its leading bytes.
///
/// `head` is at most the first four bytes of a file. Shorter input never matches.
#[must_use]
pub fn detect_native(head: &[u8]) -> Option<NativeFormat> {
    const MACHO_MAGICS: [&[u8]; 4] = [
        &[0xCF, 0xFA, 0xED, 0xFE],
        &[0xFE, 0xED, 0xFA, 0xCF],
        &[0xCE, 0xFA, 0xED, 0xFE],
        &[0xFE, 0xED, 0xFA, 0xCE],
    ];

    if head.starts_with(&b"\x7fELF"[..]) {
        Some(NativeFormat::Elf)
    } else if head.starts_with(&b"MZ"[..]) {
        Some(NativeFormat::Pe)
    } else if MACHO_MAGICS.iter().any(|magic| head.starts_with(magic)) {
        Some(NativeFormat::MachO)
    } else {
        None
    }
}

/// One node of the payload tree, as the caller found it on disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PayloadEntry {
    /// A regular file. `head` carries its first bytes (enough for [`detect_native`]).
    File {
        path: String,
        size: u64,
        executable: bool,
        head: Vec<u8>,
    },
    /// A directory, kept so that empty directories survive the round trip.
    Directory { path: String },
    /// A symbolic link. `target` is stored verbatim, relative or dangling.
    Symlink { path: String, target: String },
}

impl PayloadEntry {
    fn path(&self) -> &str {
        match self {
            PayloadEntry::File { path, .. }
            | PayloadEntry::Directory { path }
            | PayloadEntry::Symlink { path, .. } => path,
        }
    }
}

/// What the caller asks to be packaged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadSpec {
    /// The conda package name.
    pub name: String,
    /// The conda package version.
    pub version: String,
    /// The one platform subdir the package is built for.
    pub platform: Platform,
    /// Declared `noarch`. Refused for any payload that contains a native executable.
    pub noarch: bool,
    /// An optional payload-relative executable to expose on `PATH` as a launcher.
    pub entry_point: Option<String>,
}

/// A payload that passed [`validate_payload`], with the totals the budgets judge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedPayload {
    pub file_count: u64,
    pub directory_count: u64,
    pub symlink_count: u64,
    /// Sum of regular-file sizes. Links and directories contribute nothing.
    pub total_bytes: u64,
}

/// The one-line remedy for a blob budget violation (AC#6).
const BLOB_REMEDY: &str = "the wrapped package is a single file that the channel must accept and that cannot be split; remove files the consumer does not need (debug symbols, unused locales, test data), and confirm the channel's upload limit before raising budgets.max_blob_mib";
/// The one-line remedy for a transport budget violation (AC#6).
const TRANSPORT_REMEDY: &str = "the transport stores the package and the unpacked prefix files, so the payload is counted twice; remove files the consumer does not need, or raise budgets.max_transport_mib only with a reviewed reason";

/// Check a payload tree against everything that must hold before a build is attempted.
///
/// # Errors
///
/// Returns the first rule the request breaks, in this order: the package name and version, each
/// path, path conflicts, link containment, native executables against `noarch` and the platform,
/// then the entry point.
pub fn validate_payload(
    spec: &PayloadSpec,
    entries: &[PayloadEntry],
) -> Result<ValidatedPayload, PayloadError> {
    validate_spec_identity(spec)?;

    let index = index_entries(entries)?;
    for entry in entries {
        if let PayloadEntry::Symlink { path, target } = entry
            && !link_stays_inside(path, target)
        {
            return Err(PayloadError::EscapingSymlink {
                link: path.clone(),
                target: target.clone(),
            });
        }
    }

    let mut totals = ValidatedPayload {
        file_count: 0,
        directory_count: 0,
        symlink_count: 0,
        total_bytes: 0,
    };
    for entry in entries {
        match entry {
            PayloadEntry::Directory { .. } => totals.directory_count += 1,
            PayloadEntry::Symlink { .. } => totals.symlink_count += 1,
            PayloadEntry::File {
                path, size, head, ..
            } => {
                totals.file_count += 1;
                totals.total_bytes = totals.total_bytes.saturating_add(*size);
                if let Some(format) = detect_native(head) {
                    check_native(spec, path, format)?;
                }
            }
        }
    }

    if let Some(entry_point) = &spec.entry_point {
        validate_entry_point(spec, entry_point, &index)?;
    }
    Ok(totals)
}

fn validate_name(name: &str) -> Result<(), PayloadError> {
    let mut chars = name.chars();
    let starts_well = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let rest_well =
        chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '_' | '-'));
    if starts_well && rest_well {
        Ok(())
    } else {
        Err(PayloadError::InvalidName(name.to_string()))
    }
}

/// Validate the identity fields a spec carries (package name and version) without a payload tree.
///
/// A config declares these before any bytes exist, so a typo is refused at load time rather than
/// after a multi-hundred-megabyte fetch (AC#13's early gate on the config side).
///
/// # Errors
///
/// [`PayloadError::InvalidName`] or [`PayloadError::InvalidVersion`].
pub fn validate_spec_identity(spec: &PayloadSpec) -> Result<(), PayloadError> {
    validate_name(&spec.name)?;
    validate_version(&spec.version)
}

fn validate_version(version: &str) -> Result<(), PayloadError> {
    // conda uses `-` to separate version from build, so a version may not contain one.
    let mut chars = version.chars();
    let starts_well = chars.next().is_some_and(|c| c.is_ascii_alphanumeric());
    let rest_well = chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+'));
    if starts_well && rest_well {
        Ok(())
    } else {
        Err(PayloadError::InvalidVersion(version.to_string()))
    }
}

/// Index every entry by path, refusing unsafe paths and every shape the install cannot keep:
/// a duplicate, a non-directory where a directory is needed, and anything beneath a file or a
/// link. Directories may share a path only with themselves, which is a duplicate anyway.
fn index_entries(
    entries: &[PayloadEntry],
) -> Result<BTreeMap<String, &PayloadEntry>, PayloadError> {
    let mut index: BTreeMap<String, &PayloadEntry> = BTreeMap::new();
    for entry in entries {
        let path = entry.path();
        validate_relative_path(path)?;
        if index.contains_key(path) {
            return Err(PayloadError::ConflictingPath(path.to_string()));
        }
        for ancestor in ancestors(path) {
            if index
                .get(ancestor)
                .is_some_and(|existing| !matches!(existing, PayloadEntry::Directory { .. }))
            {
                return Err(PayloadError::ConflictingPath(path.to_string()));
            }
        }
        if !matches!(entry, PayloadEntry::Directory { .. }) && has_descendant(&index, path) {
            return Err(PayloadError::ConflictingPath(path.to_string()));
        }
        index.insert(path.to_string(), entry);
    }
    Ok(index)
}

/// Every proper ancestor of a relative path: `a/b/c` yields `a` and `a/b`.
fn ancestors(path: &str) -> impl Iterator<Item = &str> {
    path.match_indices('/').map(move |(end, _)| &path[..end])
}

fn has_descendant(index: &BTreeMap<String, &PayloadEntry>, path: &str) -> bool {
    let prefix = format!("{path}/");
    index
        .range(prefix.clone()..)
        .next()
        .is_some_and(|(key, _)| key.starts_with(&prefix))
}

fn validate_relative_path(path: &str) -> Result<(), PayloadError> {
    let unsafe_path = || PayloadError::UnsafePath(path.to_string());
    if path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.chars().any(char::is_control)
    {
        return Err(unsafe_path());
    }
    if path
        .split('/')
        .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return Err(unsafe_path());
    }
    Ok(())
}

/// A relative link stays inside the payload when, resolved lexically from its own directory,
/// it never climbs above the root. Absolute targets, empty targets and targets containing `\`
/// never do.
///
/// This is sound only because [`index_entries`] refuses any symlink used as a parent: then every
/// directory on the link's path is a real directory at that logical path, so the lexical walk
/// matches the layout the installed tree will have.
fn link_stays_inside(link: &str, target: &str) -> bool {
    if target.is_empty() || target.starts_with('/') || target.contains('\\') {
        return false;
    }
    // Depth of the link's own directory: its path components minus the link's own name.
    let mut depth = i64::try_from(link.split('/').count()).unwrap_or(i64::MAX) - 1;
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => depth += 1,
        }
    }
    true
}

fn check_native(spec: &PayloadSpec, path: &str, format: NativeFormat) -> Result<(), PayloadError> {
    if spec.noarch {
        return Err(PayloadError::NativeDeclaredNoarch {
            path: path.to_string(),
            format,
        });
    }
    if !format.runs_on(spec.platform) {
        return Err(PayloadError::PlatformMismatch {
            path: path.to_string(),
            format,
            platform: spec.platform,
        });
    }
    Ok(())
}

fn validate_entry_point(
    spec: &PayloadSpec,
    entry_point: &str,
    index: &BTreeMap<String, &PayloadEntry>,
) -> Result<(), PayloadError> {
    // The launcher writes this path into a shell script, so only characters that need no quoting
    // are allowed. Payload paths may contain anything else; the launcher may not.
    let quotable = !entry_point.is_empty()
        && !entry_point.starts_with('/')
        && entry_point
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'));
    if !quotable {
        return Err(PayloadError::EntryPointUnsafe(entry_point.to_string()));
    }
    if spec.platform == Platform::Win64 {
        return Err(PayloadError::LauncherUnsupported(spec.platform));
    }
    match index.get(entry_point) {
        Some(PayloadEntry::File {
            executable: true, ..
        }) => Ok(()),
        Some(PayloadEntry::File { .. }) => Err(PayloadError::EntryPointNotExecutable(
            entry_point.to_string(),
        )),
        _ => Err(PayloadError::EntryPointMissing(entry_point.to_string())),
    }
}

/// Judge the payload against the blob and transport budgets.
///
/// Both figures are conservative. The blob is compared with the uncompressed total, an upper
/// bound on the compressed `.conda`. The transport is compared with twice that total, because
/// the transport stores the package and the unpacked prefix files, and `record_file` keeps a
/// blob per path. The exact built sizes are checked after the build, in a later slice.
///
/// # Errors
///
/// Returns [`PayloadError::Budget`] listing every exceeded budget, blob first.
pub fn budget_preflight(
    payload: &ValidatedPayload,
    budgets: &TransportBudgets,
) -> Result<(), PayloadError> {
    let mut violations = Vec::new();
    push_over(
        &mut violations,
        "budgets.max_blob_mib",
        payload.total_bytes,
        budgets.max_blob_bytes,
        BLOB_REMEDY,
    );
    push_over(
        &mut violations,
        "budgets.max_transport_mib",
        payload.total_bytes.saturating_mul(2),
        budgets.max_transport_bytes,
        TRANSPORT_REMEDY,
    );
    if violations.is_empty() {
        Ok(())
    } else {
        Err(PayloadError::Budget(violations))
    }
}

fn push_over(
    violations: &mut Vec<BudgetViolation>,
    field: &'static str,
    measured_bytes: u64,
    limit_bytes: u64,
    remedy: &'static str,
) {
    if measured_bytes > limit_bytes {
        violations.push(BudgetViolation {
            field,
            measured_bytes,
            limit_bytes,
            remedy,
        });
    }
}

/// The package identity recorded in the provenance document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageIdentity {
    pub name: String,
    pub version: String,
    pub platform: String,
}

/// The external bytes that entered the channel: where they came from and what they hashed to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceArtifact {
    pub url: String,
    /// SHA256 of the artifact as fetched, lowercase hexadecimal.
    pub sha256: String,
}

/// Totals of what was packaged, so an auditor can compare them with the installed tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PayloadTotals {
    pub files: u64,
    pub bytes: u64,
}

/// The provenance document, written beside the recipe and shipped inside the package (AC#7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Provenance {
    pub schema: u32,
    pub package: PackageIdentity,
    pub source: SourceArtifact,
    pub payload: PayloadTotals,
}

/// Build the provenance record for a validated payload.
///
/// # Errors
///
/// Refuses a source that is not an `https://` URL without whitespace, and a digest that is not
/// 64 lowercase hexadecimal digits. An auditor needs both to verify the bytes independently.
pub fn provenance(
    spec: &PayloadSpec,
    payload: &ValidatedPayload,
    source_url: &str,
    source_sha256: &str,
) -> Result<Provenance, PayloadError> {
    validate_source(source_url, source_sha256)?;
    Ok(Provenance {
        schema: PROVENANCE_SCHEMA,
        package: PackageIdentity {
            name: spec.name.clone(),
            version: spec.version.clone(),
            platform: spec.platform.as_str().to_string(),
        },
        source: SourceArtifact {
            url: source_url.to_string(),
            sha256: source_sha256.to_string(),
        },
        payload: PayloadTotals {
            files: payload.file_count,
            bytes: payload.total_bytes,
        },
    })
}

/// Validate an upstream source declaration: an `https://` URL without whitespace, and a digest
/// that is 64 lowercase hexadecimal digits. Shared by [`provenance`] and the config loader, so a
/// hand-written config is refused before a fetch rather than after.
///
/// # Errors
///
/// [`PayloadError::InvalidSourceUrl`] or [`PayloadError::InvalidSha256`].
pub fn validate_source(source_url: &str, source_sha256: &str) -> Result<(), PayloadError> {
    let url_well = source_url.len() > "https://".len()
        && source_url.starts_with("https://")
        && !source_url
            .chars()
            .any(|c| c.is_whitespace() || c.is_control());
    if !url_well {
        return Err(PayloadError::InvalidSourceUrl(source_url.to_string()));
    }
    let digest_well = source_sha256.len() == 64
        && source_sha256
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if !digest_well {
        return Err(PayloadError::InvalidSha256(source_sha256.to_string()));
    }
    Ok(())
}

/// The provenance document as it is written: pretty JSON with a trailing newline.
///
/// # Panics
///
/// Never in practice: the document is plain data made of strings and integers, which always
/// serialise.
#[must_use]
pub fn render_provenance(provenance: &Provenance) -> String {
    let mut json = serde_json::to_string_pretty(provenance)
        .expect("provenance is plain data and always serialises");
    json.push('\n');
    json
}

/// Write the provenance document into `dir` under [`PROVENANCE_FILE`] and return its path.
///
/// # Errors
///
/// Returns [`PayloadError::Io`] when the file cannot be written.
pub fn write_provenance(dir: &Path, provenance: &Provenance) -> Result<PathBuf, PayloadError> {
    let path = dir.join(PROVENANCE_FILE);
    std::fs::write(&path, render_provenance(provenance)).map_err(|error| PayloadError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    Ok(path)
}

/// Compare the SHA256 of the artifact a `prepare` command produced against the digest the config
/// declared (AC#13).
///
/// This is the gate that turns "the upstream changed under us" from a green pipeline packing
/// different bytes into a named refusal. The digest is the fetched artifact's, not the tree's:
/// the config declares where the command left the downloaded bytes.
///
/// # Errors
///
/// [`PayloadError::Io`] when the artifact cannot be read (a `prepare` command that exited 0 but
/// wrote nothing lands here, naming the path), or [`PayloadError::ArtifactMismatch`] when the
/// bytes differ from the declared digest.
pub fn verify_artifact(path: &Path, expected_sha256: &str) -> Result<(), PayloadError> {
    let actual = crate::shard::sha256_file(path).map_err(|error| PayloadError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    if actual == expected_sha256 {
        Ok(())
    } else {
        Err(PayloadError::ArtifactMismatch {
            path: path.display().to_string(),
            expected: expected_sha256.to_string(),
            actual,
        })
    }
}

/// Read a payload tree from disk into [`PayloadEntry`] values, the form the pure rules judge.
///
/// `root` is a directory (walked recursively, children sorted for a stable recipe) or a single
/// regular file (one entry, named by its file name). Paths are relative to `root` with `/`
/// separators. Symlinks are recorded verbatim and never followed; the first four bytes of each
/// regular file travel along so [`detect_native`] can judge it without re-reading.
///
/// # Errors
///
/// [`PayloadError::Io`] when the tree cannot be walked, or [`PayloadError::NonUtf8Path`] for a
/// path the recipe could not name.
pub fn read_payload_tree(root: &Path) -> Result<Vec<PayloadEntry>, PayloadError> {
    let metadata = std::fs::symlink_metadata(root).map_err(|error| PayloadError::Io {
        path: root.display().to_string(),
        message: error.to_string(),
    })?;
    if !metadata.is_dir() {
        let name = root
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| PayloadError::NonUtf8Path(root.display().to_string()))?;
        return Ok(vec![file_entry(name, root, &metadata)?]);
    }

    let mut entries = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .sort_by_file_name();
    for item in walker {
        let item = item.map_err(|error| PayloadError::Io {
            path: error.path().map_or_else(
                || root.display().to_string(),
                |path| path.display().to_string(),
            ),
            message: error.to_string(),
        })?;
        let path = item.path();
        let relative = path.strip_prefix(root).map_err(|error| PayloadError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        let relative = relative
            .to_str()
            .ok_or_else(|| PayloadError::NonUtf8Path(path.display().to_string()))?
            .replace('\\', "/");
        let metadata = std::fs::symlink_metadata(path).map_err(|error| PayloadError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        entries.push(entry_for(&relative, path, &metadata)?);
    }
    Ok(entries)
}

fn entry_for(
    relative: &str,
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<PayloadEntry, PayloadError> {
    if metadata.file_type().is_symlink() {
        let target = std::fs::read_link(path).map_err(|error| PayloadError::Io {
            path: path.display().to_string(),
            message: error.to_string(),
        })?;
        let target = target
            .to_str()
            .ok_or_else(|| PayloadError::NonUtf8Path(path.display().to_string()))?;
        return Ok(PayloadEntry::Symlink {
            path: relative.to_string(),
            target: target.to_string(),
        });
    }
    if metadata.is_dir() {
        return Ok(PayloadEntry::Directory {
            path: relative.to_string(),
        });
    }
    file_entry(relative, path, metadata)
}

fn file_entry(
    relative: &str,
    path: &Path,
    metadata: &std::fs::Metadata,
) -> Result<PayloadEntry, PayloadError> {
    let executable = is_executable(metadata);
    let head = read_head(path)?;
    Ok(PayloadEntry::File {
        path: relative.to_string(),
        size: metadata.len(),
        executable,
        head,
    })
}

#[cfg(unix)]
fn is_executable(metadata: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(_metadata: &std::fs::Metadata) -> bool {
    false
}

fn read_head(path: &Path) -> Result<Vec<u8>, PayloadError> {
    use std::io::Read as _;
    let mut file = std::fs::File::open(path).map_err(|error| PayloadError::Io {
        path: path.display().to_string(),
        message: error.to_string(),
    })?;
    let mut head = [0u8; 4];
    let mut read = 0;
    while read < head.len() {
        let count = file
            .read(&mut head[read..])
            .map_err(|error| PayloadError::Io {
                path: path.display().to_string(),
                message: error.to_string(),
            })?;
        if count == 0 {
            break;
        }
        read += count;
    }
    Ok(head[..read].to_vec())
}

/// Render the `rattler-build` recipe for a validated payload.
///
/// The package installs the payload at `$PREFIX/libexec/<name>/`, keeping modes and links with
/// `cp -a`, and copies the provenance document beside it. With an entry point it also writes
/// `$PREFIX/bin/<basename>`, a POSIX launcher that `exec`s the real file in place, so runtime
/// lookups relative to the executable still resolve.
///
/// The recipe's `source: path:` and `$RECIPE_DIR` semantics were verified with `rattler-build`
/// 0.76.1 on 2026-10-12 (ADR-28). That build check does not replace the AC#3/#5 package install
/// round trip.
#[must_use]
pub fn render_recipe(spec: &PayloadSpec, _payload: &ValidatedPayload) -> String {
    let name = &spec.name;
    let install = format!("$PREFIX/libexec/{name}");
    let mut recipe = String::new();
    // `write!` into a String cannot fail, so the results are ignored.
    let _ = write!(
        recipe,
        "package:\n  name: \"{name}\"\n  version: \"{version}\"\n\n\
         source:\n  - path: {PAYLOAD_DIR}\n  - path: {PROVENANCE_FILE}\n\n\
         build:\n  number: 0\n  script: |\n    set -eu\n",
        version = spec.version,
    );
    if let Some(entry) = &spec.entry_point {
        let launcher = entry.rsplit('/').next().unwrap_or(entry);
        let _ = writeln!(recipe, "    mkdir -p \"{install}\" \"$PREFIX/bin\"");
        let _ = writeln!(
            recipe,
            "    cp -a \"$RECIPE_DIR/{PAYLOAD_DIR}/.\" \"{install}/\""
        );
        let _ = writeln!(
            recipe,
            "    cp \"$RECIPE_DIR/{PROVENANCE_FILE}\" \"{install}/\""
        );
        let _ = writeln!(
            recipe,
            "    cat > \"$PREFIX/bin/{launcher}\" <<'PIXI_SANDBOX_LAUNCHER'"
        );
        let _ = writeln!(recipe, "    #!/bin/sh");
        let _ = writeln!(
            recipe,
            "    exec \"$(dirname \"$0\")/../libexec/{name}/{entry}\" \"$@\""
        );
        let _ = writeln!(recipe, "    PIXI_SANDBOX_LAUNCHER");
        let _ = writeln!(recipe, "    chmod 755 \"$PREFIX/bin/{launcher}\"");
    } else {
        let _ = writeln!(recipe, "    mkdir -p \"{install}\"");
        let _ = writeln!(
            recipe,
            "    cp -a \"$RECIPE_DIR/{PAYLOAD_DIR}/.\" \"{install}/\""
        );
        let _ = writeln!(
            recipe,
            "    cp \"$RECIPE_DIR/{PROVENANCE_FILE}\" \"{install}/\""
        );
    }
    recipe
}

/// Everything the toolchain needs to build one package, decided before any I/O.
#[derive(Debug, Clone, PartialEq)]
pub struct WrapPlan {
    pub spec: PayloadSpec,
    pub recipe: String,
    pub provenance: Provenance,
    pub payload: ValidatedPayload,
}

/// One request to wrap a payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapRequest {
    pub spec: PayloadSpec,
    pub entries: Vec<PayloadEntry>,
    pub source_url: String,
    pub source_sha256: String,
    pub budgets: TransportBudgets,
}

/// Apply every pure rule and return the plan. Nothing is built.
///
/// # Errors
///
/// The first refusal from [`validate_payload`], [`budget_preflight`] or [`provenance`].
pub fn plan_wrap(request: &WrapRequest) -> Result<WrapPlan, PayloadError> {
    let payload = validate_payload(&request.spec, &request.entries)?;
    budget_preflight(&payload, &request.budgets)?;
    let provenance = provenance(
        &request.spec,
        &payload,
        &request.source_url,
        &request.source_sha256,
    )?;
    Ok(WrapPlan {
        recipe: render_recipe(&request.spec, &payload),
        spec: request.spec.clone(),
        provenance,
        payload,
    })
}

/// The effectful seam: `rattler-build` and the channel indexer sit behind it, so the rules above
/// are tested with a fake (ADR-9's shape, applied to the build tool).
///
/// The two directories are kept apart on purpose. `rattler-build` writes intermediate recipe
/// scratch and its own build log beside the package it produces, so `work_dir` is disposable;
/// `channel_dir` is the reviewable output that is published, and the indexer is the only thing
/// that writes there.
pub trait PayloadToolchain {
    /// Build one `.conda` for `plan` under `work_dir` and return its path.
    ///
    /// # Errors
    ///
    /// Any failure to produce the package.
    fn build(&self, plan: &WrapPlan, work_dir: &Path) -> Result<PathBuf, PayloadError>;

    /// Add `package` to the filesystem `channel_dir` and (re)generate its index.
    ///
    /// # Errors
    ///
    /// Any failure to index the channel.
    fn index(&self, channel_dir: &Path, package: &Path) -> Result<(), PayloadError>;
}

/// Plan the wrap, then build and index through the toolchain. A refused payload never reaches
/// the toolchain, so a refused package is never built or published (AC#2, AC#6).
///
/// # Errors
///
/// A refusal from [`plan_wrap`], or the toolchain's own failure. The channel is not indexed when
/// the build fails.
pub fn wrap(
    request: &WrapRequest,
    toolchain: &dyn PayloadToolchain,
    work_dir: &Path,
    channel_dir: &Path,
) -> Result<PathBuf, PayloadError> {
    let plan = plan_wrap(request)?;
    let package = toolchain.build(&plan, work_dir)?;
    toolchain.index(channel_dir, &package)?;
    Ok(package)
}

/// Every way a payload can be refused, each naming what the operator must change.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PayloadError {
    #[error(
        "package name `{0}` must start with a lowercase letter or digit and use only lowercase letters, digits, `.`, `_` or `-`"
    )]
    InvalidName(String),
    #[error(
        "version `{0}` must start with a letter or digit and use only letters, digits, `.`, `_` or `+` (conda reserves `-`)"
    )]
    InvalidVersion(String),
    #[error(
        "payload path `{0}` is not a safe relative path: it must be non-empty, not absolute, contain no `\\`, `.` or `..` components and no control characters"
    )]
    UnsafePath(String),
    #[error(
        "payload path `{0}` conflicts with another entry: a duplicate, a non-directory where a directory is needed, or a path beneath a file or link"
    )]
    ConflictingPath(String),
    #[error(
        "symlink `{link}` -> `{target}` leaves the payload; a link must resolve inside the payload root"
    )]
    EscapingSymlink { link: String, target: String },
    #[error(
        "`{path}` is a {format} executable, which is specific to one operating system, so a noarch package cannot carry it; declare the platform instead"
    )]
    NativeDeclaredNoarch { path: String, format: NativeFormat },
    #[error("`{path}` is a {format} executable and does not run on {platform}")]
    PlatformMismatch {
        path: String,
        format: NativeFormat,
        platform: Platform,
    },
    #[error("entry point `{0}` does not name a file in the payload")]
    EntryPointMissing(String),
    #[error("entry point `{0}` is not an executable file")]
    EntryPointNotExecutable(String),
    #[error(
        "entry point `{0}` may contain only letters, digits, `.`, `_`, `-` and `/`, and must be relative"
    )]
    EntryPointUnsafe(String),
    #[error("launchers are POSIX shell scripts, so {0} cannot expose an entry point yet")]
    LauncherUnsupported(Platform),
    #[error(
        "source `{0}` must be an https:// URL with no whitespace; provenance records only https sources"
    )]
    InvalidSourceUrl(String),
    #[error("source sha256 `{0}` must be exactly 64 lowercase hexadecimal digits")]
    InvalidSha256(String),
    #[error(
        "fetched artifact {path} has sha256 {actual}, but the config declared {expected}; the upstream changed — verify the new bytes and update source.sha256, or pin a version that still matches"
    )]
    ArtifactMismatch {
        path: String,
        expected: String,
        actual: String,
    },
    #[error(
        "payload path {0} is not valid UTF-8; payload paths must be UTF-8 so the recipe can name them"
    )]
    NonUtf8Path(String),
    #[error(
        "the config-declared `prepare` command `{command}` failed: {detail}; it must fetch the payload and leave it at the configured artifact/payload paths"
    )]
    PrepareFailed { command: String, detail: String },
    #[error("{0}")]
    InvalidConfig(String),
    #[error("payload platform `{0}` is not one of the five supported platform ids")]
    InvalidPlatform(String),
    #[error("{}", budget_message(.0))]
    Budget(Vec<BudgetViolation>),
    #[error("the build toolchain failed: {0}")]
    Toolchain(String),
    #[error("io error at {path}: {message}")]
    Io { path: String, message: String },
}

fn budget_message(violations: &[BudgetViolation]) -> String {
    violations
        .iter()
        .map(|v| {
            format!(
                "{} exceeded: measured {} bytes ({:.1} MiB), limit {} bytes ({:.1} MiB); {}",
                v.field,
                v.measured_bytes,
                bytes_to_mib(v.measured_bytes),
                v.limit_bytes,
                bytes_to_mib(v.limit_bytes),
                v.remedy
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}
