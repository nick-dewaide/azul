use std::path::{Path, PathBuf};

use anyhow::Result;
use sha2::{Digest, Sha256};

/// Compute a SHA-256 checksum of the given content.
pub fn checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(&result))
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Check if a conflict exists.
/// Returns true if the file on disk has changed since the last known checksum.
pub fn has_conflict(last_known_checksum: &str, current_file_content: &str) -> bool {
    let current = checksum(current_file_content);
    current != last_known_checksum
}

/// Write the Studio version of a script to a sidecar `.studio` file.
pub fn write_studio_sidecar(file_path: &Path, studio_source: &str) -> Result<()> {
    let sidecar_path = sidecar_path(file_path);
    fs_err::write(&sidecar_path, studio_source)?;
    log::warn!(
        "Conflict detected! Studio version written to {}",
        sidecar_path.display()
    );
    Ok(())
}

/// Get the sidecar path for conflict resolution.
pub fn sidecar_path(file_path: &Path) -> PathBuf {
    let mut sidecar = file_path.as_os_str().to_os_string();
    sidecar.push(".studio");
    PathBuf::from(sidecar)
}

/// Mark a file as deleted by renaming it to `.deleted`.
pub fn mark_deleted(file_path: &Path) -> Result<()> {
    let mut deleted_path = file_path.as_os_str().to_os_string();
    deleted_path.push(".deleted");
    let deleted_path = PathBuf::from(deleted_path);

    if file_path.exists() {
        fs_err::rename(file_path, &deleted_path)?;
        log::info!(
            "Marked as deleted: {} → {}",
            file_path.display(),
            deleted_path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checksum_deterministic() {
        let a = checksum("hello world");
        let b = checksum("hello world");
        assert_eq!(a, b);
    }

    #[test]
    fn test_checksum_different_content() {
        let a = checksum("hello");
        let b = checksum("world");
        assert_ne!(a, b);
    }

    #[test]
    fn test_has_conflict() {
        let content = "print('hello')";
        let cs = checksum(content);
        assert!(!has_conflict(&cs, content));
        assert!(has_conflict(&cs, "print('goodbye')"));
    }
}
