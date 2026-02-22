//! Digest computation for snapshot manifests.
//!
//! Provides deterministic SHA256 digest computation for EPT lists and
//! snapshot manifests. Follows the same pattern as seed digest computation.
//!
//! ## Digest Types
//!
//! - **EPT Digest**: Hash of ordered EP IDs
//! - **Manifest Digest**: Hash of full manifest (includes `created_at`)
//! - **Semantic Digest**: Hash excluding `created_at` (for idempotency)
//!
//! ## Determinism Guarantees
//!
//! - Same input → same digest (canonical JSON serialization)
//! - Different order → different digest (order-sensitive)
//! - Semantic digest stable across timestamps

use crate::errors::Result;
use crate::snapshot::manifest::SnapshotManifest;
use sha2::{Digest, Sha256};

/// Compute digest of ordered EPT.
///
/// Creates a deterministic SHA256 hash of the ordered EP ID list.
/// Order matters: different orderings produce different digests.
///
/// ## Arguments
///
/// - `ept`: Ordered list of EP identifiers
///
/// ## Returns
///
/// Hex-encoded SHA256 digest (64 characters)
///
/// ## Errors
///
/// Returns `EttleXError::Serialization` if JSON serialization fails.
///
/// ## Example
///
/// ```no_run
/// use ettlex_core::snapshot::digest::compute_ept_digest;
///
/// let ept = vec!["ep:a".to_string(), "ep:b".to_string()];
/// let digest = compute_ept_digest(&ept).unwrap();
/// assert_eq!(digest.len(), 64); // SHA256 hex length
/// ```
pub fn compute_ept_digest(ept: &[String]) -> Result<String> {
    let canonical = serde_json::to_string(ept)?;
    Ok(hash_string(&canonical))
}

/// Compute full manifest digest (includes `created_at`).
///
/// Creates a SHA256 hash of the complete manifest including the timestamp.
/// Different timestamps produce different digests.
///
/// ## Arguments
///
/// - `manifest`: Complete snapshot manifest
///
/// ## Returns
///
/// Hex-encoded SHA256 digest (64 characters)
///
/// ## Errors
///
/// Returns `EttleXError::Serialization` if JSON serialization fails.
pub fn compute_manifest_digest(manifest: &SnapshotManifest) -> Result<String> {
    let canonical = serde_json::to_string(manifest)?;
    Ok(hash_string(&canonical))
}

/// Compute semantic manifest digest (excludes `created_at`).
///
/// Creates a SHA256 hash of the manifest with the timestamp field excluded.
/// This enables idempotency checks: the same semantic state produces the
/// same digest regardless of when the snapshot was created.
///
/// ## Arguments
///
/// - `manifest`: Complete snapshot manifest
///
/// ## Returns
///
/// Hex-encoded SHA256 digest (64 characters)
///
/// ## Errors
///
/// Returns `EttleXError::Serialization` if JSON serialization fails.
///
/// ## Idempotency Property
///
/// ```text
/// manifest1.created_at != manifest2.created_at
/// BUT
/// compute_semantic_digest(manifest1) == compute_semantic_digest(manifest2)
/// IF all other fields are identical
/// ```
pub fn compute_semantic_digest(manifest: &SnapshotManifest) -> Result<String> {
    // Create a copy with created_at zeroed out
    let mut manifest_copy = manifest.clone();
    manifest_copy.created_at = String::new();

    // Also zero out the manifest_digest (which includes timestamp)
    manifest_copy.manifest_digest = String::new();

    let canonical = serde_json::to_string(&manifest_copy)?;
    Ok(hash_string(&canonical))
}

/// Hash a string using SHA256.
///
/// Internal helper for deterministic digest computation.
fn hash_string(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string_deterministic() {
        let input = "test";
        let hash1 = hash_string(input);
        let hash2 = hash_string(input);
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }

    #[test]
    fn test_hash_string_different_inputs() {
        let hash1 = hash_string("test1");
        let hash2 = hash_string("test2");
        assert_ne!(hash1, hash2);
    }
}
