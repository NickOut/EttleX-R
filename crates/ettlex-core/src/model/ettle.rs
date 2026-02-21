use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::metadata::Metadata;

/// Ettle - the fundamental unit of architectural structure
///
/// An Ettle represents a concept, component, or decision point in the architecture.
/// Ettles form a refinement tree through parent-child relationships, with each
/// relationship mediated by an EP (Ettle Partition).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ettle {
    /// Unique identifier for this Ettle (UUID v7)
    pub id: String,

    /// Human-readable title
    pub title: String,

    /// Optional parent Ettle ID (None for root Ettles)
    pub parent_id: Option<String>,

    /// List of EP IDs owned by this Ettle (in creation order, not necessarily ordinal order)
    pub ep_ids: Vec<String>,

    /// Extensible metadata storage
    pub metadata: Metadata,

    /// Timestamp when this Ettle was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when this Ettle was last updated
    pub updated_at: DateTime<Utc>,

    /// Tombstone flag - if true, this Ettle is considered deleted
    pub deleted: bool,
}

impl Ettle {
    /// Create a new Ettle with the given ID and title
    ///
    /// # Arguments
    /// * `id` - Unique identifier (typically UUID v7)
    /// * `title` - Human-readable title
    ///
    /// # Returns
    /// A new Ettle with no parent, no EPs, and current timestamps
    pub fn new(id: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title,
            parent_id: None,
            ep_ids: Vec::new(),
            metadata: Metadata::new(),
            created_at: now,
            updated_at: now,
            deleted: false,
        }
    }

    /// Check if this Ettle is a root (has no parent)
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Check if this Ettle has a parent
    pub fn has_parent(&self) -> bool {
        self.parent_id.is_some()
    }

    /// Check if this Ettle is a leaf (has no EPs with children)
    ///
    /// Note: This only checks if ep_ids is empty. To properly determine
    /// if an Ettle is a leaf, you need to check if any of its EPs have
    /// child_ettle_id set, which requires access to the Store.
    pub fn has_eps(&self) -> bool {
        !self.ep_ids.is_empty()
    }

    /// Check if this Ettle is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted
    }

    /// Add an EP ID to this Ettle's EP list
    pub fn add_ep_id(&mut self, ep_id: String) {
        if !self.ep_ids.contains(&ep_id) {
            self.ep_ids.push(ep_id);
        }
    }

    /// Remove an EP ID from this Ettle's EP list
    #[allow(dead_code)]
    pub(crate) fn remove_ep_id(&mut self, ep_id: &str) {
        self.ep_ids.retain(|id| id != ep_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_ettle() {
        let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());

        assert_eq!(ettle.id, "ettle-1");
        assert_eq!(ettle.title, "Test Ettle");
        assert!(ettle.is_root());
        assert!(!ettle.has_parent());
        assert!(!ettle.has_eps());
        assert!(!ettle.is_deleted());
        assert!(ettle.metadata.is_empty());
    }

    #[test]
    fn test_add_remove_ep_id() {
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        ettle.add_ep_id("ep-1".to_string());
        assert!(ettle.has_eps());
        assert_eq!(ettle.ep_ids.len(), 1);

        ettle.add_ep_id("ep-2".to_string());
        assert_eq!(ettle.ep_ids.len(), 2);

        // Adding duplicate should not increase count
        ettle.add_ep_id("ep-1".to_string());
        assert_eq!(ettle.ep_ids.len(), 2);

        ettle.remove_ep_id("ep-1");
        assert_eq!(ettle.ep_ids.len(), 1);
        assert_eq!(ettle.ep_ids[0], "ep-2");
    }
}
