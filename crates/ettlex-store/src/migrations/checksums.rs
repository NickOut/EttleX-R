//! Checksum validation for migrations
//!
//! Computes SHA256 checksums of migration SQL to detect tampering

use sha2::{Digest, Sha256};

/// Compute SHA256 checksum of a string
pub fn compute_checksum(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_checksum() {
        let content = "SELECT 1";
        let checksum = compute_checksum(content);
        assert_eq!(checksum.len(), 64); // SHA256 is 64 hex chars
    }

    #[test]
    fn test_checksum_deterministic() {
        let content = "SELECT 1";
        let checksum1 = compute_checksum(content);
        let checksum2 = compute_checksum(content);
        assert_eq!(checksum1, checksum2);
    }
}
