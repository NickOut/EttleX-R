//! Anchor policy trait and implementations
//!
//! This module defines the `AnchorPolicy` trait, which determines whether
//! entities should be preserved (anchored) or can be hard-deleted during
//! deletion operations.

use std::collections::HashSet;

/// Policy trait for determining anchoring status of entities
///
/// Anchored entities are preserved via tombstone deletion (deleted=true flag),
/// while non-anchored entities can be hard-deleted (removed from storage).
///
/// This policy is injected into the `apply()` function to control deletion
/// behavior based on application-specific rules (e.g., published vs. draft).
pub trait AnchorPolicy {
    /// Check if an EP should be anchored (preserved via tombstone)
    ///
    /// # Arguments
    /// * `ep_id` - ID of the EP to check
    ///
    /// # Returns
    /// * `true` - EP should be tombstoned on delete (preserve history)
    /// * `false` - EP can be hard-deleted (remove from storage)
    fn is_anchored_ep(&self, ep_id: &str) -> bool;

    /// Check if an Ettle should be anchored (preserved via tombstone)
    ///
    /// Note: In Phase 0.5, all Ettle deletions are tombstone-only regardless
    /// of policy. This method is provided for forward compatibility.
    ///
    /// # Arguments
    /// * `ettle_id` - ID of the Ettle to check
    ///
    /// # Returns
    /// * `true` - Ettle should be tombstoned on delete
    /// * `false` - Ettle can be hard-deleted (not used in Phase 0.5)
    fn is_anchored_ettle(&self, ettle_id: &str) -> bool;
}

/// Policy that treats all entities as non-anchored (churn mode)
///
/// This policy allows hard deletion of all non-anchored entities, which is
/// useful during prototyping/design phases where artifacts are frequently
/// created and discarded.
///
/// # Example
/// ```
/// use ettlex_core::policy::{AnchorPolicy, NeverAnchoredPolicy};
///
/// let policy = NeverAnchoredPolicy;
/// assert_eq!(policy.is_anchored_ep("any-ep-id"), false);
/// assert_eq!(policy.is_anchored_ettle("any-ettle-id"), false);
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NeverAnchoredPolicy;

impl AnchorPolicy for NeverAnchoredPolicy {
    fn is_anchored_ep(&self, _ep_id: &str) -> bool {
        false
    }

    fn is_anchored_ettle(&self, _ettle_id: &str) -> bool {
        false
    }
}

/// Policy that anchors only specific declared entities
///
/// This policy allows fine-grained control over which entities are preserved
/// vs. hard-deleted, based on explicit ID sets.
///
/// # Example
/// ```
/// use ettlex_core::policy::{AnchorPolicy, SelectedAnchoredPolicy};
/// use std::collections::HashSet;
///
/// let mut anchored_eps = HashSet::new();
/// anchored_eps.insert("ep-published-1".to_string());
///
/// let policy = SelectedAnchoredPolicy::new(anchored_eps, HashSet::new());
///
/// assert_eq!(policy.is_anchored_ep("ep-published-1"), true);
/// assert_eq!(policy.is_anchored_ep("ep-draft-2"), false);
/// ```
#[derive(Debug, Clone, Default)]
pub struct SelectedAnchoredPolicy {
    /// Set of EP IDs that should be anchored
    anchored_eps: HashSet<String>,
    /// Set of Ettle IDs that should be anchored
    anchored_ettles: HashSet<String>,
}

impl SelectedAnchoredPolicy {
    /// Create a new SelectedAnchoredPolicy with specified anchored entity sets
    ///
    /// # Arguments
    /// * `anchored_eps` - Set of EP IDs to anchor
    /// * `anchored_ettles` - Set of Ettle IDs to anchor
    pub fn new(anchored_eps: HashSet<String>, anchored_ettles: HashSet<String>) -> Self {
        Self {
            anchored_eps,
            anchored_ettles,
        }
    }

    /// Create a policy with only specified EPs anchored
    pub fn with_eps(anchored_eps: HashSet<String>) -> Self {
        Self::new(anchored_eps, HashSet::new())
    }

    /// Create a policy with only specified Ettles anchored
    pub fn with_ettles(anchored_ettles: HashSet<String>) -> Self {
        Self::new(HashSet::new(), anchored_ettles)
    }
}

impl AnchorPolicy for SelectedAnchoredPolicy {
    fn is_anchored_ep(&self, ep_id: &str) -> bool {
        self.anchored_eps.contains(ep_id)
    }

    fn is_anchored_ettle(&self, ettle_id: &str) -> bool {
        self.anchored_ettles.contains(ettle_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_never_anchored_policy() {
        let policy = NeverAnchoredPolicy;

        assert!(!policy.is_anchored_ep("ep-1"));
        assert!(!policy.is_anchored_ep("ep-2"));
        assert!(!policy.is_anchored_ettle("ettle-1"));
        assert!(!policy.is_anchored_ettle("ettle-2"));
    }

    #[test]
    fn test_selected_anchored_policy_eps() {
        let mut anchored = HashSet::new();
        anchored.insert("ep-published".to_string());
        anchored.insert("ep-important".to_string());

        let policy = SelectedAnchoredPolicy::with_eps(anchored);

        assert!(policy.is_anchored_ep("ep-published"));
        assert!(policy.is_anchored_ep("ep-important"));
        assert!(!policy.is_anchored_ep("ep-draft"));
        assert!(!policy.is_anchored_ettle("any-ettle"));
    }

    #[test]
    fn test_selected_anchored_policy_ettles() {
        let mut anchored = HashSet::new();
        anchored.insert("ettle-v1".to_string());

        let policy = SelectedAnchoredPolicy::with_ettles(anchored);

        assert!(policy.is_anchored_ettle("ettle-v1"));
        assert!(!policy.is_anchored_ettle("ettle-draft"));
        assert!(!policy.is_anchored_ep("any-ep"));
    }

    #[test]
    fn test_selected_anchored_policy_both() {
        let mut eps = HashSet::new();
        eps.insert("ep-1".to_string());

        let mut ettles = HashSet::new();
        ettles.insert("ettle-1".to_string());

        let policy = SelectedAnchoredPolicy::new(eps, ettles);

        assert!(policy.is_anchored_ep("ep-1"));
        assert!(!policy.is_anchored_ep("ep-2"));
        assert!(policy.is_anchored_ettle("ettle-1"));
        assert!(!policy.is_anchored_ettle("ettle-2"));
    }
}
