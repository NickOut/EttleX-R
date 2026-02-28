use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Ettle Partition (EP) - represents a refinement relationship or statement
///
/// Each EP:
/// - Belongs to exactly one Ettle (via `ettle_id`)
/// - Has a unique ordinal within that Ettle (for deterministic ordering)
/// - Optionally maps to a child Ettle (via `child_ettle_id`)
/// - Contains WHY/WHAT/HOW descriptive text
/// - Has a normative flag indicating whether it's binding
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ep {
    /// Unique identifier for this EP (UUID v7)
    pub id: String,

    /// The Ettle that owns this EP
    pub ettle_id: String,

    /// Ordinal position within the owning Ettle (0-based, immutable after creation)
    pub ordinal: u32,

    /// Optional child Ettle ID that this EP refines to
    pub child_ettle_id: Option<String>,

    /// Whether this EP is normative (binding/required)
    pub normative: bool,

    /// WHY: Rationale or motivation for this partition
    pub why: String,

    /// WHAT: Description of what this partition represents
    pub what: String,

    /// HOW: Implementation or operational details
    pub how: String,

    /// SHA-256 hex digest of canonical WHY+WHAT+HOW JSON (alphabetical keys)
    pub content_digest: String,

    /// Timestamp when this EP was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when this EP was last updated
    pub updated_at: DateTime<Utc>,

    /// Tombstone flag - if true, this EP is considered deleted
    pub deleted: bool,
}

impl Ep {
    /// Create a new EP with the given parameters
    ///
    /// # Arguments
    /// * `id` - Unique identifier (typically UUID v7)
    /// * `ettle_id` - Parent Ettle that owns this EP
    /// * `ordinal` - Position within parent (0-based, immutable)
    /// * `normative` - Whether this EP is binding
    /// * `why` - Rationale text
    /// * `what` - Description text
    /// * `how` - Implementation text
    pub fn new(
        id: String,
        ettle_id: String,
        ordinal: u32,
        normative: bool,
        why: String,
        what: String,
        how: String,
    ) -> Self {
        let now = Utc::now();
        let content_digest = Self::compute_content_digest(&why, &what, &how);
        Self {
            id,
            ettle_id,
            ordinal,
            child_ettle_id: None,
            normative,
            why,
            what,
            how,
            content_digest,
            created_at: now,
            updated_at: now,
            deleted: false,
        }
    }

    fn compute_content_digest(why: &str, what: &str, how: &str) -> String {
        let mut map = std::collections::BTreeMap::new();
        map.insert("how", how);
        map.insert("what", what);
        map.insert("why", why);
        let json = serde_json::to_string(&map).expect("BTreeMap serialization is infallible");
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if this EP is a leaf (has no child)
    pub fn is_leaf(&self) -> bool {
        self.child_ettle_id.is_none()
    }

    /// Check if this EP maps to a child Ettle
    pub fn has_child(&self) -> bool {
        self.child_ettle_id.is_some()
    }

    /// Check if this EP is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ep_content_digest_is_64_chars() {
        let ep = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why text".to_string(),
            "What text".to_string(),
            "How text".to_string(),
        );
        assert_eq!(ep.content_digest.len(), 64, "SHA-256 hex must be 64 chars");
    }

    #[test]
    fn test_ep_content_digest_is_deterministic() {
        let ep1 = Ep::new(
            "ep-id-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Same why".to_string(),
            "Same what".to_string(),
            "Same how".to_string(),
        );
        let ep2 = Ep::new(
            "ep-id-2".to_string(), // different id
            "ettle-2".to_string(), // different ettle
            1,
            false,
            "Same why".to_string(),
            "Same what".to_string(),
            "Same how".to_string(),
        );
        assert_eq!(
            ep1.content_digest, ep2.content_digest,
            "Same content → same digest regardless of id/ettle"
        );
    }

    #[test]
    fn test_ep_content_digest_changes_with_content() {
        let ep1 = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why A".to_string(),
            "What A".to_string(),
            "How A".to_string(),
        );
        let ep2 = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why B".to_string(),
            "What B".to_string(),
            "How B".to_string(),
        );
        assert_ne!(
            ep1.content_digest, ep2.content_digest,
            "Different content → different digest"
        );
    }

    #[test]
    fn test_new_ep() {
        let ep = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why text".to_string(),
            "What text".to_string(),
            "How text".to_string(),
        );

        assert_eq!(ep.id, "ep-1");
        assert_eq!(ep.ettle_id, "ettle-1");
        assert_eq!(ep.ordinal, 0);
        assert!(ep.normative);
        assert_eq!(ep.why, "Why text");
        assert_eq!(ep.what, "What text");
        assert_eq!(ep.how, "How text");
        assert!(ep.is_leaf());
        assert!(!ep.has_child());
        assert!(!ep.is_deleted());
    }
}
