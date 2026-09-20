//! Content sharding.
//!
//! Transport rule (measured, see `.knowledge/design.md` §3):
//! * shards are **whole files** — git content-addresses them, so unchanged files
//!   are deduplicated across envs *and* across publishes for free;
//! * only a single file larger than the shard limit is cut into `.partNNN`
//!   pieces, because GitHub hard-blocks any git blob above 100 MiB.

use crate::error::{Error, Result};
use crate::manifest::{Blob, Part};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

/// Default shard limit: 95 MiB, comfortably under GitHub's 100 MiB per-file block.
pub const DEFAULT_SHARD_LIMIT_BYTES: u64 = 95 * 1024 * 1024;

const COPY_BUFFER: usize = 1 << 20;

/// Lowercase hex sha256 of a byte slice.
pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
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
    Ok(format!("{:x}", hasher.finalize()))
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
        let abs_part = dir.join(Path::new(&rel_part).file_name().expect("file name"));
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
            sha256: format!("{:x}", hasher.finalize()),
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
    // Windows cannot rename onto an existing file; on Unix this replaces it.
    if dst.exists() {
        fs::remove_file(dst).map_err(|e| Error::io(dst, e))?;
    }
    if let Err(e) = fs::rename(&tmp, dst) {
        let _ = fs::remove_file(&tmp);
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
        let src = parts_root.join(Path::new(&part.path).file_name().expect("file name"));
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

    let actual = format!("{:x}", hasher.finalize());
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
        fs::copy(&src, dst).map_err(|e| Error::io(&src, e))?;
        return verify_file(dst, &blob.sha256, blob.size);
    }
    // The parts sit in the directory the original file lived in, under `src_root`.
    let parts_root = match Path::new(&blob.path).parent() {
        Some(dir) => src_root.join(dir),
        None => src_root.to_path_buf(),
    };
    join_parts(dst, &parts_root, &blob.parts, &blob.sha256, blob.size)
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
