use sha2::{Digest, Sha256};
use std::path::Path;

pub fn sha256_file(path: &Path) -> anyhow::Result<String> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 8192];
    loop {
        let n = file.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn verify_sha256sums(kit_dir: &Path) -> crate::error::Result<()> {
    let sums_path = kit_dir.join("SHA256SUMS");
    if !sums_path.exists() {
        return Ok(());
    }
    let content = std::fs::read_to_string(&sums_path).map_err(|e| {
        crate::error::SandboxError::Integrity(format!("cannot read SHA256SUMS: {e}"))
    })?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Format: "<sha256>  <filename>" or "<sha256> *<filename>"
        let mut parts = line.split_whitespace();
        let expected = parts.next().ok_or_else(|| {
            crate::error::SandboxError::Integrity(format!("malformed SHA256SUMS line: {line}"))
        })?;
        let filename = parts.next().ok_or_else(|| {
            crate::error::SandboxError::Integrity(format!("malformed SHA256SUMS line: {line}"))
        })?;
        // Handle * prefix for binary mode
        let filename = filename.trim_start_matches('*');
        let file_path = kit_dir.join(filename);
        if !file_path.exists() {
            return Err(crate::error::SandboxError::Integrity(format!(
                "SHA256SUMS references missing file: {filename}"
            )));
        }
        let actual = sha256_file(&file_path).map_err(|e| {
            crate::error::SandboxError::Integrity(format!("sha256 failed for {filename}: {e}"))
        })?;
        if actual != expected {
            return Err(crate::error::SandboxError::Integrity(format!(
                "SHA256SUMS mismatch for {filename}: expected {expected}, got {actual} — re-fetch the branch, do not retry"
            )));
        }
    }
    Ok(())
}
