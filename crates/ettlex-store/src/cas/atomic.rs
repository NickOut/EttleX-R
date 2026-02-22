//! Atomic write primitives
//!
//! Uses temp→rename pattern to ensure no partial writes

#![allow(clippy::result_large_err)]

use crate::errors::{io_error, Result};
use std::fs;
use std::path::Path;

/// Atomically write bytes to a file
///
/// Uses temp file + rename to ensure atomic write
pub fn atomic_write(target_path: &Path, content: &[u8]) -> Result<()> {
    // Create parent directory if it doesn't exist
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent).map_err(|e| io_error("create_cas_dir", e))?;
    }

    // Create temp file in the same directory
    let temp_path = target_path.with_extension("tmp");

    // Write to temp file
    fs::write(&temp_path, content).map_err(|e| io_error("write_cas_temp", e))?;

    // Atomically rename temp to target
    fs::rename(&temp_path, target_path).map_err(|e| io_error("rename_cas_temp", e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_atomic_write() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        atomic_write(&target, b"hello").unwrap();

        let content = fs::read(&target).unwrap();
        assert_eq!(content, b"hello");
    }

    #[test]
    fn test_atomic_write_creates_parent() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("subdir").join("test.txt");

        atomic_write(&target, b"nested").unwrap();

        let content = fs::read(&target).unwrap();
        assert_eq!(content, b"nested");
    }

    #[test]
    fn test_no_tmp_files_after_write() {
        let temp_dir = TempDir::new().unwrap();
        let target = temp_dir.path().join("test.txt");

        atomic_write(&target, b"clean").unwrap();

        // Check no .tmp files remain
        let tmp_count = fs::read_dir(temp_dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.ends_with(".tmp"))
                    .unwrap_or(false)
            })
            .count();

        assert_eq!(tmp_count, 0);
    }
}
