//! Constraint engine boundary module.
//!
//! This module defines the stable evaluation interface for constraints in EttleX Phase 1.
//! It provides the `evaluate()` function which computes the constraint state for a given EPT,
//! producing `declared_refs` (deduplicated, ordered) and per-family `FamilyEvaluation` records
//! with `ConstraintFamilyStatus::Uncomputed` for all families in Phase 1.
//!
//! ## Ordering rules
//!
//! `declared_refs` are ordered by `(ordinal, constraint_id)` — None ordinals sort last
//! (treated as `i32::MAX`). This is deterministic within a single EP. Across EPs in the EPT,
//! the first-seen ordinal for a given constraint_id is used (earlier EPs win).
//!
//! ## UNCOMPUTED semantics
//!
//! In Phase 1, no constraint families have active evaluation logic. All families report
//! `status: Uncomputed`, meaning the manifest records which constraints are declared but
//! does not validate them against the EPT. This is intentional and documented.

use crate::errors::ExError;
use crate::ops::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Context passed to `evaluate()`.
///
/// EP-era fields (`leaf_ep_id`, `ept_ep_ids`) are retired in Slice 03.
/// The struct is retained for API compatibility; `evaluate()` now returns an
/// empty evaluation unconditionally.
pub struct ConstraintEvalCtx {
    /// Retired — EP construct removed in Slice 03.
    #[allow(dead_code)]
    pub leaf_ep_id: String,
    /// Retired — EP construct removed in Slice 03.
    #[allow(dead_code)]
    pub ept_ep_ids: Vec<String>,
    /// Policy reference string (e.g. "policy/default@0")
    pub policy_ref: String,
    /// Profile reference string (e.g. "profile/default@0")
    pub profile_ref: String,
}

/// A single declared constraint reference in the evaluation output.
#[derive(Debug)]
pub struct DeclaredConstraintRef {
    /// The constraint's unique ID
    pub constraint_id: String,
    /// The constraint's family
    pub family: String,
    /// The content digest of the constraint payload
    pub payload_digest: String,
}

/// Evaluation status for a constraint family.
///
/// In Phase 1, all families report `Uncomputed`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConstraintFamilyStatus {
    /// No evaluation has been performed (Phase 1 default for all families)
    #[serde(rename = "UNCOMPUTED")]
    Uncomputed,
}

/// Per-family evaluation record.
#[derive(Debug)]
pub struct FamilyEvaluation {
    /// Evaluation status for this family
    pub status: ConstraintFamilyStatus,
    /// Digest of the family's declared constraint set
    pub digest: String,
    /// Opaque family-specific section (reserved for future use)
    pub opaque_section: Option<serde_json::Value>,
}

/// Full result of a constraint evaluation pass.
#[derive(Debug)]
pub struct ConstraintEvaluation {
    /// Ordered, deduplicated list of constraint references across the EPT
    pub declared_refs: Vec<DeclaredConstraintRef>,
    /// Per-family evaluation map (BTreeMap for deterministic ordering)
    pub families: BTreeMap<String, FamilyEvaluation>,
    /// Digest of the full constraints state (declared refs + family digests)
    pub constraints_digest: String,
}

/// Evaluate constraints for an EPT.
///
/// Collects all constraint references attached to EPs in `ctx.ept_ep_ids`, deduplicates
/// by `constraint_id` (first occurrence wins), sorts by `(ordinal, constraint_id)`,
/// groups by family, and computes deterministic digests.
///
/// # Phase 1 behaviour
///
/// - All families report `ConstraintFamilyStatus::Uncomputed`
/// - EPs not present in the store are silently skipped
/// - Tombstoned constraints attached to EPs are excluded from `declared_refs`
///
/// # Errors
///
/// Returns `ExError` if JSON serialization fails during digest computation.
#[allow(clippy::result_large_err)]
pub fn evaluate(_ctx: &ConstraintEvalCtx, store: &Store) -> Result<ConstraintEvaluation, ExError> {
    use sha2::{Digest as _, Sha256};

    // Slice 03: EP construct retired. EP constraint refs no longer exist.
    // Return empty evaluation unconditionally.
    let _ = store; // suppress unused warning
    let declared_refs: Vec<DeclaredConstraintRef> = Vec::new();

    // Group by family for per-family evaluation
    let mut family_groups: BTreeMap<String, Vec<&DeclaredConstraintRef>> = BTreeMap::new();
    for r in &declared_refs {
        family_groups.entry(r.family.clone()).or_default().push(r);
    }

    // Build per-family evaluations
    let mut families: BTreeMap<String, FamilyEvaluation> = BTreeMap::new();
    for (family_name, refs) in &family_groups {
        // Compute family digest from sorted constraint IDs in this family
        let ids: Vec<&str> = refs.iter().map(|r| r.constraint_id.as_str()).collect();
        let canonical = serde_json::to_string(&ids).map_err(|e| {
            ExError::new(crate::errors::ExErrorKind::Serialization)
                .with_message(format!("Failed to serialize family ids: {}", e))
        })?;
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let digest = hex::encode(hasher.finalize());

        families.insert(
            family_name.clone(),
            FamilyEvaluation {
                status: ConstraintFamilyStatus::Uncomputed,
                digest,
                opaque_section: None,
            },
        );
    }

    // Compute constraints_digest over (declared_ref ids, family names + digests)
    let digest_input: Vec<serde_json::Value> = {
        let ref_ids: Vec<&str> = declared_refs
            .iter()
            .map(|r| r.constraint_id.as_str())
            .collect();
        let family_digests: Vec<(&str, &str)> = families
            .iter()
            .map(|(k, v)| (k.as_str(), v.digest.as_str()))
            .collect();
        vec![
            serde_json::to_value(&ref_ids).unwrap_or(serde_json::Value::Null),
            serde_json::to_value(&family_digests).unwrap_or(serde_json::Value::Null),
        ]
    };

    let canonical = serde_json::to_string(&digest_input).map_err(|e| {
        ExError::new(crate::errors::ExErrorKind::Serialization).with_message(format!(
            "Failed to serialize constraints digest input: {}",
            e
        ))
    })?;
    let mut hasher = Sha256::new();
    hasher.update(canonical.as_bytes());
    let constraints_digest = hex::encode(hasher.finalize());

    Ok(ConstraintEvaluation {
        declared_refs,
        families,
        constraints_digest,
    })
}
