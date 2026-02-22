//! Snapshot manifest generation and schema.
//!
//! A snapshot manifest is a canonical representation of EPT state at a moment in time.
//! It includes ordered EP entries, policy references, and computed digests.
//!
//! ## Schema Version
//!
//! Current manifest schema version: **1**
//!
//! ## Manifest Fields
//!
//! - `manifest_schema_version`: Schema version (currently 1)
//! - `created_at`: RFC3339 timestamp
//! - `policy_ref`: Policy identifier
//! - `profile_ref`: Profile identifier
//! - `ept`: Ordered list of EP entries with ordinals
//! - `effective_constraints`: Constraint IDs (empty in v0)
//! - `constraint_resolution`: Constraint resolution details (empty in v0)
//! - `coverage`: Coverage metrics (empty in v0)
//! - `exceptions`: Exception records (empty in v0)
//! - `root_ettle_id`: Root ettle identifier
//! - `ept_digest`: Digest of ordered EP IDs
//! - `manifest_digest`: Digest including created_at
//! - `semantic_manifest_digest`: Digest excluding created_at (for idempotency)
//! - `store_schema_version`: Store schema version
//! - `seed_digest`: Optional seed digest

use crate::errors::Result;
use serde::{Deserialize, Serialize};

/// Snapshot manifest schema.
///
/// Contains all fields required for deterministic snapshot commits,
/// including EPT state, policy references, and computed digests.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotManifest {
    /// Manifest schema version (currently 1)
    pub manifest_schema_version: u32,

    /// RFC3339 timestamp of manifest creation
    pub created_at: String,

    /// Policy reference (e.g., "policy/default@0")
    pub policy_ref: String,

    /// Profile reference (e.g., "profile/default@0")
    pub profile_ref: String,

    /// Ordered list of EP entries with computed digests
    pub ept: Vec<EpEntry>,

    /// Effective constraint IDs (empty in v0)
    pub effective_constraints: Vec<String>,

    /// Constraint resolution details (empty in v0)
    pub constraint_resolution: serde_json::Value,

    /// Coverage metrics (empty in v0)
    pub coverage: serde_json::Value,

    /// Exception records (empty in v0)
    pub exceptions: Vec<String>,

    /// Root ettle identifier
    pub root_ettle_id: String,

    /// Digest of ordered EP IDs (computed from ept field)
    pub ept_digest: String,

    /// Full manifest digest including created_at
    pub manifest_digest: String,

    /// Semantic digest excluding created_at (for idempotency checks)
    pub semantic_manifest_digest: String,

    /// Store schema version
    pub store_schema_version: String,

    /// Optional seed digest (if imported from seed)
    pub seed_digest: Option<String>,
}

/// Entry in the EPT (Effective Processing Tree).
///
/// Represents a single EP node with its position, digest, and normative status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpEntry {
    /// EP identifier
    pub ep_id: String,

    /// Zero-based ordinal position in EPT
    pub ordinal: u32,

    /// Whether this EP is normative (always true in v0)
    pub normative: bool,

    /// Content digest of the EP
    pub ep_digest: String,
}

/// Generate a snapshot manifest from EPT state.
///
/// Creates a canonical manifest with all required fields, computed digests,
/// and RFC3339 timestamp. The manifest is ready for persistence to CAS.
///
/// ## Arguments
///
/// - `ept`: Ordered list of EP identifiers (from `compute_ept`)
/// - `policy_ref`: Policy identifier (e.g., "policy/default@0")
/// - `profile_ref`: Profile identifier (e.g., "profile/default@0")
/// - `root_ettle_id`: Root ettle identifier
/// - `store_schema_version`: Store schema version (e.g., "0001")
/// - `seed_digest`: Optional seed digest if imported from seed
///
/// ## Returns
///
/// Populated `SnapshotManifest` with all digests computed.
///
/// ## Errors
///
/// Returns `EttleXError::Serialization` if digest computation fails.
///
/// ## Example
///
/// ```no_run
/// use ettlex_core::snapshot::manifest::generate_manifest;
///
/// let ept = vec!["ep:root:0".to_string(), "ep:root:1".to_string()];
/// let manifest = generate_manifest(
///     ept,
///     "policy/default@0".to_string(),
///     "profile/default@0".to_string(),
///     "ettle:root".to_string(),
///     "0001".to_string(),
///     None,
/// ).unwrap();
/// ```
pub fn generate_manifest(
    ept: Vec<String>,
    policy_ref: String,
    profile_ref: String,
    root_ettle_id: String,
    store_schema_version: String,
    seed_digest: Option<String>,
) -> Result<SnapshotManifest> {
    use super::digest::{compute_ept_digest, compute_manifest_digest, compute_semantic_digest};

    // Create EP entries with ordinals
    let ep_entries: Vec<EpEntry> = ept
        .iter()
        .enumerate()
        .map(|(idx, ep_id)| EpEntry {
            ep_id: ep_id.clone(),
            ordinal: idx as u32,
            normative: true,                     // All EPs are normative in v0
            ep_digest: compute_ep_digest(ep_id), // Stub for now
        })
        .collect();

    // Compute EPT digest from ordered EP IDs
    let ept_digest = compute_ept_digest(&ept)?;

    // Generate timestamp
    let created_at = chrono::Utc::now().to_rfc3339();

    // Create manifest (without full digest initially)
    let mut manifest = SnapshotManifest {
        manifest_schema_version: 1,
        created_at,
        policy_ref,
        profile_ref,
        ept: ep_entries,
        effective_constraints: Vec::new(), // Empty in v0
        constraint_resolution: serde_json::Value::Object(serde_json::Map::new()), // Empty in v0
        coverage: serde_json::Value::Object(serde_json::Map::new()), // Empty in v0
        exceptions: Vec::new(),            // Empty in v0
        root_ettle_id,
        ept_digest,
        manifest_digest: String::new(),          // Computed below
        semantic_manifest_digest: String::new(), // Computed below
        store_schema_version,
        seed_digest,
    };

    // Compute digests (with and without timestamp)
    manifest.semantic_manifest_digest = compute_semantic_digest(&manifest)?;
    manifest.manifest_digest = compute_manifest_digest(&manifest)?;

    Ok(manifest)
}

/// Compute EP content digest.
///
/// Stub implementation - will be replaced with actual EP content hashing
/// when EP persistence is implemented.
fn compute_ep_digest(ep_id: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(ep_id.as_bytes());
    hex::encode(hasher.finalize())
}
