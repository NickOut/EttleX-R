//! Refinement Traversal (RT) — EP-era parent_id links retired in Slice 03.
//!
//! Without parent_id, the RT always returns just the leaf node itself.

use crate::errors::Result;
use crate::ops::Store;

/// Compute Refinement Traversal — returns just the leaf node (no parent links).
///
/// # Errors
/// Returns `NotFound` if no Ettle with `leaf_id` exists in the store.
pub fn compute_rt(store: &Store, leaf_id: &str) -> Result<Vec<String>> {
    store.get_ettle(leaf_id)?;
    Ok(vec![leaf_id.to_string()])
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
}
