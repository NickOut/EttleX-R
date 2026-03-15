use super::rt::compute_rt;
use crate::errors::{ExError, ExErrorKind, Result};
use crate::ops::{active_eps, Store};

/// Compute EP Traversal (EPT) from root to leaf
///
/// EPT is the sequence of EP IDs that form a complete refinement path from
/// the root EP0 down to a leaf EP. It walks the RT, collecting:
/// 1. EP0 from each Ettle in the RT (except the leaf)
/// 2. The EP from each parent that maps to the next child in the RT,
///    determined via `child.parent_ep_id` (the authoritative join field)
/// 3. The specified leaf EP (or EP0 if leaf has only one EP)
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `leaf_id` - ID of the leaf Ettle
/// * `leaf_ep_ordinal` - Optional ordinal of the leaf EP to end with.
///   If None and leaf has only one EP, uses that EP.
///   If None and leaf has multiple EPs, returns EptAmbiguousLeafEp error.
///
/// # Returns
/// Vector of EP IDs in root-to-leaf order
///
/// # Errors
/// * `EttleNotFound` - If leaf doesn't exist
/// * `EptMissingMapping` - If a child Ettle has no `parent_ep_id` set
/// * `EptAmbiguousLeafEp` - If leaf has multiple EPs and ordinal not specified
/// * `EptLeafEpNotFound` - If specified leaf EP ordinal doesn't exist
pub fn compute_ept(
    store: &Store,
    leaf_id: &str,
    leaf_ep_ordinal: Option<u32>,
) -> Result<Vec<String>> {
    // Compute RT first
    let rt = compute_rt(store, leaf_id)?;

    if rt.is_empty() {
        return Ok(Vec::new());
    }

    let mut ept = Vec::new();

    // Step 1: Add root EP0
    let root_id = &rt[0];
    let root = store.get_ettle(root_id)?;

    let active = active_eps(store, root)?;
    let root_ep0 = active.iter().find(|ep| ep.ordinal == 0).ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_entity_id(root_id.clone())
            .with_ordinal(0)
            .with_message("Leaf EP with ordinal not found")
    })?;

    ept.push(root_ep0.id.clone());

    // Step 2: For each transition (parent -> child), add the mapping EP.
    // The child's parent_ep_id is the authoritative join field — look it up directly.
    for i in 0..rt.len() - 1 {
        let current_id = &rt[i];
        let next_child_id = &rt[i + 1];

        let next_child = store.get_ettle(next_child_id)?;
        let mapping_ep_id = next_child.parent_ep_id.as_ref().ok_or_else(|| {
            ExError::new(ExErrorKind::MissingMapping)
                .with_entity_id(next_child_id.clone())
                .with_message(format!(
                    "No EP in parent {} maps to child",
                    current_id.clone()
                ))
        })?;

        ept.push(mapping_ep_id.clone());
    }

    // Step 3: Handle leaf EP
    let leaf_ettle_id = rt.last().ok_or_else(|| {
        ExError::new(ExErrorKind::Internal).with_message("RT should never be empty")
    })?;
    let leaf_ettle = store.get_ettle(leaf_ettle_id)?;

    // Count active EPs in leaf
    let leaf_active = active_eps(store, leaf_ettle)?;
    let leaf_ep_count = leaf_active.len();

    if let Some(ord) = leaf_ep_ordinal {
        // Ordinal specified - add it if not already present
        let leaf_ep_id = find_specific_ep(store, leaf_ettle_id, ord)?;

        if !ept.contains(&leaf_ep_id) {
            ept.push(leaf_ep_id);
        }
    } else {
        // No ordinal specified - check if leaf has multiple EPs
        if leaf_ep_count > 1 {
            return Err(ExError::new(ExErrorKind::AmbiguousLeafSelection)
                .with_entity_id(leaf_ettle_id.clone())
                .with_message("Leaf ettle has multiple EPs, must specify ordinal"));
        }
        // Leaf has exactly 1 EP - add it if not already present
        if leaf_ep_count == 1 {
            let leaf_ep = &leaf_active[0];
            if !ept.contains(&leaf_ep.id) {
                ept.push(leaf_ep.id.clone());
            }
        }
    }

    Ok(ept)
}

/// Find EP with specific ordinal in an Ettle
fn find_specific_ep(store: &Store, ettle_id: &str, ordinal: u32) -> Result<String> {
    let ettle = store.get_ettle(ettle_id)?;
    let active = active_eps(store, ettle)?;

    active
        .iter()
        .find(|ep| ep.ordinal == ordinal)
        .map(|ep| ep.id.clone())
        .ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_entity_id(ettle_id)
                .with_ordinal(ordinal)
                .with_message("Leaf EP with ordinal not found")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Ep, Ettle};

    #[test]
    fn test_compute_ept_single_ettle() {
        let mut store = Store::new();

        let mut ettle = Ettle::new("root".to_string(), "Root".to_string());
        let ep0 = Ep::new(
            "ep0".to_string(),
            "root".to_string(),
            0,
            true,
            String::new(),
            String::new(),
            String::new(),
        );

        ettle.add_ep_id("ep0".to_string());
        store.insert_ettle(ettle);
        store.insert_ep(ep0);

        let ept = compute_ept(&store, "root", None).unwrap();

        assert_eq!(ept.len(), 1);
        assert_eq!(ept[0], "ep0");
    }
}
