use crate::errors::{EttleXError, Result};
use crate::model::{Ep, Ettle};
use crate::ops::Store;

/// Returns the active (non-deleted) EPs for a given Ettle, sorted by ordinal ascending.
///
/// This function implements the "active EP projection" concept (R3 requirement):
/// - Filters out tombstoned EPs (deleted == true)
/// - Returns EPs in deterministic order (sorted by ordinal)
/// - Validates bidirectional membership consistency
///
/// # Arguments
/// * `store` - The Store containing all Ettles and EPs
/// * `ettle` - The Ettle whose active EPs should be returned
///
/// # Returns
/// A Vec of EP references sorted by ordinal (ascending), or an error if:
/// - An EP in ettle.ep_ids doesn't exist in the store
/// - An EP's ettle_id doesn't match the owning Ettle's ID (membership inconsistency)
///
/// # Errors
/// - `EpListContainsUnknownId` - ettle.ep_ids contains an EP ID not in the store
/// - `MembershipInconsistent` - EP.ettle_id doesn't match owning Ettle.id
///
/// # Example
/// ```rust,ignore
/// let active = active_eps(&store, &ettle)?;
/// for ep in active {
///     println!("EP{}: {}", ep.ordinal, ep.what);
/// }
/// ```
pub fn active_eps<'a>(store: &'a Store, ettle: &Ettle) -> Result<Vec<&'a Ep>> {
    let mut eps = Vec::new();

    // Collect all EPs listed in ettle.ep_ids
    for ep_id in &ettle.ep_ids {
        // Check if EP exists in store (allow tombstoned EPs)
        let ep = store
            .eps
            .get(ep_id)
            .ok_or_else(|| EttleXError::EpListContainsUnknownId {
                ettle_id: ettle.id.clone(),
                ep_id: ep_id.clone(),
            })?;

        // Validate bidirectional membership consistency (R1 requirement)
        if ep.ettle_id != ettle.id {
            return Err(EttleXError::MembershipInconsistent {
                ep_id: ep.id.clone(),
                ep_ettle_id: ep.ettle_id.clone(),
                owner_ettle_id: ettle.id.clone(),
            });
        }

        // Only include non-deleted EPs in the active set
        if !ep.deleted {
            eps.push(ep);
        }
    }

    // Sort by ordinal ascending for deterministic ordering (R3 requirement)
    eps.sort_by_key(|ep| ep.ordinal);

    Ok(eps)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ep;

    #[test]
    fn test_active_eps_empty() {
        let store = Store::new();
        let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        let result = active_eps(&store, &ettle).unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_active_eps_filters_deleted() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        // Create EP0 (active)
        let ep0 = Ep::new(
            "ep-0".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-0".to_string());
        store.insert_ep(ep0);

        // Create EP1 (deleted)
        let mut ep1 = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            1,
            false,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ep1.deleted = true;
        ettle.add_ep_id("ep-1".to_string());
        store.insert_ep(ep1);

        // Create EP2 (active)
        let ep2 = Ep::new(
            "ep-2".to_string(),
            "ettle-1".to_string(),
            2,
            false,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-2".to_string());
        store.insert_ep(ep2);

        let result = active_eps(&store, &ettle).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "ep-0");
        assert_eq!(result[1].id, "ep-2");
    }

    #[test]
    fn test_active_eps_sorted_by_ordinal() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        // Add EPs in non-ordinal order to ep_ids
        let ep2 = Ep::new(
            "ep-2".to_string(),
            "ettle-1".to_string(),
            2,
            false,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-2".to_string());
        store.insert_ep(ep2);

        let ep0 = Ep::new(
            "ep-0".to_string(),
            "ettle-1".to_string(),
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-0".to_string());
        store.insert_ep(ep0);

        let ep1 = Ep::new(
            "ep-1".to_string(),
            "ettle-1".to_string(),
            1,
            false,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-1".to_string());
        store.insert_ep(ep1);

        let result = active_eps(&store, &ettle).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].ordinal, 0);
        assert_eq!(result[1].ordinal, 1);
        assert_eq!(result[2].ordinal, 2);
    }

    #[test]
    fn test_active_eps_unknown_ep_id() {
        let store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
        ettle.add_ep_id("nonexistent-ep".to_string());

        let result = active_eps(&store, &ettle);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::EpListContainsUnknownId { .. })
        ));
    }

    #[test]
    fn test_active_eps_membership_inconsistent() {
        let mut store = Store::new();
        let mut ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());

        // Create EP that points to different ettle
        let ep = Ep::new(
            "ep-0".to_string(),
            "different-ettle".to_string(), // Wrong ettle_id!
            0,
            true,
            "Why".to_string(),
            "What".to_string(),
            "How".to_string(),
        );
        ettle.add_ep_id("ep-0".to_string());
        store.insert_ep(ep);

        let result = active_eps(&store, &ettle);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::MembershipInconsistent { .. })
        ));
    }
}
