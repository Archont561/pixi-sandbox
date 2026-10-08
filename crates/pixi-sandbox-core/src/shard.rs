//! Content sharding.
//!
//! Transport rule (measured, see `.knowledge/design.md` §3):
//! * shards are **whole files** — git content-addresses them, so unchanged files
//!   are deduplicated across envs *and* across publishes for free;
//! * only a single file larger than the shard limit is cut into `.partNNN`
//!   pieces, because GitHub hard-blocks any git blob above 100 MiB.

use crate::error::{Error, Result};
use crate::manifest::{Blob, Part, rel_path_file_name};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Default shard limit: 95 MiB, comfortably under GitHub's 100 MiB per-file block.
pub const DEFAULT_SHARD_LIMIT_BYTES: u64 = 95 * 1024 * 1024;

const COPY_BUFFER: usize = 1 << 20;

fn hex_digest(bytes: impl AsRef<[u8]>) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let bytes = bytes.as_ref();
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

/// Lowercase hex sha256 of a byte slice.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_digest(hasher.finalize())
}

/// Lowercase hex sha256 of a file, streamed.
pub fn sha256_file(path: &Path) -> Result<String> {
    let file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut reader = BufReader::with_capacity(COPY_BUFFER, file);
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; COPY_BUFFER];
    loop {
        let n = reader.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_digest(hasher.finalize()))
}

/// Name of the `index`-th split part of `rel_path` (0-based).
pub fn part_path(rel_path: &str, index: usize) -> String {
    format!("{rel_path}.part{index:03}")
}

/// Verify that `path` matches an expected sha256 and size.
pub fn verify_file(path: &Path, expected_sha256: &str, expected_size: u64) -> Result<()> {
    let meta = fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if meta.len() != expected_size {
        return Err(Error::SizeMismatch {
            path: path.display().to_string(),
            expected: expected_size,
            actual: meta.len(),
        });
    }
    let actual = sha256_file(path)?;
    if actual != expected_sha256 {
        return Err(Error::Integrity {
            path: path.display().to_string(),
            expected: expected_sha256.to_string(),
            actual,
        });
    }
    Ok(())
}

/// Describe a file as a manifest [`Blob`], splitting it in place if it exceeds `limit`.
///
/// `root` is the directory the blob's `path` is relative to. When the file is split,
/// the original is removed and the parts take its place.
pub fn record_file(root: &Path, rel_path: &str, limit: u64) -> Result<Blob> {
    let abs = root.join(rel_path);
    let size = fs::metadata(&abs).map_err(|e| Error::io(&abs, e))?.len();
    let sha256 = sha256_file(&abs)?;

    let mut blob = Blob {
        path: rel_path.to_string(),
        size,
        sha256,
        parts: Vec::new(),
    };

    if size > limit {
        blob.parts = split_file(&abs, rel_path, limit)?;
        fs::remove_file(&abs).map_err(|e| Error::io(&abs, e))?;
    }
    Ok(blob)
}

/// Cut `abs` into `limit`-sized parts named `<rel_path>.partNNN` next to it.
pub fn split_file(abs: &Path, rel_path: &str, limit: u64) -> Result<Vec<Part>> {
    let dir = abs.parent().unwrap_or_else(|| Path::new("."));
    let file = File::open(abs).map_err(|e| Error::io(abs, e))?;
    let mut reader = BufReader::with_capacity(COPY_BUFFER, file);
    let mut parts = Vec::new();
    let mut index = 0usize;
    let mut buf = vec![0u8; COPY_BUFFER];

    loop {
        // read up to `limit` bytes for this part
        let mut written = 0u64;
        let rel_part = part_path(rel_path, index);
        // Built here from `rel_path`, so it always carries a name; the fallible helper keeps
        // this file free of `expect` and costs nothing on the happy path (TASK-80 AC#3).
        let abs_part = dir.join(rel_path_file_name(&rel_part, "part path")?);
        let mut out = BufWriter::new(File::create(&abs_part).map_err(|e| Error::io(&abs_part, e))?);
        let mut hasher = Sha256::new();

        while written < limit {
            let want = std::cmp::min(COPY_BUFFER as u64, limit - written) as usize;
            let n = reader
                .read(&mut buf[..want])
                .map_err(|e| Error::io(abs, e))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])
                .map_err(|e| Error::io(&abs_part, e))?;
            hasher.update(&buf[..n]);
            written += n as u64;
        }
        out.flush().map_err(|e| Error::io(&abs_part, e))?;
        drop(out);

        if written == 0 {
            // nothing more to write; drop the empty part we just created
            let _ = fs::remove_file(&abs_part);
            break;
        }

        parts.push(Part {
            path: rel_part,
            size: written,
            sha256: hex_digest(hasher.finalize()),
        });
        index += 1;

        if written < limit {
            break; // that was the tail
        }
    }

    if parts.len() < 2 {
        return Err(Error::Invalid(format!(
            "split_file({}) produced {} part(s); splitting is pointless below the limit",
            abs.display(),
            parts.len()
        )));
    }
    Ok(parts)
}

/// Reassemble `parts` into `dst`, verifying every part and the result.
///
/// `parts_root` is the directory the parts live in — the directory the split file
/// itself lived in (`channel/linux-64/big.conda.part000` sits next to where
/// `channel/linux-64/big.conda` would have been). It is *not* the destination.
///
/// The join is staged in a sibling temp file and renamed into place only after the
/// whole blob verifies, so a corrupt or truncated branch can never leave a
/// half-written file behind (decision D7).
pub fn join_parts(
    dst: &Path,
    parts_root: &Path,
    parts: &[Part],
    expected_sha256: &str,
    expected_size: u64,
) -> Result<()> {
    let parent = dst.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;

    let tmp = temp_sibling(dst);
    if let Err(err) = assemble(&tmp, parts_root, parts, expected_sha256, expected_size) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    replace_with(&tmp, dst)
}

/// Move the staged file onto `dst`, replacing whatever is already there.
///
/// Rename first and unlink only as a fallback. On Unix the rename is atomic *and* it works
/// when `dst` is a binary this machine is currently executing: it swaps the directory entry
/// and leaves the busy inode to the running process. Removing first would instead open a
/// window in which the tool does not exist at all. Windows cannot rename onto an existing
/// file, so there the fallback is the only way.
fn replace_with(tmp: &Path, dst: &Path) -> Result<()> {
    if fs::rename(tmp, dst).is_ok() {
        return Ok(());
    }
    if dst.exists() {
        fs::remove_file(dst).map_err(|e| Error::io(dst, e))?;
    }
    if let Err(e) = fs::rename(tmp, dst) {
        let _ = fs::remove_file(tmp);
        return Err(Error::io(dst, e));
    }
    Ok(())
}

/// A unique, hidden path next to `dst` — same filesystem, so the final rename is cheap.
fn temp_sibling(dst: &Path) -> PathBuf {
    let name = dst
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "blob".to_string());
    dst.with_file_name(format!(".{name}.join{}", std::process::id()))
}

fn assemble(
    tmp: &Path,
    parts_root: &Path,
    parts: &[Part],
    expected_sha256: &str,
    expected_size: u64,
) -> Result<()> {
    let mut out = BufWriter::new(File::create(tmp).map_err(|e| Error::io(tmp, e))?);
    let mut hasher = Sha256::new();
    let mut total = 0u64;

    for part in parts {
        let src = parts_root.join(rel_path_file_name(&part.path, "part path")?);
        if !src.exists() {
            return Err(Error::MissingPart(part.path.clone()));
        }
        verify_file(&src, &part.sha256, part.size)?;

        let mut reader = BufReader::with_capacity(
            COPY_BUFFER,
            File::open(&src).map_err(|e| Error::io(&src, e))?,
        );
        let mut buf = vec![0u8; COPY_BUFFER];
        loop {
            let n = reader.read(&mut buf).map_err(|e| Error::io(&src, e))?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| Error::io(tmp, e))?;
            hasher.update(&buf[..n]);
            total += n as u64;
        }
    }
    out.flush().map_err(|e| Error::io(tmp, e))?;
    drop(out);

    let actual = hex_digest(hasher.finalize());
    if actual != expected_sha256 {
        return Err(Error::Integrity {
            path: tmp.display().to_string(),
            expected: expected_sha256.to_string(),
            actual,
        });
    }
    if total != expected_size {
        return Err(Error::SizeMismatch {
            path: tmp.display().to_string(),
            expected: expected_size,
            actual: total,
        });
    }
    Ok(())
}

/// Copy a blob (joining parts when needed) from `src_root` into `dst`, verifying it.
///
/// This never writes into `src_root`: a fetched branch checkout is treated as read-only.
pub fn materialise(src_root: &Path, blob: &Blob, dst: &Path) -> Result<()> {
    if blob.parts.is_empty() {
        let src = src_root.join(&blob.path);
        // `std::fs::copy` opens the destination for writing *before* reading the source, so
        // copying a blob onto itself would truncate it. Treat that as the verify it was
        // always meant to be.
        if src == dst {
            return verify_file(dst, &blob.sha256, blob.size);
        }
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        // Staged in a sibling and renamed, exactly like `join_parts` — an unsplit blob deserves
        // the same all-or-nothing guarantee (D7), and there is a second reason the split path
        // never had to state: `fs::copy` opens the *destination* for writing, which Linux
        // refuses with ETXTBSY ("Text file busy") when that destination is a binary currently
        // being executed. `restore` materialising over a live `.pixi/tools/<platform>/pixi` —
        // the very binary a running `pixi run` is executing — is exactly that case, and it
        // failed the restore with an error naming the *source* path.
        let tmp = temp_sibling(dst);
        let staged = fs::copy(&src, &tmp)
            // A missing blob is a fact about the branch, so name the source; everything else
            // (no space, no permission) happened on the side being written.
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => Error::io(&src, e),
                _ => Error::io(&tmp, e),
            })
            .and_then(|_| verify_file(&tmp, &blob.sha256, blob.size));
        if let Err(err) = staged {
            let _ = fs::remove_file(&tmp);
            return Err(err);
        }
        return replace_with(&tmp, dst);
    }
    // The parts sit in the directory the original file lived in, under `src_root`.
    let parts_root = match Path::new(&blob.path).parent() {
        Some(dir) => src_root.join(dir),
        None => src_root.to_path_buf(),
    };
    join_parts(dst, &parts_root, &blob.parts, &blob.sha256, blob.size)
}

/// Read a blob's bytes into memory, verifying size and digest first (joining parts when
/// the blob travelled split).
///
/// For the small metadata blobs (an environment's `files.json`), not payload: a `.conda`
/// archive has no business being held in memory, and the size guard below refuses to
/// believe a manifest that claims one is.
pub fn read_blob(root: &Path, blob: &Blob) -> Result<Vec<u8>> {
    // A corrupted manifest may declare an absurd size; never pre-allocate on that number.
    const INLINE_CAP: u64 = 64 * 1024 * 1024;
    if blob.size > INLINE_CAP {
        return Err(Error::Invalid(format!(
            "{}: refusing to read a {} byte blob into memory",
            blob.path, blob.size
        )));
    }

    if blob.parts.is_empty() {
        let abs = root.join(&blob.path);
        let bytes = fs::read(&abs).map_err(|e| Error::io(&abs, e))?;
        if bytes.len() as u64 != blob.size {
            return Err(Error::SizeMismatch {
                path: blob.path.clone(),
                expected: blob.size,
                actual: bytes.len() as u64,
            });
        }
        let actual = sha256_bytes(&bytes);
        if actual != blob.sha256 {
            return Err(Error::Integrity {
                path: blob.path.clone(),
                expected: blob.sha256.clone(),
                actual,
            });
        }
        return Ok(bytes);
    }

    let parts_root = match Path::new(&blob.path).parent() {
        Some(dir) => root.join(dir),
        None => root.to_path_buf(),
    };
    let mut out = Vec::with_capacity(blob.size.min(INLINE_CAP) as usize);
    for part in &blob.parts {
        let src = parts_root.join(rel_path_file_name(&part.path, "part path")?);
        let bytes = fs::read(&src).map_err(|e| Error::io(&src, e))?;
        if bytes.len() as u64 != part.size {
            return Err(Error::SizeMismatch {
                path: part.path.clone(),
                expected: part.size,
                actual: bytes.len() as u64,
            });
        }
        let actual = sha256_bytes(&bytes);
        if actual != part.sha256 {
            return Err(Error::Integrity {
                path: part.path.clone(),
                expected: part.sha256.clone(),
                actual,
            });
        }
        out.extend_from_slice(&bytes);
    }
    if out.len() as u64 != blob.size {
        return Err(Error::SizeMismatch {
            path: blob.path.clone(),
            expected: blob.size,
            actual: out.len() as u64,
        });
    }
    let actual = sha256_bytes(&out);
    if actual != blob.sha256 {
        return Err(Error::Integrity {
            path: blob.path.clone(),
            expected: blob.sha256.clone(),
            actual,
        });
    }
    Ok(out)
}

/// All files under `root` (recursively), relative paths, sorted — the set of shards
/// a pack operation must record.
pub fn files_under(root: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).sort_by_file_name() {
        let entry = entry.map_err(|e| Error::Other(e.to_string()))?;
        if entry.file_type().is_file() {
            out.push(entry.path().to_path_buf());
        }
    }
    Ok(out)
}
