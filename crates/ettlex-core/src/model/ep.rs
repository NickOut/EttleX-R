use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
        Self {
            id,
            ettle_id,
            ordinal,
            child_ettle_id: None,
            normative,
            why,
            what,
            how,
            created_at: now,
            updated_at: now,
            deleted: false,
        }
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
