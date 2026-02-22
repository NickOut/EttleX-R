//! Filesystem-based Content-Addressable Storage
//!
//! Provides atomic writes, collision detection, and content-addressed reads

#![allow(clippy::result_large_err)]

use crate::cas::atomic::atomic_write;
use crate::cas::sharding::shard_path;
use crate::errors::{cas_collision, cas_missing, io_error, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// Filesystem-based CAS store
pub struct FsStore {
    root: PathBuf,
}

impl FsStore {
    /// Create a new CAS store at the given root directory
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Write content to CAS and return the digest
    ///
    /// - Computes SHA256 digest
    /// - Writes atomically using temp→rename
    /// - Idempotent: writing same content twice succeeds
    /// - Detects collisions: writing different content with same digest fails
    pub fn write(&self, content: &[u8], extension: &str) -> Result<String> {
        // Compute digest
        let digest = self.compute_digest(content);

        // Compute target path
        let target_path = shard_path(&self.root, &digest, extension);

        // Check if file already exists
        if target_path.exists() {
            // Verify content matches (idempotency + collision detection)
            let existing_content = fs::read(&target_path).map_err(|e| io_error("read_cas", e))?;

            if existing_content == content {
                // Idempotent: same content, same digest - OK
                return Ok(digest);
            } else {
                // Collision: different content, same digest - ERROR
                return Err(cas_collision(&digest));
            }
        }

        // Write atomically
        atomic_write(&target_path, content)?;

        Ok(digest)
    }

    /// Read content from CAS by digest
    ///
    /// Returns error if blob not found
    pub fn read(&self, digest: &str) -> Result<Vec<u8>> {
        // Try common extensions
        let extensions = ["txt", "bin", "json", "md"];

        for ext in &extensions {
            let path = shard_path(&self.root, digest, ext);
            if path.exists() {
                return fs::read(&path).map_err(|e| io_error("read_cas", e));
            }
        }

        // Not found with any extension
        Err(cas_missing(digest))
    }

    /// Compute SHA256 digest of content
    fn compute_digest(&self, content: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(content);
        let result = hasher.finalize();
        hex::encode(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_test_cas() -> (FsStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let cas = FsStore::new(temp_dir.path());
        (cas, temp_dir)
    }

    #[test]
    fn test_write_read_roundtrip() {
        let (cas, _dir) = setup_test_cas();

        let content = b"Hello, CAS!";
        let digest = cas.write(content, "txt").unwrap();

        let read_content = cas.read(&digest).unwrap();
        assert_eq!(content, &read_content[..]);
    }

    #[test]
    fn test_idempotent_write() {
        let (cas, _dir) = setup_test_cas();

        let content = b"Idempotent";
        let digest1 = cas.write(content, "txt").unwrap();
        let digest2 = cas.write(content, "txt").unwrap();

        assert_eq!(digest1, digest2);
    }

    #[test]
    fn test_read_missing() {
        let (cas, _dir) = setup_test_cas();

        let fake_digest = "0".repeat(64);
        let result = cas.read(&fake_digest);

        assert!(result.is_err());
    }

    #[test]
    fn test_digest_is_sha256() {
        let (cas, _dir) = setup_test_cas();

        let content = b"test";
        let digest = cas.write(content, "txt").unwrap();

        assert_eq!(digest.len(), 64); // SHA256 is 64 hex chars
    }
}
