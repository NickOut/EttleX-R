//! Constraint domain model
//!
//! This module defines the domain model for constraints, which are family-agnostic
//! validation rules attached to EPs. Constraints participate in snapshot manifests
//! and maintain deterministic ordering.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::fmt;

/// A constraint instance that can be attached to EPs
///
/// Constraints are family-agnostic and support open extension through string fields.
/// The payload_digest is computed from the serialized payload_json to enable
/// content-addressable storage and deduplication.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Constraint {
    /// Unique identifier (UUIDv7)
    pub constraint_id: String,

    /// Constraint family (e.g., "ABB", "SBB", "Custom")
    pub family: String,

    /// Constraint kind within family (e.g., "OwnershipRule", "ComplianceCheck")
    pub kind: String,

    /// Constraint scope (e.g., "EP", "Leaf", "Subtree")
    pub scope: String,

    /// Constraint configuration as JSON
    pub payload_json: JsonValue,

    /// SHA-256 digest of canonical payload_json
    pub payload_digest: String,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Deletion timestamp (tombstone pattern)
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Constraint {
    /// Create a new constraint instance
    ///
    /// The payload_digest is computed from the canonical JSON representation
    /// of the payload to ensure deterministic content addressing.
    pub fn new(
        constraint_id: String,
        family: String,
        kind: String,
        scope: String,
        payload_json: JsonValue,
    ) -> Self {
        let now = Utc::now();
        let payload_digest = Self::compute_payload_digest(&payload_json);

        Self {
            constraint_id,
            family,
            kind,
            scope,
            payload_json,
            payload_digest,
            created_at: now,
            updated_at: now,
            deleted_at: None,
        }
    }

    /// Compute SHA-256 digest of payload JSON
    ///
    /// Uses canonical JSON serialization (sorted keys) for deterministic hashing.
    fn compute_payload_digest(payload: &JsonValue) -> String {
        let canonical_json =
            serde_json::to_string(payload).expect("JSON value should always serialize");
        let mut hasher = Sha256::new();
        hasher.update(canonical_json.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if constraint is tombstoned (soft-deleted)
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Mark constraint as deleted (tombstone pattern)
    pub fn tombstone(&mut self) {
        self.deleted_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Update constraint payload and recompute digest
    pub fn update_payload(&mut self, new_payload: JsonValue) {
        self.payload_json = new_payload;
        self.payload_digest = Self::compute_payload_digest(&self.payload_json);
        self.updated_at = Utc::now();
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Constraint({}, family={}, kind={}, scope={})",
            self.constraint_id, self.family, self.kind, self.scope
        )
    }
}

/// EP-to-Constraint attachment record
///
/// Represents the many-to-many relationship between EPs and constraints.
/// The ordinal field ensures deterministic ordering for manifest generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EpConstraintRef {
    /// EP identifier
    pub ep_id: String,

    /// Constraint identifier
    pub constraint_id: String,

    /// Ordinal position for deterministic ordering
    pub ordinal: i32,

    /// Attachment timestamp
    pub created_at: DateTime<Utc>,
}

impl EpConstraintRef {
    /// Create a new EP-to-constraint attachment
    pub fn new(ep_id: String, constraint_id: String, ordinal: i32) -> Self {
        Self {
            ep_id,
            constraint_id,
            ordinal,
            created_at: Utc::now(),
        }
    }
}

impl fmt::Display for EpConstraintRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EpConstraintRef(ep={}, constraint={}, ordinal={})",
            self.ep_id, self.constraint_id, self.ordinal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_constraint_new() {
        let payload = json!({"rule": "owner_must_exist"});
        let constraint = Constraint::new(
            "c1".to_string(),
            "ABB".to_string(),
            "OwnershipRule".to_string(),
            "EP".to_string(),
            payload.clone(),
        );

        assert_eq!(constraint.constraint_id, "c1");
        assert_eq!(constraint.family, "ABB");
        assert_eq!(constraint.kind, "OwnershipRule");
        assert_eq!(constraint.scope, "EP");
        assert_eq!(constraint.payload_json, payload);
        assert!(!constraint.payload_digest.is_empty());
        assert!(!constraint.is_deleted());
    }

    #[test]
    fn test_constraint_digest_deterministic() {
        let payload = json!({"rule": "owner_must_exist", "value": 42});
        let c1 = Constraint::new(
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload.clone(),
        );
        let c2 = Constraint::new(
            "c2".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload.clone(),
        );

        assert_eq!(c1.payload_digest, c2.payload_digest);
    }

    #[test]
    fn test_constraint_tombstone() {
        let payload = json!({"rule": "test"});
        let mut constraint = Constraint::new(
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        );

        assert!(!constraint.is_deleted());
        constraint.tombstone();
        assert!(constraint.is_deleted());
    }

    #[test]
    fn test_constraint_update_payload() {
        let payload1 = json!({"rule": "old"});
        let payload2 = json!({"rule": "new"});
        let mut constraint = Constraint::new(
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload1,
        );

        let old_digest = constraint.payload_digest.clone();
        let old_updated = constraint.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        constraint.update_payload(payload2.clone());

        assert_eq!(constraint.payload_json, payload2);
        assert_ne!(constraint.payload_digest, old_digest);
        assert!(constraint.updated_at > old_updated);
    }

    #[test]
    fn test_ep_constraint_ref_new() {
        let ref_record = EpConstraintRef::new("ep1".to_string(), "c1".to_string(), 0);

        assert_eq!(ref_record.ep_id, "ep1");
        assert_eq!(ref_record.constraint_id, "c1");
        assert_eq!(ref_record.ordinal, 0);
    }
}
