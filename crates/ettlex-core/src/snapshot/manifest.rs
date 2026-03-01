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
//! - `constraints`: Constraints envelope (family-agnostic, extensible)
//!   - `declared_refs`: Ordered list of constraint refs
//!   - `families`: Map of family-specific constraint data (BTreeMap for determinism)
//!   - `applicable_abb`: Frozen ABB projection (backward compatibility)
//!   - `resolved_sbb`: Frozen SBB projection (backward compatibility)
//!   - `resolution_evidence`: Evidence records
//!   - `constraints_digest`: Digest of constraints envelope
//! - `coverage`: Coverage metrics (empty in v0)
//! - `exceptions`: Exception records (empty in v0)
//! - `root_ettle_id`: Root ettle identifier
//! - `ept_digest`: Digest of ordered EP IDs
//! - `manifest_digest`: Digest including created_at
//! - `semantic_manifest_digest`: Digest excluding created_at (for idempotency)
//! - `store_schema_version`: Store schema version
//! - `seed_digest`: Optional seed digest

use crate::constraint_engine::{ConstraintEvalCtx, ConstraintFamilyStatus};
use crate::errors::Result;
use crate::ops::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Constraints envelope for snapshot manifests.
///
/// Provides a family-agnostic, extensible structure for constraint tracking
/// while preserving frozen ABB→SBB projections for backward compatibility.
///
/// ## Determinism Guarantees
///
/// - `declared_refs`: Ordered by family, kind, id (lexicographic)
/// - `families`: BTreeMap ensures deterministic key ordering
/// - All nested lists maintain deterministic order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConstraintsEnvelope {
    /// Ordered list of constraint refs active on the EPT
    pub declared_refs: Vec<String>,

    /// Map of family_name → family-specific data
    /// BTreeMap ensures deterministic serialization order
    pub families: BTreeMap<String, FamilyConstraints>,

    /// Frozen ABB constraint projection (for backward compatibility)
    pub applicable_abb: Vec<String>,

    /// Frozen SBB constraint projection (for backward compatibility)
    pub resolved_sbb: Vec<String>,

    /// Resolution evidence records (predicate matches, selected ids, etc.)
    pub resolution_evidence: Vec<serde_json::Value>,

    /// Digest of canonicalized constraints envelope (excluding manifest-level created_at)
    pub constraints_digest: String,
}

impl ConstraintsEnvelope {
    /// Create a constraints envelope from EPT constraints
    ///
    /// Collects all constraints attached to EPs in the EPT, groups them by family,
    /// and computes digests. ABB/SBB projections are kept empty for backward compatibility.
    ///
    /// # Arguments
    ///
    /// * `ept` - Ordered list of EP IDs in the EPT
    /// * `store` - Store containing constraint data
    ///
    /// # Returns
    ///
    /// A populated ConstraintsEnvelope with deterministic ordering
    ///
    /// # Errors
    ///
    /// Returns `EttleXError::Serialization` if JSON serialization fails during digest computation.
    pub fn from_ept(ept: &[String], store: &Store) -> Result<Self> {
        use crate::constraint_engine;

        // Use a synthetic leaf EP ID (first EP in EPT, or empty string for empty EPT)
        let leaf_ep_id = ept.first().cloned().unwrap_or_default();

        let ctx = ConstraintEvalCtx {
            leaf_ep_id,
            ept_ep_ids: ept.to_vec(),
            policy_ref: String::new(),
            profile_ref: String::new(),
        };

        let eval = constraint_engine::evaluate(&ctx, store).map_err(|e| {
            crate::errors::EttleXError::Serialization {
                message: format!("constraint_engine::evaluate failed: {}", e),
            }
        })?;

        // Build declared_refs as plain constraint IDs (ordinal-ordered, deduplicated)
        let declared_refs: Vec<String> = eval
            .declared_refs
            .iter()
            .map(|r| r.constraint_id.clone())
            .collect();

        // Build per-family data from evaluation result
        let mut family_constraints_map: BTreeMap<String, FamilyConstraints> = BTreeMap::new();
        for (family_name, family_eval) in eval.families {
            // active_refs: plain constraint IDs for this family
            let active_refs: Vec<String> = eval
                .declared_refs
                .iter()
                .filter(|r| r.family == family_name)
                .map(|r| r.constraint_id.clone())
                .collect();

            family_constraints_map.insert(
                family_name,
                FamilyConstraints {
                    status: family_eval.status,
                    active_refs,
                    outcomes: Vec::new(), // Empty in v0
                    evidence: Vec::new(), // Empty in v0
                    digest: family_eval.digest,
                },
            );
        }

        Ok(ConstraintsEnvelope {
            declared_refs,
            families: family_constraints_map,
            applicable_abb: Vec::new(), // Backward compat only, kept empty
            resolved_sbb: Vec::new(),   // Backward compat only, kept empty
            resolution_evidence: Vec::new(),
            constraints_digest: eval.constraints_digest,
        })
    }
}

/// Family-specific constraint data.
///
/// Allows different constraint families (ABB→SBB, observability, etc.)
/// to coexist without schema lock-in.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FamilyConstraints {
    /// Evaluation status for this family (UNCOMPUTED in Phase 1)
    pub status: ConstraintFamilyStatus,

    /// Active constraint IDs for this family (plain IDs, not "family:kind:id")
    pub active_refs: Vec<String>,

    /// Outcomes from constraint evaluation
    pub outcomes: Vec<serde_json::Value>,

    /// Evidence supporting the outcomes
    pub evidence: Vec<serde_json::Value>,

    /// Digest of this family's data
    pub digest: String,
}

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

    /// Constraints envelope (family-agnostic, extensible)
    pub constraints: ConstraintsEnvelope,

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
/// use ettlex_core::ops::Store;
///
/// let ept = vec!["ep:root:0".to_string(), "ep:root:1".to_string()];
/// let manifest = generate_manifest(
///     ept,
///     "policy/default@0".to_string(),
///     "profile/default@0".to_string(),
///     "ettle:root".to_string(),
///     "0001".to_string(),
///     None,
///     &Store::new(),
/// ).unwrap();
/// ```
pub fn generate_manifest(
    ept: Vec<String>,
    policy_ref: String,
    profile_ref: String,
    root_ettle_id: String,
    store_schema_version: String,
    seed_digest: Option<String>,
    store: &Store,
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

    // Create constraints envelope from EPT
    let constraints = ConstraintsEnvelope::from_ept(&ept, store)?;

    // Create manifest (without full digest initially)
    let mut manifest = SnapshotManifest {
        manifest_schema_version: 1,
        created_at,
        policy_ref,
        profile_ref,
        ept: ep_entries,
        constraints,
        coverage: serde_json::Value::Object(serde_json::Map::new()), // Empty in v0
        exceptions: Vec::new(),                                      // Empty in v0
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
