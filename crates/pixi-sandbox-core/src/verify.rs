//! Verification: everything a person needs before trusting a branch, and nothing that writes.
//!
//! Two jobs live here:
//!
//! 1. **Check every declared byte.** [`verify`] walks a manifest and reports *all* failures
//!    instead of stopping at the first — an airlock operator wants the full list, not a game
//!    of whack-a-mole.
//! 2. **Refuse to ship a dynamic tool** (decision D4). The `~/.pixi/bin` shims are 766 KiB
//!    trampolines that exec a dynamically linked binary inside their own prefix; a transport
//!    that carries one works perfectly on the machine that built it and fails on the airlock.
//!    [`linkage_of`] is what catches that.

use crate::manifest::{Blob, MANIFEST_DIR, MANIFEST_FILE, Manifest};
use crate::shard;
use std::io::Read;
use std::path::{Path, PathBuf};

/// Where the manifest of a transport / extracted branch lives.
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
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Missing => "missing",
            Kind::Integrity => "integrity",
            Kind::SizeMismatch => "size",
            Kind::MissingPart => "missing-part",
            Kind::DynamicTool => "dynamic-tool",
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
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn mebibytes(&self) -> f64 {
        self.bytes as f64 / (1024.0 * 1024.0)
    }
}

/// Check every blob of the selected environments (all of them when `envs` is `None`),
/// plus the embedded tools' integrity and linkage.
///
/// Never writes anything, never needs the network.
pub fn verify(manifest: &Manifest, branch_location: &Path, envs: Option<&[String]>) -> Report {
    let mut report = Report::default();

    for (env, blob) in manifest.blobs(envs) {
        let abs = manifest.blob_abs_path(branch_location, blob);
        report.files += 1;
        report.bytes += blob.size;
        check_blob(env, blob, &abs, &mut report);
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
            report.failures.push(from_error(env, &blob.path, e));
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
            .map(|n| n.to_owned())
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
            report.failures.push(from_error(env, &part.path, e));
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

fn from_error(env: &str, path: &str, e: crate::error::Error) -> Check {
    let kind = match &e {
        crate::error::Error::MissingPart(_) => Kind::MissingPart,
        crate::error::Error::SizeMismatch { .. } => Kind::SizeMismatch,
        crate::error::Error::Integrity { .. } => Kind::Integrity,
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
    /// A script (e.g. the reference prototype): fine, it just needs its interpreter.
    Script,
    /// Mach-O / PE: system-linked by definition; not our airlock problem here.
    System,
    Unknown,
}

impl Linkage {
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
pub fn linkage_of(path: &Path) -> Linkage {
    let mut file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Linkage::Unknown,
    };
    let mut head = [0u8; 64];
    let n = match file.read(&mut head) {
        Ok(n) => n,
        Err(_) => return Linkage::Unknown,
    };
    if n < 20 {
        return Linkage::Unknown;
    }

    // Scripts: `#!` — the prototype ships this way.
    if &head[..2] == b"#!" {
        return Linkage::Script;
    }

    // Mach-O (both endiannesses, 32/64 bit) and PE (`MZ`).
    const MACHO: [[u8; 4]; 4] = [
        [0xfe, 0xed, 0xfa, 0xce],
        [0xce, 0xfa, 0xed, 0xfe],
        [0xfe, 0xed, 0xfa, 0xcf],
        [0xcf, 0xfa, 0xed, 0xfe],
    ];
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
            read_u16(&head[54..56]) as u64,
            read_u16(&head[56..58]) as u64,
        )
    } else {
        (
            read_u32(&head[28..32]) as u64,
            read_u16(&head[42..44]) as u64,
            read_u16(&head[44..46]) as u64,
        )
    };

    if phoff == 0 || phentsize == 0 || phnum == 0 {
        // No program headers at all: a relocatable object, not an executable.
        return Linkage::Unknown;
    }

    use std::io::{Seek, SeekFrom};
    if file.seek(SeekFrom::Start(phoff)).is_err() {
        return Linkage::Unknown;
    }
    let mut entry = vec![0u8; phentsize as usize];
    for _ in 0..phnum.min(64) {
        if file.read_exact(&mut entry).is_err() {
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
