use std::collections::HashMap;

use crate::errors::{ExError, ExErrorKind, Result};
use crate::model::{Constraint, Decision, DecisionEvidenceItem, DecisionLink, Ettle};

/// In-memory store for Ettles and Constraints
///
/// This is a simple HashMap-based storage implementation for Phase 1.
/// Not thread-safe (no Arc/RwLock) - designed for single-threaded use.
/// All storage access is encapsulated here for easy refactoring in future phases.
#[derive(Debug, Clone, Default)]
pub struct Store {
    /// Map of Ettle ID to Ettle
    pub(crate) ettles: HashMap<String, Ettle>,
    /// Map of Constraint ID to Constraint
    pub(crate) constraints: HashMap<String, Constraint>,
    /// Map of Decision ID to Decision
    pub(crate) decisions: HashMap<String, Decision>,
    /// Map of Evidence Capture ID to DecisionEvidenceItem
    pub(crate) decision_evidence_items: HashMap<String, DecisionEvidenceItem>,
    /// Map of (Decision ID, Target Kind, Target ID, Relation Kind) to DecisionLink
    pub(crate) decision_links: HashMap<(String, String, String, String), DecisionLink>,
}

impl Store {
    /// Create a new empty Store
    pub fn new() -> Self {
        Self {
            ettles: HashMap::new(),
            constraints: HashMap::new(),
            decisions: HashMap::new(),
            decision_evidence_items: HashMap::new(),
            decision_links: HashMap::new(),
        }
    }

    /// Get an Ettle by ID
    ///
    /// Returns the Ettle if found, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the ettle doesn't exist.
    pub fn get_ettle(&self, id: &str) -> Result<&Ettle> {
        self.ettles.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Ettle not found")
        })
    }

    /// Get a mutable reference to an Ettle by ID
    ///
    /// Returns the Ettle if found, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the ettle doesn't exist.
    pub fn get_ettle_mut(&mut self, id: &str) -> Result<&mut Ettle> {
        self.ettles.get_mut(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Ettle not found")
        })
    }

    /// List all Ettles
    pub fn list_ettles(&self) -> Vec<&Ettle> {
        self.ettles.values().collect()
    }

    /// Insert an Ettle into the store
    ///
    /// This is an internal method used by CRUD operations and test helpers.
    pub fn insert_ettle(&mut self, ettle: Ettle) {
        self.ettles.insert(ettle.id.clone(), ettle);
    }

    /// Check if an Ettle exists
    #[allow(dead_code)]
    pub(crate) fn ettle_exists(&self, id: &str) -> bool {
        self.ettles.contains_key(id)
    }

    /// Get a Constraint by ID
    ///
    /// Returns the Constraint if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the constraint doesn't exist,
    /// or `Deleted` if it was tombstoned.
    pub fn get_constraint(&self, id: &str) -> Result<&Constraint> {
        let constraint = self.constraints.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Constraint not found")
        })?;

        if constraint.is_deleted() {
            return Err(ExError::new(ExErrorKind::Deleted)
                .with_entity_id(id.to_string())
                .with_message("Constraint was deleted"));
        }

        Ok(constraint)
    }

    /// Get a mutable reference to a Constraint by ID
    ///
    /// Returns the Constraint if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the constraint doesn't exist,
    /// or `Deleted` if it was tombstoned.
    pub fn get_constraint_mut(&mut self, id: &str) -> Result<&mut Constraint> {
        let constraint = self.constraints.get_mut(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Constraint not found")
        })?;

        if constraint.is_deleted() {
            return Err(ExError::new(ExErrorKind::Deleted)
                .with_entity_id(id.to_string())
                .with_message("Constraint was deleted"));
        }

        Ok(constraint)
    }

    /// Insert a Constraint into the store
    ///
    /// This is an internal method used by CRUD operations.
    pub fn insert_constraint(&mut self, constraint: Constraint) {
        self.constraints
            .insert(constraint.constraint_id.clone(), constraint);
    }

    /// List all non-deleted Constraints
    pub fn list_constraints(&self) -> Vec<&Constraint> {
        self.constraints
            .values()
            .filter(|c| !c.is_deleted())
            .collect()
    }

    /// Get a Constraint by ID, including tombstoned constraints
    ///
    /// Returns the Constraint regardless of tombstone status.
    /// Used for history access (e.g., reading tombstoned constraints for audit).
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the constraint doesn't exist.
    pub fn get_constraint_including_deleted(&self, id: &str) -> Result<&Constraint> {
        self.constraints.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Constraint not found")
        })
    }

    // ===== Decision Methods =====

    /// Get a Decision by ID
    ///
    /// Returns the Decision if found and not tombstoned, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the decision doesn't exist,
    /// or `Deleted` if it was tombstoned.
    pub fn get_decision(&self, id: &str) -> Result<&Decision> {
        let decision = self.decisions.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Decision not found")
        })?;

        if decision.is_tombstoned() {
            return Err(ExError::new(ExErrorKind::Deleted)
                .with_entity_id(id.to_string())
                .with_message("Decision was deleted"));
        }

        Ok(decision)
    }

    /// Get a mutable reference to a Decision by ID
    ///
    /// Returns the Decision if found and not tombstoned, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the decision doesn't exist,
    /// or `Deleted` if it was tombstoned.
    pub fn get_decision_mut(&mut self, id: &str) -> Result<&mut Decision> {
        let decision = self.decisions.get_mut(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Decision not found")
        })?;

        if decision.is_tombstoned() {
            return Err(ExError::new(ExErrorKind::Deleted)
                .with_entity_id(id.to_string())
                .with_message("Decision was deleted"));
        }

        Ok(decision)
    }

    /// Get a Decision by ID, including tombstoned decisions.
    ///
    /// Unlike `get_decision`, this method returns the decision regardless of
    /// tombstone status. Used internally for persistence after tombstone operations.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the decision doesn't exist at all.
    pub fn get_decision_including_deleted(&self, id: &str) -> Result<&Decision> {
        self.decisions.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(id.to_string())
                .with_message("Decision not found")
        })
    }

    /// Insert a Decision into the store
    ///
    /// This is an internal method used by CRUD operations.
    pub fn insert_decision(&mut self, decision: Decision) {
        self.decisions
            .insert(decision.decision_id.clone(), decision);
    }

    /// Get a DecisionEvidenceItem by ID
    ///
    /// # Errors
    ///
    /// Returns `NotFound` if the evidence item doesn't exist.
    pub fn get_evidence_item(&self, id: &str) -> Result<&DecisionEvidenceItem> {
        self.decision_evidence_items.get(id).ok_or_else(|| {
            ExError::new(ExErrorKind::Internal)
                .with_message(format!("Evidence item not found: {}", id))
        })
    }

    /// Insert a DecisionEvidenceItem into the store
    ///
    /// This is an internal method used by decision operations.
    pub fn insert_evidence_item(&mut self, item: DecisionEvidenceItem) {
        self.decision_evidence_items
            .insert(item.evidence_capture_id.clone(), item);
    }

    /// Insert a DecisionLink into the store
    ///
    /// This is an internal method used by decision link operations.
    pub fn insert_decision_link(&mut self, link: DecisionLink) {
        let key = (
            link.decision_id.clone(),
            link.target_kind.clone(),
            link.target_id.clone(),
            link.relation_kind.clone(),
        );
        self.decision_links.insert(key, link);
    }

    /// Remove a DecisionLink from the store
    ///
    /// This is an internal method used by decision unlink operations.
    pub fn remove_decision_link(
        &mut self,
        decision_id: &str,
        target_kind: &str,
        target_id: &str,
        relation_kind: &str,
    ) {
        let key = (
            decision_id.to_string(),
            target_kind.to_string(),
            target_id.to_string(),
            relation_kind.to_string(),
        );
        self.decision_links.remove(&key);
    }

    /// Check if a decision link exists
    pub fn is_decision_linked(
        &self,
        decision_id: &str,
        target_kind: &str,
        target_id: &str,
        relation_kind: &str,
    ) -> bool {
        let key = (
            decision_id.to_string(),
            target_kind.to_string(),
            target_id.to_string(),
            relation_kind.to_string(),
        );
        self.decision_links.contains_key(&key)
    }

    /// List all DecisionLinks for a given target
    pub fn list_decision_links_for_target(
        &self,
        target_kind: &str,
        target_id: &str,
    ) -> Vec<&DecisionLink> {
        self.decision_links
            .values()
            .filter(|link| link.target_kind == target_kind && link.target_id == target_id)
            .collect()
    }

    /// Get a specific DecisionLink by its composite key
    pub fn get_decision_link(
        &self,
        decision_id: &str,
        target_kind: &str,
        target_id: &str,
        relation_kind: &str,
    ) -> Option<&DecisionLink> {
        let key = (
            decision_id.to_string(),
            target_kind.to_string(),
            target_id.to_string(),
            relation_kind.to_string(),
        );
        self.decision_links.get(&key)
    }

    /// List all non-tombstoned Decisions
    pub fn list_decisions(&self) -> Vec<&Decision> {
        self.decisions
            .values()
            .filter(|d| !d.is_tombstoned())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_store() {
        let store = Store::new();
        assert_eq!(store.list_ettles().len(), 0);
    }

    #[test]
    fn test_insert_and_get_ettle() {
        let mut store = Store::new();
        let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        store.insert_ettle(ettle.clone());

        let retrieved = store.get_ettle("ettle-1").unwrap();
        assert_eq!(retrieved.id, "ettle-1");
        assert_eq!(retrieved.title, "Test");
    }

    #[test]
    fn test_get_nonexistent_ettle() {
        let store = Store::new();
        let result = store.get_ettle("nonexistent");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
    }
}
