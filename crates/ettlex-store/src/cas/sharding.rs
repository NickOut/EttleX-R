//! Sharding logic for CAS
//!
//! Shards blobs into subdirectories based on the first 2 hex characters
//! of the digest to avoid filesystem performance issues with too many
//! files in a single directory.

use std::path::{Path, PathBuf};

/// Compute the shard path for a given digest
///
/// For digest "abc123...", returns "<root>/ab/abc123.<ext>"
pub fn shard_path(root: &Path, digest: &str, extension: &str) -> PathBuf {
    // Get first 2 chars for shard directory
    let shard = &digest[..2.min(digest.len())];

    root.join(shard).join(format!("{}.{}", digest, extension))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_path() {
        let root = Path::new("/cas");
        let digest = "abc123def456";
        let path = shard_path(root, digest, "txt");

        assert_eq!(path, PathBuf::from("/cas/ab/abc123def456.txt"));
    }

    #[test]
    fn test_shard_path_full_digest() {
        let root = Path::new("/cas");
        let digest = "a".repeat(64); // Full SHA256
        let path = shard_path(root, &digest, "bin");

        let expected_shard = "aa";
        assert!(path.starts_with(Path::new("/cas").join(expected_shard)));
    }
}
