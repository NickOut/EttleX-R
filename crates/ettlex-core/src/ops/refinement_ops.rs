use chrono::Utc;
use std::collections::HashSet;

use super::{active_eps, store::Store};
use crate::errors::{EttleXError, Result};

/// Set the parent of an Ettle
///
/// This operation updates the child's `parent_id` field. It performs cycle
/// detection to ensure the tree remains acyclic.
///
/// To make an Ettle a root (no parent), pass `None` for parent_id.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `child_id` - ID of the Ettle whose parent to set
/// * `parent_id` - Optional ID of the new parent (None = make root)
///
/// # Errors
/// * `EttleNotFound` - If child or parent doesn't exist
/// * `EttleDeleted` - If child or parent was deleted
/// * `CycleDetected` - If setting this parent would create a cycle
pub fn set_parent(store: &mut Store, child_id: &str, parent_id: Option<&str>) -> Result<()> {
    // Verify child exists
    store.get_ettle(child_id)?;

    // If setting parent (not making root), verify parent exists and check for cycles
    if let Some(pid) = parent_id {
        store.get_ettle(pid).map_err(|e| match e {
            EttleXError::EttleNotFound { ettle_id } => EttleXError::ParentNotFound { ettle_id },
            other => other,
        })?;

        // Check for direct cycle (child == parent)
        if child_id == pid {
            return Err(EttleXError::CycleDetected {
                ettle_id: child_id.to_string(),
            });
        }

        // Check for indirect cycle (parent is ancestor of child)
        if would_create_cycle(store, child_id, pid)? {
            return Err(EttleXError::CycleDetected {
                ettle_id: child_id.to_string(),
            });
        }
    }

    // Update child's parent_id
    let child = store.get_ettle_mut(child_id)?;
    child.parent_id = parent_id.map(|s| s.to_string());
    child.updated_at = Utc::now();

    Ok(())
}

/// Link a child Ettle to an EP
///
/// This creates a one-to-one mapping between an EP and a child Ettle.
/// Both the EP's `child_ettle_id` and the child's `parent_id` are updated.
///
/// Refinement constraints (R4):
/// - EP must not be deleted (enforced by get_ep)
/// - Child Ettle must not be deleted (enforced by get_ettle)
/// - EP must be in parent's active EP set
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `ep_id` - ID of the EP to link from
/// * `child_id` - ID of the child Ettle to link to
///
/// # Errors
/// * `EpNotFound` - If EP doesn't exist
/// * `EttleNotFound` - If child Ettle doesn't exist
/// * `EpDeleted` - If EP was deleted
/// * `EttleDeleted` - If child was deleted
/// * `ChildAlreadyHasParent` - If child already has a parent
/// * `EpAlreadyHasChild` - If EP already maps to a different child
pub fn link_child(store: &mut Store, ep_id: &str, child_id: &str) -> Result<()> {
    // Verify EP exists and is not deleted (R4: EP must not be deleted)
    let ep = store.get_ep(ep_id)?;

    // Verify child exists and is not deleted (R4: child must not be deleted)
    let child = store.get_ettle(child_id)?;

    // Get parent_id from EP
    let parent_id = ep.ettle_id.clone();

    // Verify EP is in parent's active set (R4: EP must be active)
    let parent = store.get_ettle(&parent_id)?;
    let active = active_eps(store, parent)?;
    if !active.iter().any(|e| e.id == ep_id) {
        return Err(EttleXError::EpDeleted {
            ep_id: ep_id.to_string(),
        });
    }

    // Check if EP already has a child
    if let Some(ref existing_child_id) = ep.child_ettle_id {
        return Err(EttleXError::EpAlreadyHasChild {
            ep_id: ep_id.to_string(),
            current_child_id: existing_child_id.clone(),
        });
    }

    // Check if child already has a parent
    if let Some(ref existing_parent_id) = child.parent_id {
        return Err(EttleXError::ChildAlreadyHasParent {
            child_id: child_id.to_string(),
            ep_id: ep_id.to_string(),
            current_parent_id: existing_parent_id.clone(),
        });
    }

    // Update EP's child_ettle_id
    let ep = store.get_ep_mut(ep_id)?;
    ep.child_ettle_id = Some(child_id.to_string());
    ep.updated_at = Utc::now();

    // Update child's parent_id
    let child = store.get_ettle_mut(child_id)?;
    child.parent_id = Some(parent_id);
    child.updated_at = Utc::now();

    Ok(())
}

/// Unlink a child Ettle from an EP
///
/// This removes the one-to-one mapping by clearing the EP's `child_ettle_id`
/// and the child's `parent_id`.
///
/// If the EP has no child, this is a no-op.
///
/// # Arguments
/// * `store` - Mutable reference to the Store
/// * `ep_id` - ID of the EP to unlink from
///
/// # Errors
/// * `EpNotFound` - If EP doesn't exist
/// * `EpDeleted` - If EP was deleted
pub fn unlink_child(store: &mut Store, ep_id: &str) -> Result<()> {
    // Get EP
    let ep = store.get_ep(ep_id)?;

    // If EP has no child, this is a no-op
    let child_id = match &ep.child_ettle_id {
        Some(id) => id.clone(),
        None => return Ok(()), // No-op
    };

    // Clear EP's child_ettle_id
    let ep = store.get_ep_mut(ep_id)?;
    ep.child_ettle_id = None;
    ep.updated_at = Utc::now();

    // Clear child's parent_id (if child still exists and is not deleted)
    if let Ok(child) = store.get_ettle_mut(&child_id) {
        child.parent_id = None;
        child.updated_at = Utc::now();
    }

    Ok(())
}

/// List children of an Ettle in ordinal order
///
/// Returns the IDs of all child Ettles linked via this Ettle's active EPs,
/// sorted by EP ordinal (ascending). Only active (non-deleted) EPs are considered.
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `ettle_id` - ID of the parent Ettle
///
/// # Returns
/// Vector of child Ettle IDs in ordinal order
///
/// # Errors
/// * `EttleNotFound` - If Ettle doesn't exist
/// * `EttleDeleted` - If Ettle was deleted
pub fn list_children(store: &Store, ettle_id: &str) -> Result<Vec<String>> {
    let ettle = store.get_ettle(ettle_id)?;

    // Get active EPs only
    let active = active_eps(store, ettle)?;

    // Collect child_ids from active EPs (already sorted by ordinal)
    let child_ids: Vec<String> = active
        .iter()
        .filter_map(|ep| ep.child_ettle_id.clone())
        .collect();

    Ok(child_ids)
}

/// Check if setting parent_id on child would create a cycle
///
/// This performs a DFS from the proposed parent upward, checking if we reach
/// the child (which would mean child is an ancestor of parent).
fn would_create_cycle(store: &Store, child_id: &str, parent_id: &str) -> Result<bool> {
    let mut visited = HashSet::new();
    let mut current = Some(parent_id);

    while let Some(id) = current {
        // If we've visited this node, we have a cycle (not from this operation)
        if !visited.insert(id) {
            return Ok(false); // Existing cycle, but not caused by this operation
        }

        // If we reach the child, we would create a cycle
        if id == child_id {
            return Ok(true);
        }

        // Move up to parent
        let ettle = store.get_ettle(id)?;
        current = ettle.parent_id.as_deref();
    }

    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ettle;

    #[test]
    fn test_set_parent_updates_parent_id() {
        let mut store = Store::new();

        let parent = Ettle::new("parent".to_string(), "Parent".to_string());
        let child = Ettle::new("child".to_string(), "Child".to_string());

        store.insert_ettle(parent);
        store.insert_ettle(child);

        set_parent(&mut store, "child", Some("parent")).unwrap();

        let child = store.get_ettle("child").unwrap();
        assert_eq!(child.parent_id, Some("parent".to_string()));
    }

    #[test]
    fn test_would_create_cycle_detects_cycle() {
        let mut store = Store::new();

        let a = Ettle::new("a".to_string(), "A".to_string());
        let mut b = Ettle::new("b".to_string(), "B".to_string());
        let mut c = Ettle::new("c".to_string(), "C".to_string());

        b.parent_id = Some("a".to_string());
        c.parent_id = Some("b".to_string());

        store.insert_ettle(a);
        store.insert_ettle(b);
        store.insert_ettle(c);

        // Check if A -> C would create cycle (yes, C is ancestor of A via B)
        assert!(would_create_cycle(&store, "a", "c").unwrap());

        // Check if C -> A would not create cycle (A is ancestor of C, so this is OK direction)
        assert!(!would_create_cycle(&store, "c", "a").unwrap());
    }
}
