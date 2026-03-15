use crate::errors::{ExError, ExErrorKind, Result};
use crate::ops::Store;

use super::invariants;

/// Validate the entire refinement tree
///
/// Runs all invariant checks and returns an error if any violations are found.
/// This implements the comprehensive validation contract from Phase 0.5 spec:
///
/// 1. All referenced Ettles/EPs exist (or tombstoned)
/// 2. Bidirectional membership consistency (EP.ettle_id ↔ Ettle.ep_ids)
/// 3. Active EP projection determinism (via active_eps() use in invariants)
/// 4. Parent chain integrity (no cycles, no orphans)
/// 5. No multiple parents (implicit via parent_id being Option<String>)
/// 6. Refinement mapping integrity (children have EP mappings, no duplicates)
/// 7. Deletion safety (deleted EPs don't have mappings, EPs don't map to deleted children)
///
/// Deleted Ettles and EPs are filtered from active checks but still validated for consistency.
///
/// # Arguments
/// * `store` - Reference to the Store to validate
///
/// # Errors
/// Returns the first validation error encountered. For exhaustive error
/// reporting, call the individual invariant functions directly.
pub fn validate_tree(store: &Store) -> Result<()> {
    // Requirement 1 & 2: Check for unknown EP refs and bidirectional membership
    let unknown_ep_refs = invariants::find_unknown_ep_refs(store);
    if let Some((ettle_id, ep_id)) = unknown_ep_refs.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_entity_id(ettle_id.clone())
            .with_message(format!("ep_ids contains unknown EP ID: {}", ep_id.clone())));
    }

    let eps_with_unknown_ettle = invariants::find_eps_with_unknown_ettle(store);
    if let Some((ep_id, ettle_id)) = eps_with_unknown_ettle.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_ep_id(ep_id.clone())
            .with_message(format!(
                "EP has ettle_id pointing to unknown ettle: {}",
                ettle_id.clone()
            )));
    }

    let membership_inconsistencies = invariants::find_membership_inconsistencies(store);
    if let Some((ep_id, ep_ettle_id, owner_ettle_id)) = membership_inconsistencies.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_ep_id(ep_id.clone())
            .with_message(format!(
                "EP has ettle_id={} but is owned by ettle {}",
                ep_ettle_id.clone(),
                owner_ettle_id.clone()
            )));
    }

    let ep_orphans = invariants::find_ep_orphans(store);
    if let Some((ep_id, ettle_id)) = ep_orphans.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_ep_id(ep_id.clone())
            .with_message(format!(
                "EP points to ettle {} but is not listed in its ep_ids",
                ettle_id.clone()
            )));
    }

    // Requirement 4: Parent chain integrity
    for ettle in store.list_ettles() {
        if invariants::has_cycle(store, &ettle.id) {
            return Err(ExError::new(ExErrorKind::CycleDetected)
                .with_entity_id(ettle.id.clone())
                .with_message("Setting parent would create a cycle"));
        }
    }

    let orphans = invariants::find_orphans(store);
    if let Some((child_id, parent_id)) = orphans.first() {
        return Err(ExError::new(ExErrorKind::NotFound)
            .with_entity_id(child_id.clone())
            .with_message(format!("Parent {} does not exist", parent_id.clone())));
    }

    // Requirement 6: Refinement mapping integrity (uses active_eps internally)
    let missing_mappings = invariants::find_children_without_ep_mapping(store);
    if let Some((child_id, parent_id)) = missing_mappings.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_entity_id(child_id.clone())
            .with_message(format!(
                "Child has no EP mapping from parent {}",
                parent_id.clone()
            )));
    }

    let duplicate_ordinals = invariants::find_duplicate_ordinals(store);
    if let Some((ettle_id, ordinal)) = duplicate_ordinals.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_entity_id(ettle_id.clone())
            .with_ordinal(*ordinal)
            .with_message("Duplicate EP ordinal"));
    }

    let duplicate_mappings = invariants::find_duplicate_child_mappings(store);
    if let Some((child_id, ep_ids)) = duplicate_mappings.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_entity_id(child_id.clone())
            .with_message(format!("Referenced by multiple EPs: {:?}", ep_ids.clone(),)));
    }

    let invalid_refs = invariants::find_eps_with_nonexistent_children(store);
    if let Some((ep_id, child_id)) = invalid_refs.first() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_ep_id(ep_id.clone())
            .with_message(format!(
                "References non-existent child: {}",
                child_id.clone()
            )));
    }

    // Requirement 7: Deletion safety constraints
    let deleted_ep_mappings = invariants::find_deleted_ep_mappings(store);
    if let Some(ep_id) = deleted_ep_mappings.first() {
        // This is a data integrity issue - deleted EPs shouldn't have mappings
        return Err(ExError::new(ExErrorKind::Deleted)
            .with_ep_id(ep_id.clone())
            .with_message("EP has child mapping but EP is deleted"));
    }

    let deleted_child_mappings = invariants::find_deleted_child_mappings(store);
    if let Some((ep_id, child_id)) = deleted_child_mappings.first() {
        return Err(ExError::new(ExErrorKind::Deleted)
            .with_ep_id(ep_id.clone())
            .with_message(format!(
                "EP maps to deleted child ettle {}",
                child_id.clone()
            )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ep, Ettle};

    #[test]
    fn test_validate_tree_empty_store() {
        let store = Store::new();
        assert!(validate_tree(&store).is_ok());
    }

    #[test]
    fn test_validate_tree_single_root() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("root".to_string(), "Root".to_string());
        ettle.add_ep_id("ep0".to_string()); // Add bidirectional membership

        let ep0 = Ep::new(
            "ep0".to_string(),
            "root".to_string(),
            0,
            true,
            String::new(),
            String::new(),
            String::new(),
        );

        store.insert_ep(ep0);
        store.insert_ettle(ettle);

        assert!(validate_tree(&store).is_ok());
    }

    #[test]
    fn test_validate_tree_detects_cycle() {
        let mut store = Store::new();

        let mut a = Ettle::new("a".to_string(), "A".to_string());
        let mut b = Ettle::new("b".to_string(), "B".to_string());

        a.parent_id = Some("b".to_string());
        b.parent_id = Some("a".to_string());

        store.insert_ettle(a);
        store.insert_ettle(b);

        let result = validate_tree(&store);
        assert!(result.is_err());
        // Should detect cycle
    }
}
