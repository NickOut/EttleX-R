use crate::errors::{EttleXError, Result};
use crate::ops::Store;

/// Compute Refinement Traversal (RT) from root to leaf
///
/// RT is the sequence of Ettle IDs from the root of the tree down to the
/// specified leaf Ettle, following parent pointers upward and then reversing
/// to get root-to-leaf order.
///
/// # Arguments
/// * `store` - Reference to the Store
/// * `leaf_id` - ID of the leaf Ettle to compute RT for
///
/// # Returns
/// Vector of Ettle IDs in root-to-leaf order
///
/// # Errors
/// * `EttleNotFound` - If leaf doesn't exist
/// * `EttleDeleted` - If leaf was deleted
/// * `RtParentChainBroken` - If parent_id points to nonexistent Ettle
pub fn compute_rt(store: &Store, leaf_id: &str) -> Result<Vec<String>> {
    // Verify leaf exists
    store.get_ettle(leaf_id)?;

    // Follow parent pointers upward to build leaf-to-root path
    let mut path = Vec::new();
    let mut current = Some(leaf_id);

    while let Some(id) = current {
        path.push(id.to_string());

        // Get current Ettle
        let ettle = store
            .get_ettle(id)
            .map_err(|_| EttleXError::RtParentChainBroken {
                ettle_id: id.to_string(),
            })?;

        // Move to parent
        current = ettle.parent_id.as_deref();
    }

    // Reverse to get root-to-leaf order
    path.reverse();

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Ettle;

    #[test]
    fn test_compute_rt_single_ettle() {
        let mut store = Store::new();
        let ettle = Ettle::new("root".to_string(), "Root".to_string());
        store.insert_ettle(ettle);

        let rt = compute_rt(&store, "root").unwrap();

        assert_eq!(rt.len(), 1);
        assert_eq!(rt[0], "root");
    }

    #[test]
    fn test_compute_rt_chain() {
        let mut store = Store::new();

        let a = Ettle::new("a".to_string(), "A".to_string());
        let mut b = Ettle::new("b".to_string(), "B".to_string());
        let mut c = Ettle::new("c".to_string(), "C".to_string());

        b.parent_id = Some("a".to_string());
        c.parent_id = Some("b".to_string());

        store.insert_ettle(a);
        store.insert_ettle(b);
        store.insert_ettle(c);

        let rt = compute_rt(&store, "c").unwrap();

        assert_eq!(rt, vec!["a", "b", "c"]);
    }
}
