//! Verification: everything a person needs before trusting a branch, and nothing that writes.
//!
//! Three jobs live here:
//!
//! 1. **Check every declared byte.** [`verify`] walks a manifest and reports *all* failures
//!    instead of stopping at the first — an airlock operator wants the full list, not a game
//!    of whack-a-mole.
//! 2. **Check the tree that comes out the other end.** [`verify_restored`] compares a
//!    restored project against the manifest's per-file oracle (D13) — the branch being intact
//!    and each written blob matching its sha does not by itself prove the *prefix* is right,
//!    and a forged-but-plausible prefix used to pass every check.
//! 3. **Refuse to ship a dynamic tool** (decision D4). The `~/.pixi/bin` shims are 766 KiB
//!    trampolines that exec a dynamically linked binary inside their own prefix; a transport
//!    that carries one works perfectly on the machine that built it and fails on the airlock.
//!    [`linkage_of`] is what catches that.

use crate::manifest::{Blob, MANIFEST_DIR, MANIFEST_FILE, Manifest};
use crate::shard;
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

/// Where the manifest of a transport / extracted branch lives.
#[must_use]
pub fn manifest_path(branch_location: &Path) -> PathBuf {
    branch_location.join(MANIFEST_DIR).join(MANIFEST_FILE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Missing,
    Integrity,
    SizeMismatch,
    MissingPart,
    DynamicTool,
    /// Only used by [`verify_restored`]: a file that is on disk but not in the manifest's
    /// file list, and is not one of the bookkeeping files pixi/restore own.
    Unexpected,
    /// Only used by [`verify_restored`]: the content matches but the executable bit does not.
    Mode,
}

impl Kind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Missing => "missing",
            Kind::Integrity => "integrity",
            Kind::SizeMismatch => "size",
            Kind::MissingPart => "missing-part",
            Kind::DynamicTool => "dynamic-tool",
            Kind::Unexpected => "unexpected",
            Kind::Mode => "mode",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Check {
    pub path: String,
    pub kind: Kind,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct Report {
    /// Logical blobs checked (a split blob counts once).
    pub files: u64,
    /// Sum of the declared (whole-file) sizes, i.e. the payload actually shipped.
    pub bytes: u64,
    pub failures: Vec<Check>,
}

impl Report {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }

    /// The verified byte count in MiB, for human-readable reports. The u64->f64 cast
    /// can lose the low bits of huge counts; that is inherent to a display conversion.
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn mebibytes(&self) -> f64 {
        self.bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Check every blob of the selected environments (all of them when `envs` is `None`),
/// the per-env file lists that act as the restored-tree oracle (D13), plus the embedded
/// tools' integrity and linkage.
///
/// Never writes anything, never needs the network.
#[must_use]
pub fn verify(manifest: &Manifest, branch_location: &Path, envs: Option<&[String]>) -> Report {
    let mut report = Report::default();

    for (env, blob) in manifest.blobs(envs) {
        let abs = manifest.blob_abs_path(branch_location, blob);
        report.files += 1;
        report.bytes += blob.size;
        check_blob(env, blob, &abs, &mut report);
    }

    // The oracle is only as good as its own bytes: an env's files manifest is a blob like
    // any other and is verified here, so `doctor --verify-restored` can trust what it reads.
    for (name, env) in selected_envs(manifest, envs) {
        if let Some(files) = &env.files {
            report.files += 1;
            report.bytes += files.blob.size;
            let abs = manifest.blob_abs_path(branch_location, &files.blob);
            check_blob(name, &files.blob, &abs, &mut report);
        }
    }

    for (name, tool) in &manifest.tools {
        let Some(rel) = &tool.path else { continue };
        let abs = branch_location.join(MANIFEST_DIR).join(rel);
        if !abs.exists() {
            report.failures.push(Check {
                path: rel.clone(),
                kind: Kind::Missing,
                detail: format!("tool {name} is declared but not present"),
            });
            continue;
        }
        report.files += 1;
        report.bytes += tool.size_bytes;

        // Size is always checkable; the sha256 is checked against the recorded pin.
        match std::fs::metadata(&abs) {
            Ok(meta) if meta.len() != tool.size_bytes => report.failures.push(Check {
                path: rel.clone(),
                kind: Kind::SizeMismatch,
                detail: format!(
                    "tool {name}: expected {} bytes, found {}",
                    tool.size_bytes,
                    meta.len()
                ),
            }),
            Err(e) => report.failures.push(Check {
                path: rel.clone(),
                kind: Kind::Missing,
                detail: format!("tool {name}: {e}"),
            }),
            _ => {}
        }

        if let Some(pin) = &tool.pinned_sha256 {
            match shard::sha256_file(&abs) {
                Ok(actual) if &actual != pin => report.failures.push(Check {
                    path: rel.clone(),
                    kind: Kind::Integrity,
                    detail: format!(
                        "tool {name} does not match its pin: expected {pin}, got {actual}"
                    ),
                }),
                Err(e) => report.failures.push(Check {
                    path: rel.clone(),
                    kind: Kind::Missing,
                    detail: format!("tool {name}: {e}"),
                }),
                _ => {}
            }
        }

        if tool.linkage != "script" && linkage_of(&abs) == Linkage::Dynamic {
            report.failures.push(Check {
                path: rel.clone(),
                kind: Kind::DynamicTool,
                detail: format!(
                    "tool {name} is dynamically linked; it will not run on an airlock \
                     (ship the static release asset — see decisions D4)"
                ),
            });
        }
    }

    report
}

/// The envs a caller selected, or all of them (the same selection rule `Manifest::blobs`
/// applies, kept in one place for the file lists, which are not part of `blobs`).
fn selected_envs<'a>(
    manifest: &'a Manifest,
    envs: Option<&[String]>,
) -> impl Iterator<Item = (&'a String, &'a crate::manifest::Env)> {
    manifest.envs.iter().filter(move |(name, _)| {
        envs.is_none_or(|selected| selected.iter().any(|s| s == name.as_str()))
    })
}

/// The result of checking a *restored project* against the manifest's per-file oracle.
#[derive(Debug, Clone, Default)]
pub struct RestoredReport {
    /// What was checked and what failed, in the same shape as a transport verification so
    /// `doctor` prints both the same way.
    pub report: Report,
    /// Environments whose per-file digests were compared.
    pub verified: Vec<String>,
    /// Environments the manifest carries no oracle for (schema 1): reported, not failed —
    /// an old branch cannot be retrofitted, and pretending otherwise would be the exact
    /// "shape check dressed up as integrity" this exists to end.
    pub unverifiable: Vec<String>,
}

impl RestoredReport {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.report.ok()
    }
}

/// Verify the tree a restore produced against the manifest's per-file digests (D13).
///
/// `project` is the restored project root (holding `.pixi/envs/<name>`), `branch_location`
/// the transport the manifest — and therefore the oracle — came from. `work_dir` only needs
/// naming when the restore used a non-default one: the canonicalisation must neutralise the
/// same restore-scratch paths the restore itself embedded.
///
/// Collects every mismatch instead of stopping at the first, and writes nothing.
#[must_use]
pub fn verify_restored(
    manifest: &Manifest,
    branch_location: &Path,
    project: &Path,
    envs: Option<&[String]>,
    work_dir: Option<&Path>,
) -> RestoredReport {
    let mut restored = RestoredReport::default();

    for (name, env) in selected_envs(manifest, envs) {
        let Some(files) = &env.files else {
            restored.unverifiable.push(name.clone());
            continue;
        };
        let failures_before = restored.report.failures.len();
        verify_env_restored(
            branch_location,
            project,
            name,
            env,
            files,
            work_dir,
            &mut restored,
        );
        if restored.report.failures.len() == failures_before {
            restored.verified.push(name.clone());
        }
    }
    restored
}

fn verify_env_restored(
    branch_location: &Path,
    project: &Path,
    name: &str,
    env: &crate::manifest::Env,
    files: &crate::manifest::EnvFiles,
    work_dir: Option<&Path>,
    restored: &mut RestoredReport,
) {
    let report = &mut restored.report;
    let prefix = project.join(".pixi").join("envs").join(name);

    // The oracle itself: read, verify against its recorded digest, parse.
    let list_bytes = match shard::read_blob(&branch_location.join(MANIFEST_DIR), &files.blob) {
        Ok(bytes) => bytes,
        Err(e) => {
            report.failures.push(Check {
                path: files.blob.path.clone(),
                kind: Kind::Integrity,
                detail: format!("env {name}: the file list cannot be read or trusted: {e}"),
            });
            return;
        }
    };
    let doc = match crate::files_manifest::FilesDoc::parse(&list_bytes) {
        Ok(doc) => doc,
        Err(e) => {
            report.failures.push(Check {
                path: files.blob.path.clone(),
                kind: Kind::Integrity,
                detail: format!("env {name}: {e}"),
            });
            return;
        }
    };
    if files.entries != doc.entries() as u64 {
        report.failures.push(Check {
            path: files.blob.path.clone(),
            kind: Kind::SizeMismatch,
            detail: format!(
                "env {name}: manifest declares {} file entries, the list holds {}",
                files.entries,
                doc.entries()
            ),
        });
    }

    let walked = match crate::files_manifest::walk_prefix(&prefix) {
        Ok(walked) => walked,
        Err(e) => {
            report.failures.push(Check {
                path: format!(".pixi/envs/{name}"),
                kind: Kind::Missing,
                detail: format!("env {name}: the restored prefix cannot be walked: {e}"),
            });
            return;
        }
    };

    // The prefix-path spellings this side must neutralise: the final prefix (in both its
    // literal and canonical form — a symlinked `.pixi` must not defeat the check), plus the
    // restore-scratch paths that survive inside NUL-fixed binaries and conda-meta records.
    let mut candidates: Vec<Vec<u8>> = Vec::new();
    let mut push_candidate = |path: &PathBuf| {
        let bytes = path.to_string_lossy().into_owned().into_bytes();
        if !bytes.is_empty() && !candidates.contains(&bytes) {
            candidates.push(bytes);
        }
    };
    push_candidate(&prefix);
    if let Ok(canonical) = prefix.canonicalize() {
        push_candidate(&canonical);
    }
    let work = work_dir.map_or_else(
        || project.join(".pixi").join(".restore-work"),
        std::path::Path::to_path_buf,
    );
    push_candidate(&work.join(format!("stage-{name}")).join(name));
    push_candidate(&work.join(format!("pack-{name}")));

    let listed: std::collections::BTreeMap<&str, &crate::files_manifest::FileEntry> = doc
        .files
        .iter()
        .map(|entry| (entry.p.as_str(), entry))
        .collect();

    // Every listed entry must exist, with the recorded content, mode and symlink target.
    for entry in &doc.files {
        report.files += 1;
        if let Some(path) = walked.files.get(&entry.p) {
            report.bytes += std::fs::metadata(path).map_or(0, |m| m.len());
            if let Some(expected) = &entry.h {
                let raw = match std::fs::read(path) {
                    Ok(raw) => raw,
                    Err(e) => {
                        report.failures.push(Check {
                            path: entry.p.clone(),
                            kind: Kind::Missing,
                            detail: format!("env {name}: {e}"),
                        });
                        continue;
                    }
                };
                let actual = crate::files_manifest::canonical_sha256(&raw, &candidates);
                if &actual != expected {
                    report.failures.push(Check {
                        path: entry.p.clone(),
                        kind: Kind::Integrity,
                        detail: format!(
                            "env {name}: content does not match the manifest's file list \
                             (expected {expected}, got {actual})"
                        ),
                    });
                }
            }
            match crate::files_manifest::is_executable(path) {
                Ok(actual) if actual != entry.x => {
                    report.failures.push(Check {
                        path: entry.p.clone(),
                        kind: Kind::Mode,
                        detail: format!(
                            "env {name}: executable bit is {}, the manifest records {}",
                            if entry.x { "clear" } else { "set" },
                            if entry.x { "set" } else { "clear" }
                        ),
                    });
                }
                Err(e) => report.failures.push(Check {
                    path: entry.p.clone(),
                    kind: Kind::Missing,
                    detail: format!("env {name}: {e}"),
                }),
                _ => {}
            }
        } else if let Some(path) = walked.symlinks.get(&entry.p) {
            let actual = match std::fs::read_link(path) {
                Ok(target) => String::from_utf8_lossy(&crate::files_manifest::canonicalise(
                    target.to_string_lossy().as_bytes(),
                    &candidates,
                ))
                .into_owned(),
                Err(e) => {
                    report.failures.push(Check {
                        path: entry.p.clone(),
                        kind: Kind::Missing,
                        detail: format!("env {name}: {e}"),
                    });
                    continue;
                }
            };
            let expected = entry.l.clone().unwrap_or_default();
            if actual != expected {
                report.failures.push(Check {
                    path: entry.p.clone(),
                    kind: Kind::Integrity,
                    detail: format!(
                        "env {name}: symlink points at {actual:?}, the manifest records {expected:?}"
                    ),
                });
            }
        } else {
            report.failures.push(Check {
                path: entry.p.clone(),
                kind: Kind::Missing,
                detail: format!(
                    "env {name}: in the manifest's file list but not in the restored prefix"
                ),
            });
        }
    }

    // Nothing unlisted, except the bookkeeping files pixi and restore own (the allowlist is
    // exactly conda-meta markers — see files_manifest::ALLOWED_EXTRAS).
    for rel in walked.files.keys().chain(walked.symlinks.keys()) {
        if !listed.contains_key(rel.as_str())
            && !crate::files_manifest::ALLOWED_EXTRAS.contains(&rel.as_str())
        {
            report.failures.push(Check {
                path: rel.clone(),
                kind: Kind::Unexpected,
                detail: format!(
                    "env {name}: present in the restored prefix but not in the manifest's \
                     file list — a file the transport never carried"
                ),
            });
        }
    }

    // The fingerprint marker restore writes must still say what the manifest recorded.
    if let Some(expected) = &env.pixi_environment_fingerprint {
        let marker = prefix
            .join("conda-meta")
            .join(".pixi-environment-fingerprint");
        match std::fs::read_to_string(&marker) {
            Ok(actual) => {
                if actual.trim() != expected {
                    report.failures.push(Check {
                        path: format!(".pixi/envs/{name}/conda-meta/.pixi-environment-fingerprint"),
                        kind: Kind::Integrity,
                        detail: format!(
                            "env {name}: fingerprint is {}, the manifest records {expected}",
                            actual.trim()
                        ),
                    });
                }
            }
            Err(_) => {
                report.failures.push(Check {
                    path: format!(".pixi/envs/{name}/conda-meta/.pixi-environment-fingerprint"),
                    kind: Kind::Missing,
                    detail: format!(
                        "env {name}: restore should have written the recorded fingerprint"
                    ),
                });
            }
        }
    }
}

fn check_blob(env: &str, blob: &Blob, abs: &Path, report: &mut Report) {
    if blob.parts.is_empty() {
        if !abs.exists() {
            report.failures.push(Check {
                path: blob.path.clone(),
                kind: Kind::Missing,
                detail: format!("env {env}: blob is declared but not present"),
            });
            return;
        }
        if let Err(e) = shard::verify_file(abs, &blob.sha256, blob.size) {
            report.failures.push(from_error(env, &blob.path, &e));
        }
        return;
    }

    // Split blob: the whole file is *not* on the branch — only its parts are, next to where
    // it would have been — so the parts must each match and their sizes must add up.
    // (The whole file existing would mean someone pushed both halves; not an error.)
    let parts_root = abs.parent().unwrap_or_else(|| Path::new("."));
    let mut total = 0u64;
    for part in &blob.parts {
        let name = Path::new(&part.path)
            .file_name()
            .map(std::borrow::ToOwned::to_owned)
            .unwrap_or_default();
        let part_abs = parts_root.join(name);
        if !part_abs.exists() {
            report.failures.push(Check {
                path: part.path.clone(),
                kind: Kind::MissingPart,
                detail: format!("env {env}: expecting {}", blob.path),
            });
            continue;
        }
        total += part.size;
        if let Err(e) = shard::verify_file(&part_abs, &part.sha256, part.size) {
            report.failures.push(from_error(env, &part.path, &e));
        }
    }
    if total != blob.size && !report.failures.iter().any(|f| f.path == blob.path) {
        report.failures.push(Check {
            path: blob.path.clone(),
            kind: Kind::SizeMismatch,
            detail: format!(
                "env {env}: parts sum to {total} but the blob is {}",
                blob.size
            ),
        });
    }
}

fn from_error(env: &str, path: &str, e: &crate::error::Error) -> Check {
    let kind = match e {
        crate::error::Error::MissingPart(_) => Kind::MissingPart,
        crate::error::Error::SizeMismatch { .. } => Kind::SizeMismatch,
        crate::error::Error::Io { .. } => Kind::Missing,
        _ => Kind::Integrity,
    };
    Check {
        path: path.to_string(),
        kind,
        detail: format!("env {env}: {e}"),
    }
}

/// What kind of executable is this? `Dynamic` is the one that must never be shipped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    /// No interpreter: runs on a bare machine (musl static, or static-PIE).
    Static,
    /// Needs shared libraries / an interpreter from its build prefix.
    Dynamic,
    /// A `#!` script (e.g. a generated launcher): fine, it just needs its interpreter.
    Script,
    /// Mach-O / PE: system-linked by definition; not our airlock problem here.
    System,
    Unknown,
}

impl Linkage {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Linkage::Static => "static",
            Linkage::Dynamic => "dynamic",
            Linkage::Script => "script",
            Linkage::System => "system",
            Linkage::Unknown => "unknown",
        }
    }
}

/// Detect static vs dynamic for the platforms that matter, by reading the ELF headers.
///
/// A dynamically linked executable (including a PIE) has a `PT_INTERP` program header; a
/// static one does not. For Mach-O and PE we return [`Linkage::System`]: they always link
/// against the OS, and the airlock policy for them is a separate decision.
#[must_use]
pub fn linkage_of(path: &Path) -> Linkage {
    match std::fs::File::open(path) {
        // Streaming rather than reading the whole file: this runs over every tool in a
        // transport, and only the headers are ever needed.
        Ok(file) => linkage_of_reader(file),
        Err(_) => Linkage::Unknown,
    }
}

/// The same classification for bytes that are already in memory.
///
/// `tools update` has just downloaded the asset and must judge the *bytes* it is about to pin,
/// before anything is written to disk under a name that claims to be a reviewed static build.
/// One definition, two callers — a second copy of this logic could disagree with the one
/// `verify` uses at restore time, which is the check that has to hold.
#[must_use]
pub fn linkage_of_bytes(bytes: &[u8]) -> Linkage {
    linkage_of_reader(std::io::Cursor::new(bytes))
}

fn linkage_of_reader<R: Read + Seek>(mut reader: R) -> Linkage {
    use std::io::SeekFrom;

    // Mach-O (both endiannesses, 32/64 bit) and PE (`MZ`).
    const MACHO: [[u8; 4]; 4] = [
        [0xfe, 0xed, 0xfa, 0xce],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xcf, 0xfa, 0xed, 0xfe],
    ];

    let mut head = [0u8; 64];
    let Ok(n) = reader.read(&mut head) else {
        return Linkage::Unknown;
    };
    if n < 20 {
        return Linkage::Unknown;
    }

    // Scripts: `#!` — a launcher shipped as text, not a binary.
    if &head[..2] == b"#!" {
        return Linkage::Script;
    }

    if MACHO.contains(&[head[0], head[1], head[2], head[3]]) || &head[..2] == b"MZ" {
        return Linkage::System;
    }

    if &head[..4] != b"\x7fELF" {
        return Linkage::Unknown;
    }

    let little = head[5] == 1;
    let elf64 = head[4] == 2;

    let read_u16 = |b: &[u8]| -> u16 {
        if little {
            u16::from_le_bytes([b[0], b[1]])
        } else {
            u16::from_be_bytes([b[0], b[1]])
        }
    };
    let read_u32 = |b: &[u8]| -> u32 {
        if little {
            u32::from_le_bytes([b[0], b[1], b[2], b[3]])
        } else {
            u32::from_be_bytes([b[0], b[1], b[2], b[3]])
        }
    };
    let read_u64 = |b: &[u8]| -> u64 {
        let mut v = [0u8; 8];
        v.copy_from_slice(&b[..8]);
        if little {
            u64::from_le_bytes(v)
        } else {
            u64::from_be_bytes(v)
        }
    };

    let (phoff, phentsize, phnum) = if elf64 {
        if n < 64 {
            return Linkage::Unknown;
        }
        (
            read_u64(&head[32..40]),
            u64::from(read_u16(&head[54..56])),
            u64::from(read_u16(&head[56..58])),
        )
    } else {
        (
            u64::from(read_u32(&head[28..32])),
            u64::from(read_u16(&head[42..44])),
            u64::from(read_u16(&head[44..46])),
        )
    };

    if phoff == 0 || phentsize == 0 || phnum == 0 {
        // No program headers at all: a relocatable object, not an executable.
        return Linkage::Unknown;
    }
    // A real program header entry is 32 or 56 bytes; anything larger is a malformed
    // file. Bounding it keeps the narrowing below honest — a wrapped entry size would
    // misread the table and could report the wrong linkage.
    if phentsize > 4096 {
        return Linkage::Unknown;
    }

    if reader.seek(SeekFrom::Start(phoff)).is_err() {
        return Linkage::Unknown;
    }
    let entry_len = usize::try_from(phentsize).unwrap_or(usize::MAX);
    let mut entry = vec![0u8; entry_len];
    for _ in 0..phnum.min(64) {
        if reader.read_exact(&mut entry).is_err() {
            return Linkage::Unknown;
        }
        let p_type = read_u32(&entry[..4]);
        if p_type == 3 {
            // PT_INTERP
            return Linkage::Dynamic;
        }
    }
    Linkage::Static
}
