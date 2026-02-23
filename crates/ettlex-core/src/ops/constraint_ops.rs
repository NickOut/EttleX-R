//! Constraint operation handlers
//!
//! This module implements CRUD operations for constraints and their attachments to EPs.
//! All operations follow the TES (Temporal Event Sourcing) pattern and maintain
//! deterministic ordering for manifest generation.

use crate::errors::{EttleXError, Result};
use crate::model::{Constraint, EpConstraintRef};
use crate::ops::store::Store;
use serde_json::Value as JsonValue;

/// Create a new constraint
///
/// Creates a constraint with the specified family, kind, scope, and payload.
/// The payload digest is automatically computed for content-addressable storage.
///
/// # Arguments
///
/// * `store` - The store to add the constraint to
/// * `constraint_id` - Unique identifier for the constraint (typically UUIDv7)
/// * `family` - Constraint family (e.g., "ABB", "SBB", "Custom")
/// * `kind` - Constraint kind within family
/// * `scope` - Constraint scope (e.g., "EP", "Leaf", "Subtree")
/// * `payload_json` - Constraint configuration as JSON
///
/// # Errors
///
/// This operation is infallible for valid inputs (no constraint validation at creation time).
pub fn create_constraint(
    store: &mut Store,
    constraint_id: String,
    family: String,
    kind: String,
    scope: String,
    payload_json: JsonValue,
) -> Result<()> {
    let constraint = Constraint::new(constraint_id.clone(), family, kind, scope, payload_json);
    store.insert_constraint(constraint);
    Ok(())
}

/// Update a constraint's payload
///
/// Updates the payload of an existing constraint and recomputes the digest.
/// The constraint must exist and not be tombstoned.
///
/// # Errors
///
/// Returns `ConstraintNotFound` if the constraint doesn't exist,
/// or `ConstraintDeleted` if it was tombstoned.
pub fn update_constraint(
    store: &mut Store,
    constraint_id: &str,
    new_payload: JsonValue,
) -> Result<()> {
    let constraint = store.get_constraint_mut(constraint_id)?;
    constraint.update_payload(new_payload);
    Ok(())
}

/// Tombstone a constraint (soft delete)
///
/// Marks a constraint as deleted by setting its deleted_at timestamp.
/// The constraint remains in storage for historical snapshot references.
///
/// # Errors
///
/// Returns `ConstraintNotFound` if the constraint doesn't exist,
/// or `ConstraintDeleted` if it was already tombstoned.
pub fn tombstone_constraint(store: &mut Store, constraint_id: &str) -> Result<()> {
    let constraint = store.get_constraint_mut(constraint_id)?;
    constraint.tombstone();
    Ok(())
}

/// Attach a constraint to an EP
///
/// Creates an attachment record linking a constraint to an EP with a specific ordinal.
/// The ordinal determines the position in deterministically ordered manifests.
///
/// # Errors
///
/// Returns `ConstraintNotFound` if the constraint doesn't exist,
/// `ConstraintDeleted` if the constraint was tombstoned,
/// `EpNotFound` if the EP doesn't exist,
/// or `ConstraintAlreadyAttached` if the constraint is already attached to the EP.
pub fn attach_constraint_to_ep(
    store: &mut Store,
    ep_id: String,
    constraint_id: String,
    ordinal: i32,
) -> Result<()> {
    // Verify constraint exists and is not deleted
    let constraint = store.get_constraint(&constraint_id)?;
    if constraint.is_deleted() {
        return Err(EttleXError::ConstraintDeleted {
            constraint_id: constraint_id.clone(),
        });
    }

    // Verify EP exists and is not deleted
    store.get_ep(&ep_id)?;

    // Check if already attached
    if store.is_constraint_attached_to_ep(&ep_id, &constraint_id) {
        return Err(EttleXError::ConstraintAlreadyAttached {
            constraint_id,
            ep_id,
        });
    }

    // Create attachment record
    let ref_record = EpConstraintRef::new(ep_id, constraint_id, ordinal);
    store.insert_ep_constraint_ref(ref_record);

    Ok(())
}

/// Detach a constraint from an EP
///
/// Removes the attachment record linking a constraint to an EP.
///
/// # Errors
///
/// Returns `ConstraintNotAttached` if the constraint is not attached to the EP.
pub fn detach_constraint_from_ep(
    store: &mut Store,
    ep_id: &str,
    constraint_id: &str,
) -> Result<()> {
    if !store.is_constraint_attached_to_ep(ep_id, constraint_id) {
        return Err(EttleXError::ConstraintNotAttached {
            constraint_id: constraint_id.to_string(),
            ep_id: ep_id.to_string(),
        });
    }

    store.remove_ep_constraint_ref(ep_id, constraint_id);
    Ok(())
}

/// Get a constraint by ID
///
/// Returns a reference to the constraint if found and not deleted.
///
/// # Errors
///
/// Returns `ConstraintNotFound` if the constraint doesn't exist,
/// or `ConstraintDeleted` if it was tombstoned.
pub fn get_constraint<'a>(store: &'a Store, constraint_id: &str) -> Result<&'a Constraint> {
    store.get_constraint(constraint_id)
}

/// List all constraints attached to an EP
///
/// Returns constraints ordered by their attachment ordinal for deterministic manifest generation.
///
/// # Errors
///
/// Returns `EpNotFound` if the EP doesn't exist.
pub fn list_constraints_for_ep<'a>(store: &'a Store, ep_id: &str) -> Result<Vec<&'a Constraint>> {
    // Verify EP exists
    store.get_ep(ep_id)?;

    // Get attachment records and sort by ordinal
    let mut refs: Vec<_> = store.list_ep_constraint_refs(ep_id).into_iter().collect();
    refs.sort_by_key(|r| r.ordinal);

    // Look up constraints
    let mut constraints = Vec::new();
    for ref_record in refs {
        if let Ok(constraint) = store.get_constraint(&ref_record.constraint_id) {
            constraints.push(constraint);
        }
    }

    Ok(constraints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::ep_ops::create_ep;
    use crate::ops::ettle_ops::create_ettle;
    use serde_json::json;

    fn setup_store_with_ep() -> (Store, String, String) {
        let mut store = Store::new();

        // Create ettle with the new signature: title, metadata, why, what, how
        let ettle_id = create_ettle(
            &mut store,
            "Test Ettle".to_string(),
            None,                     // metadata
            None,                     // why
            Some("what".to_string()), // what
            Some("how".to_string()),  // how
        )
        .unwrap();

        // Create EP with the new signature: ettle_id, ordinal, normative, why, what, how
        let ep_id = create_ep(
            &mut store,
            &ettle_id,
            1,
            false, // normative
            "EP 1 Why".to_string(),
            "EP 1 What".to_string(),
            "EP 1 How".to_string(),
        )
        .unwrap();

        (store, ettle_id, ep_id)
    }

    #[test]
    fn test_create_constraint() {
        let mut store = Store::new();
        let payload = json!({"rule": "owner_must_exist"});

        let result = create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "OwnershipRule".to_string(),
            "EP".to_string(),
            payload.clone(),
        );

        assert!(result.is_ok());

        let constraint = store.get_constraint("c1").unwrap();
        assert_eq!(constraint.constraint_id, "c1");
        assert_eq!(constraint.family, "ABB");
        assert_eq!(constraint.kind, "OwnershipRule");
        assert_eq!(constraint.scope, "EP");
        assert_eq!(constraint.payload_json, payload);
    }

    #[test]
    fn test_update_constraint() {
        let mut store = Store::new();
        let payload1 = json!({"rule": "old"});
        let payload2 = json!({"rule": "new"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload1,
        )
        .unwrap();

        let result = update_constraint(&mut store, "c1", payload2.clone());
        assert!(result.is_ok());

        let constraint = store.get_constraint("c1").unwrap();
        assert_eq!(constraint.payload_json, payload2);
    }

    #[test]
    fn test_update_nonexistent_constraint() {
        let mut store = Store::new();
        let payload = json!({"rule": "test"});

        let result = update_constraint(&mut store, "nonexistent", payload);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotFound { .. })
        ));
    }

    #[test]
    fn test_tombstone_constraint() {
        let mut store = Store::new();
        let payload = json!({"rule": "test"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        )
        .unwrap();

        let result = tombstone_constraint(&mut store, "c1");
        assert!(result.is_ok());

        // Should not be able to get deleted constraint
        let result = store.get_constraint("c1");
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::ConstraintDeleted { .. })));
    }

    #[test]
    fn test_attach_constraint_to_ep() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();
        let payload = json!({"rule": "test"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        )
        .unwrap();

        let result = attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0);
        assert!(result.is_ok());

        // Verify attachment exists
        assert!(store.is_constraint_attached_to_ep(&ep_id, "c1"));
    }

    #[test]
    fn test_attach_constraint_already_attached() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();
        let payload = json!({"rule": "test"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        )
        .unwrap();

        attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0).unwrap();

        // Try to attach again
        let result = attach_constraint_to_ep(&mut store, ep_id, "c1".to_string(), 1);
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintAlreadyAttached { .. })
        ));
    }

    #[test]
    fn test_attach_deleted_constraint() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();
        let payload = json!({"rule": "test"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        )
        .unwrap();

        tombstone_constraint(&mut store, "c1").unwrap();

        let result = attach_constraint_to_ep(&mut store, ep_id, "c1".to_string(), 0);
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::ConstraintDeleted { .. })));
    }

    #[test]
    fn test_detach_constraint_from_ep() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();
        let payload = json!({"rule": "test"});

        create_constraint(
            &mut store,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        )
        .unwrap();

        attach_constraint_to_ep(&mut store, ep_id.clone(), "c1".to_string(), 0).unwrap();

        let result = detach_constraint_from_ep(&mut store, &ep_id, "c1");
        assert!(result.is_ok());

        // Verify attachment no longer exists
        assert!(!store.is_constraint_attached_to_ep(&ep_id, "c1"));
    }

    #[test]
    fn test_detach_constraint_not_attached() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

        let result = detach_constraint_from_ep(&mut store, &ep_id, "c1");
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotAttached { .. })
        ));
    }

    #[test]
    fn test_list_constraints_for_ep() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

        // Create multiple constraints
        for i in 0..3 {
            let payload = json!({"rule": format!("rule_{}", i)});
            create_constraint(
                &mut store,
                format!("c{}", i),
                "ABB".to_string(),
                "Rule".to_string(),
                "EP".to_string(),
                payload,
            )
            .unwrap();

            // Attach with reverse ordinals to test sorting
            attach_constraint_to_ep(&mut store, ep_id.clone(), format!("c{}", i), 2 - i).unwrap();
        }

        let constraints = list_constraints_for_ep(&store, &ep_id).unwrap();
        assert_eq!(constraints.len(), 3);

        // Verify ordering by ordinal
        assert_eq!(constraints[0].constraint_id, "c2"); // ordinal 0
        assert_eq!(constraints[1].constraint_id, "c1"); // ordinal 1
        assert_eq!(constraints[2].constraint_id, "c0"); // ordinal 2
    }

    #[test]
    fn test_list_constraints_for_nonexistent_ep() {
        let store = Store::new();

        let result = list_constraints_for_ep(&store, "nonexistent");
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::EpNotFound { .. })));
    }
}
