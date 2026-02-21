use std::collections::{HashMap, HashSet};

use crate::ops::{active_eps, Store};

/// Check if an Ettle is part of a cycle
///
/// Uses DFS to detect cycles in the parent chain.
pub fn has_cycle(store: &Store, ettle_id: &str) -> bool {
    let mut visited = HashSet::new();
    let mut current = Some(ettle_id);

    while let Some(id) = current {
        if !visited.insert(id) {
            // We've seen this node before - cycle detected
            return true;
        }

        // Move to parent
        if let Ok(ettle) = store.get_ettle(id) {
            current = ettle.parent_id.as_deref();
        } else {
            // Ettle not found or deleted - stop traversal
            break;
        }
    }

    false
}

/// Find Ettles that have a parent_id but the parent doesn't exist
///
/// Returns list of (child_id, parent_id) tuples
pub fn find_orphans(store: &Store) -> Vec<(String, String)> {
    let mut orphans = Vec::new();

    for ettle in store.list_ettles() {
        if let Some(ref parent_id) = ettle.parent_id {
            // Check if parent exists
            if store.get_ettle(parent_id).is_err() {
                orphans.push((ettle.id.clone(), parent_id.clone()));
            }
        }
    }

    orphans
}

/// Find children that have a parent_id but no corresponding EP mapping
///
/// Only checks active (non-deleted) EPs for mappings.
///
/// Returns list of (child_id, parent_id) tuples
pub fn find_children_without_ep_mapping(store: &Store) -> Vec<(String, String)> {
    let mut missing_mappings = Vec::new();

    for ettle in store.list_ettles() {
        if let Some(ref parent_id) = ettle.parent_id {
            // Check if parent exists
            let parent = match store.get_ettle(parent_id) {
                Ok(p) => p,
                Err(_) => continue, // Parent doesn't exist - handled by find_orphans
            };

            // Check if any of parent's ACTIVE EPs map to this child
            let active = match active_eps(store, parent) {
                Ok(eps) => eps,
                Err(_) => continue, // Skip if membership inconsistency (handled by other invariants)
            };

            let has_mapping = active
                .iter()
                .any(|ep| ep.child_ettle_id.as_deref() == Some(&ettle.id));

            if !has_mapping {
                missing_mappings.push((ettle.id.clone(), parent_id.clone()));
            }
        }
    }

    missing_mappings
}

/// Find duplicate EP ordinals within each Ettle
///
/// Only checks active (non-deleted) EPs.
///
/// Returns list of (ettle_id, ordinal) tuples for duplicates
pub fn find_duplicate_ordinals(store: &Store) -> Vec<(String, u32)> {
    let mut duplicates = Vec::new();

    for ettle in store.list_ettles() {
        // Get active EPs only
        let active = match active_eps(store, ettle) {
            Ok(eps) => eps,
            Err(_) => continue, // Skip if membership inconsistency (handled by other invariants)
        };

        let mut ordinal_counts: HashMap<u32, usize> = HashMap::new();

        for ep in active {
            *ordinal_counts.entry(ep.ordinal).or_insert(0) += 1;
        }

        for (ordinal, count) in ordinal_counts {
            if count > 1 {
                duplicates.push((ettle.id.clone(), ordinal));
            }
        }
    }

    duplicates
}

/// Find children that are mapped by multiple EPs (violation of one-to-one mapping)
///
/// Only checks active (non-deleted) EPs for mappings.
///
/// Returns list of (child_id, vec![ep_ids]) tuples
pub fn find_duplicate_child_mappings(store: &Store) -> Vec<(String, Vec<String>)> {
    let mut child_to_eps: HashMap<String, Vec<String>> = HashMap::new();

    // Collect all active EP -> child mappings
    for ettle in store.list_ettles() {
        let active = match active_eps(store, ettle) {
            Ok(eps) => eps,
            Err(_) => continue, // Skip if membership inconsistency
        };

        for ep in active {
            if let Some(ref child_id) = ep.child_ettle_id {
                child_to_eps
                    .entry(child_id.clone())
                    .or_default()
                    .push(ep.id.clone());
            }
        }
    }

    // Find children mapped by multiple EPs
    child_to_eps
        .into_iter()
        .filter(|(_, ep_ids)| ep_ids.len() > 1)
        .collect()
}

/// Find EPs that reference non-existent children
///
/// Only reports truly missing children, not deleted ones (deleted children
/// are reported by find_deleted_child_mappings).
///
/// Returns list of (ep_id, child_id) tuples
pub fn find_eps_with_nonexistent_children(store: &Store) -> Vec<(String, String)> {
    let mut invalid_refs = Vec::new();

    for ep in store.list_eps() {
        if let Some(ref child_id) = ep.child_ettle_id {
            // Check if child exists in raw store (not get_ettle which excludes deleted)
            if !store.ettles.contains_key(child_id) {
                invalid_refs.push((ep.id.clone(), child_id.clone()));
            }
        }
    }

    invalid_refs
}

/// Find bidirectional membership inconsistencies (EP.ettle_id ≠ owner Ettle)
///
/// Validates that each EP listed in an Ettle's ep_ids has the correct ettle_id
/// pointing back to the owner.
///
/// Returns list of (ep_id, ep_ettle_id, owner_ettle_id) tuples
pub fn find_membership_inconsistencies(store: &Store) -> Vec<(String, String, String)> {
    let mut inconsistencies = Vec::new();

    for ettle in store.list_ettles() {
        for ep_id in &ettle.ep_ids {
            // Check EP exists (include tombstoned EPs)
            if let Some(ep) = store.eps.get(ep_id) {
                if ep.ettle_id != ettle.id {
                    inconsistencies.push((ep.id.clone(), ep.ettle_id.clone(), ettle.id.clone()));
                }
            }
        }
    }

    inconsistencies
}

/// Find EP orphans (EP.ettle_id points to Ettle but Ettle.ep_ids doesn't include EP)
///
/// Validates that each EP's ettle_id points to an Ettle that actually lists it.
///
/// Returns list of (ep_id, ettle_id) tuples
pub fn find_ep_orphans(store: &Store) -> Vec<(String, String)> {
    let mut orphans = Vec::new();

    // Check all EPs (including tombstoned)
    for ep in &store.eps {
        // Check if the ettle_id exists
        if let Some(ettle) = store.ettles.get(&ep.1.ettle_id) {
            // Check if the Ettle's ep_ids contains this EP
            if !ettle.ep_ids.contains(&ep.1.id) {
                orphans.push((ep.1.id.clone(), ep.1.ettle_id.clone()));
            }
        }
    }

    orphans
}

/// Find Ettle.ep_ids entries that reference unknown EP IDs
///
/// Returns list of (ettle_id, unknown_ep_id) tuples
pub fn find_unknown_ep_refs(store: &Store) -> Vec<(String, String)> {
    let mut unknown_refs = Vec::new();

    for ettle in store.list_ettles() {
        for ep_id in &ettle.ep_ids {
            // Check if EP exists in store (even tombstoned EPs should exist)
            if !store.eps.contains_key(ep_id) {
                unknown_refs.push((ettle.id.clone(), ep_id.clone()));
            }
        }
    }

    unknown_refs
}

/// Find EPs whose ettle_id references a missing Ettle
///
/// Returns list of (ep_id, ettle_id) tuples
pub fn find_eps_with_unknown_ettle(store: &Store) -> Vec<(String, String)> {
    let mut unknown_ettles = Vec::new();

    // Check all EPs (including tombstoned)
    for ep in &store.eps {
        // Check if the ettle_id exists in store
        if !store.ettles.contains_key(&ep.1.ettle_id) {
            unknown_ettles.push((ep.1.id.clone(), ep.1.ettle_id.clone()));
        }
    }

    unknown_ettles
}

/// Find deleted (tombstoned) EPs that still have child mappings
///
/// Returns list of ep_id strings
pub fn find_deleted_ep_mappings(store: &Store) -> Vec<String> {
    let mut deleted_with_mappings = Vec::new();

    // Check all EPs (including tombstoned)
    for ep in &store.eps {
        if ep.1.deleted && ep.1.child_ettle_id.is_some() {
            deleted_with_mappings.push(ep.1.id.clone());
        }
    }

    deleted_with_mappings
}

/// Find EPs that map to deleted (tombstoned) child Ettles
///
/// Returns list of (ep_id, child_id) tuples
pub fn find_deleted_child_mappings(store: &Store) -> Vec<(String, String)> {
    let mut deleted_child_mappings = Vec::new();

    for ep in store.list_eps() {
        if let Some(ref child_id) = ep.child_ettle_id {
            // Check if child exists and is deleted
            if let Some(child) = store.ettles.get(child_id) {
                if child.deleted {
                    deleted_child_mappings.push((ep.id.clone(), child_id.clone()));
                }
            }
        }
    }

    deleted_child_mappings
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ep, Ettle};

    #[test]
    fn test_has_cycle_detects_self_cycle() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("a".to_string(), "A".to_string());
        ettle.parent_id = Some("a".to_string()); // Self-cycle

        store.insert_ettle(ettle);

        assert!(has_cycle(&store, "a"));
    }

    #[test]
    fn test_has_cycle_detects_two_node_cycle() {
        let mut store = Store::new();

        let mut a = Ettle::new("a".to_string(), "A".to_string());
        let mut b = Ettle::new("b".to_string(), "B".to_string());

        a.parent_id = Some("b".to_string());
        b.parent_id = Some("a".to_string());

        store.insert_ettle(a);
        store.insert_ettle(b);

        assert!(has_cycle(&store, "a"));
        assert!(has_cycle(&store, "b"));
    }

    #[test]
    fn test_has_cycle_returns_false_for_valid_chain() {
        let mut store = Store::new();

        let a = Ettle::new("a".to_string(), "A".to_string());
        let mut b = Ettle::new("b".to_string(), "B".to_string());
        let mut c = Ettle::new("c".to_string(), "C".to_string());

        b.parent_id = Some("a".to_string());
        c.parent_id = Some("b".to_string());

        store.insert_ettle(a);
        store.insert_ettle(b);
        store.insert_ettle(c);

        assert!(!has_cycle(&store, "a"));
        assert!(!has_cycle(&store, "b"));
        assert!(!has_cycle(&store, "c"));
    }

    #[test]
    fn test_find_membership_inconsistencies() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        // Create EP with wrong ettle_id
        let ep = Ep::new(
            "ep-1".to_string(),
            "different-ettle".to_string(), // Wrong!
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-1".to_string());

        store.insert_ettle(ettle);
        store.insert_ep(ep);

        let inconsistencies = find_membership_inconsistencies(&store);
        assert_eq!(inconsistencies.len(), 1);
        assert_eq!(inconsistencies[0].0, "ep-1");
        assert_eq!(inconsistencies[0].1, "different-ettle");
        assert_eq!(inconsistencies[0].2, "ettle-1");
    }

    #[test]
    fn test_find_ep_orphans() {
        let mut store = Store::new();
        let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        // Create EP that points to ettle but is NOT listed in ep_ids
        let ep = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );

        store.insert_ettle(ettle); // Don't add ep-1 to ep_ids
        store.insert_ep(ep);

        let orphans = find_ep_orphans(&store);
        assert_eq!(orphans.len(), 1);
        assert_eq!(orphans[0].0, "ep-1");
        assert_eq!(orphans[0].1, "ettle-1");
    }

    #[test]
    fn test_find_unknown_ep_refs() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
        ettle.add_ep_id("nonexistent-ep".to_string()); // EP doesn't exist!

        store.insert_ettle(ettle);

        let unknown = find_unknown_ep_refs(&store);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].0, "ettle-1");
        assert_eq!(unknown[0].1, "nonexistent-ep");
    }

    #[test]
    fn test_find_eps_with_unknown_ettle() {
        let mut store = Store::new();

        // Create EP pointing to non-existent Ettle
        let ep = Ep::new(
            "ep-1".to_string(),
            "nonexistent-ettle".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );

        store.insert_ep(ep);

        let unknown = find_eps_with_unknown_ettle(&store);
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].0, "ep-1");
        assert_eq!(unknown[0].1, "nonexistent-ettle");
    }

    #[test]
    fn test_find_deleted_ep_mappings() {
        let mut store = Store::new();

        // Create deleted EP with child mapping
        let mut ep = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ep.child_ettle_id = Some("child-1".to_string());
        ep.deleted = true;

        store.insert_ep(ep);

        let deleted_mappings = find_deleted_ep_mappings(&store);
        assert_eq!(deleted_mappings.len(), 1);
        assert_eq!(deleted_mappings[0], "ep-1");
    }

    #[test]
    fn test_find_deleted_child_mappings() {
        let mut store = Store::new();

        // Create child Ettle that's deleted
        let mut child = Ettle::new("child-1".to_string(), "Child".to_string());
        child.deleted = true;

        // Create EP mapping to deleted child
        let mut ep = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ep.child_ettle_id = Some("child-1".to_string());

        store.insert_ettle(child);
        store.insert_ep(ep);

        let deleted_child_mappings = find_deleted_child_mappings(&store);
        assert_eq!(deleted_child_mappings.len(), 1);
        assert_eq!(deleted_child_mappings[0].0, "ep-1");
        assert_eq!(deleted_child_mappings[0].1, "child-1");
    }
}
