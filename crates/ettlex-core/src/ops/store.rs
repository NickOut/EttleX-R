use std::collections::HashMap;

use crate::errors::{EttleXError, Result};
use crate::model::{
    Constraint, Decision, DecisionEvidenceItem, DecisionLink, Ep, EpConstraintRef, Ettle,
};

/// In-memory store for Ettles, EPs, and Constraints
///
/// This is a simple HashMap-based storage implementation for Phase 1.
/// Not thread-safe (no Arc/RwLock) - designed for single-threaded use.
/// All storage access is encapsulated here for easy refactoring in future phases.
#[derive(Debug, Clone, Default)]
pub struct Store {
    /// Map of Ettle ID to Ettle
    pub(crate) ettles: HashMap<String, Ettle>,
    /// Map of EP ID to EP
    pub(crate) eps: HashMap<String, Ep>,
    /// Map of Constraint ID to Constraint
    pub(crate) constraints: HashMap<String, Constraint>,
    /// Map of (EP ID, Constraint ID) to EP-Constraint attachment record
    pub(crate) ep_constraint_refs: HashMap<(String, String), EpConstraintRef>,
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
            eps: HashMap::new(),
            constraints: HashMap::new(),
            ep_constraint_refs: HashMap::new(),
            decisions: HashMap::new(),
            decision_evidence_items: HashMap::new(),
            decision_links: HashMap::new(),
        }
    }

    /// Get an Ettle by ID
    ///
    /// Returns the Ettle if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `EttleNotFound` if the ettle doesn't exist, or `EttleDeleted` if it was tombstoned.
    pub fn get_ettle(&self, id: &str) -> Result<&Ettle> {
        let ettle = self
            .ettles
            .get(id)
            .ok_or_else(|| EttleXError::EttleNotFound {
                ettle_id: id.to_string(),
            })?;

        if ettle.deleted {
            return Err(EttleXError::EttleDeleted {
                ettle_id: id.to_string(),
            });
        }

        Ok(ettle)
    }

    /// Get a mutable reference to an Ettle by ID
    ///
    /// Returns the Ettle if found and not deleted, otherwise returns an error.
    /// This is a public method to enable test helpers.
    ///
    /// # Errors
    ///
    /// Returns `EttleNotFound` if the ettle doesn't exist, or `EttleDeleted` if it was tombstoned.
    pub fn get_ettle_mut(&mut self, id: &str) -> Result<&mut Ettle> {
        let ettle = self
            .ettles
            .get_mut(id)
            .ok_or_else(|| EttleXError::EttleNotFound {
                ettle_id: id.to_string(),
            })?;

        if ettle.deleted {
            return Err(EttleXError::EttleDeleted {
                ettle_id: id.to_string(),
            });
        }

        Ok(ettle)
    }

    /// Get an EP by ID
    ///
    /// Returns the EP if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `EpNotFound` if the EP doesn't exist, or `EpDeleted` if it was tombstoned.
    pub fn get_ep(&self, id: &str) -> Result<&Ep> {
        let ep = self.eps.get(id).ok_or_else(|| EttleXError::EpNotFound {
            ep_id: id.to_string(),
        })?;

        if ep.deleted {
            return Err(EttleXError::EpDeleted {
                ep_id: id.to_string(),
            });
        }

        Ok(ep)
    }

    /// Get a mutable reference to an EP by ID
    ///
    /// Returns the EP if found and not deleted, otherwise returns an error.
    /// This is a public method to enable test helpers.
    ///
    /// # Errors
    ///
    /// Returns `EpNotFound` if the EP doesn't exist, or `EpDeleted` if it was tombstoned.
    pub fn get_ep_mut(&mut self, id: &str) -> Result<&mut Ep> {
        let ep = self
            .eps
            .get_mut(id)
            .ok_or_else(|| EttleXError::EpNotFound {
                ep_id: id.to_string(),
            })?;

        if ep.deleted {
            return Err(EttleXError::EpDeleted {
                ep_id: id.to_string(),
            });
        }

        Ok(ep)
    }

    /// List all non-deleted Ettles
    pub fn list_ettles(&self) -> Vec<&Ettle> {
        self.ettles.values().filter(|e| !e.deleted).collect()
    }

    /// List all non-deleted EPs
    pub fn list_eps(&self) -> Vec<&Ep> {
        self.eps.values().filter(|ep| !ep.deleted).collect()
    }

    /// Insert an Ettle into the store
    ///
    /// This is an internal method used by CRUD operations and test helpers.
    pub fn insert_ettle(&mut self, ettle: Ettle) {
        self.ettles.insert(ettle.id.clone(), ettle);
    }

    /// Insert an EP into the store
    ///
    /// This is an internal method used by CRUD operations and test helpers.
    pub fn insert_ep(&mut self, ep: Ep) {
        self.eps.insert(ep.id.clone(), ep);
    }

    /// Check if an Ettle exists (ignoring deleted flag)
    #[allow(dead_code)]
    pub(crate) fn ettle_exists(&self, id: &str) -> bool {
        self.ettles.contains_key(id)
    }

    /// Check if an EP exists (ignoring deleted flag)
    #[allow(dead_code)]
    pub(crate) fn ep_exists(&self, id: &str) -> bool {
        self.eps.contains_key(id)
    }

    /// Check if an EP exists in storage (including deleted EPs)
    ///
    /// This is useful for testing hard delete vs tombstone behavior.
    pub fn ep_exists_in_storage(&self, id: &str) -> bool {
        self.eps.contains_key(id)
    }

    /// Get an EP from storage, bypassing deleted check
    ///
    /// This is useful for testing tombstone behavior.
    /// Returns None if EP doesn't exist, Some(EP) if it exists (even if deleted).
    pub fn get_ep_raw(&self, id: &str) -> Option<&Ep> {
        self.eps.get(id)
    }

    /// Get a Constraint by ID
    ///
    /// Returns the Constraint if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `ConstraintNotFound` if the constraint doesn't exist,
    /// or `ConstraintDeleted` if it was tombstoned.
    pub fn get_constraint(&self, id: &str) -> Result<&Constraint> {
        let constraint =
            self.constraints
                .get(id)
                .ok_or_else(|| EttleXError::ConstraintNotFound {
                    constraint_id: id.to_string(),
                })?;

        if constraint.is_deleted() {
            return Err(EttleXError::ConstraintDeleted {
                constraint_id: id.to_string(),
            });
        }

        Ok(constraint)
    }

    /// Get a mutable reference to a Constraint by ID
    ///
    /// Returns the Constraint if found and not deleted, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `ConstraintNotFound` if the constraint doesn't exist,
    /// or `ConstraintDeleted` if it was tombstoned.
    pub fn get_constraint_mut(&mut self, id: &str) -> Result<&mut Constraint> {
        let constraint =
            self.constraints
                .get_mut(id)
                .ok_or_else(|| EttleXError::ConstraintNotFound {
                    constraint_id: id.to_string(),
                })?;

        if constraint.is_deleted() {
            return Err(EttleXError::ConstraintDeleted {
                constraint_id: id.to_string(),
            });
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

    /// Insert an EP-Constraint attachment record
    ///
    /// This is an internal method used by constraint attachment operations.
    pub fn insert_ep_constraint_ref(&mut self, ref_record: EpConstraintRef) {
        let key = (ref_record.ep_id.clone(), ref_record.constraint_id.clone());
        self.ep_constraint_refs.insert(key, ref_record);
    }

    /// Remove an EP-Constraint attachment record
    ///
    /// This is an internal method used by constraint detachment operations.
    pub fn remove_ep_constraint_ref(&mut self, ep_id: &str, constraint_id: &str) {
        let key = (ep_id.to_string(), constraint_id.to_string());
        self.ep_constraint_refs.remove(&key);
    }

    /// Check if a constraint is attached to an EP
    pub fn is_constraint_attached_to_ep(&self, ep_id: &str, constraint_id: &str) -> bool {
        let key = (ep_id.to_string(), constraint_id.to_string());
        self.ep_constraint_refs.contains_key(&key)
    }

    /// List all EP-Constraint attachment records for a given EP
    pub fn list_ep_constraint_refs(&self, ep_id: &str) -> Vec<&EpConstraintRef> {
        self.ep_constraint_refs
            .values()
            .filter(|r| r.ep_id == ep_id)
            .collect()
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
    /// Returns `ConstraintNotFound` if the constraint doesn't exist.
    pub fn get_constraint_including_deleted(&self, id: &str) -> Result<&Constraint> {
        self.constraints
            .get(id)
            .ok_or_else(|| EttleXError::ConstraintNotFound {
                constraint_id: id.to_string(),
            })
    }

    // ===== Decision Methods =====

    /// Get a Decision by ID
    ///
    /// Returns the Decision if found and not tombstoned, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `DecisionNotFound` if the decision doesn't exist,
    /// or `DecisionDeleted` if it was tombstoned.
    pub fn get_decision(&self, id: &str) -> Result<&Decision> {
        let decision = self
            .decisions
            .get(id)
            .ok_or_else(|| EttleXError::DecisionNotFound {
                decision_id: id.to_string(),
            })?;

        if decision.is_tombstoned() {
            return Err(EttleXError::DecisionDeleted {
                decision_id: id.to_string(),
            });
        }

        Ok(decision)
    }

    /// Get a mutable reference to a Decision by ID
    ///
    /// Returns the Decision if found and not tombstoned, otherwise returns an error.
    ///
    /// # Errors
    ///
    /// Returns `DecisionNotFound` if the decision doesn't exist,
    /// or `DecisionDeleted` if it was tombstoned.
    pub fn get_decision_mut(&mut self, id: &str) -> Result<&mut Decision> {
        let decision = self
            .decisions
            .get_mut(id)
            .ok_or_else(|| EttleXError::DecisionNotFound {
                decision_id: id.to_string(),
            })?;

        if decision.is_tombstoned() {
            return Err(EttleXError::DecisionDeleted {
                decision_id: id.to_string(),
            });
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
    /// Returns `DecisionNotFound` if the decision doesn't exist at all.
    pub fn get_decision_including_deleted(&self, id: &str) -> Result<&Decision> {
        self.decisions
            .get(id)
            .ok_or_else(|| EttleXError::DecisionNotFound {
                decision_id: id.to_string(),
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
        self.decision_evidence_items
            .get(id)
            .ok_or_else(|| EttleXError::Internal {
                message: format!("Evidence item not found: {}", id),
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
        assert_eq!(store.list_eps().len(), 0);
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
        assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
    }

    #[test]
    fn test_get_deleted_ettle() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
        ettle.deleted = true;

        store.insert_ettle(ettle);

        let result = store.get_ettle("ettle-1");
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
    }
}
