//! Constraint operation handlers
//!
//! This module implements CRUD operations for constraints.
//! EP-attachment operations have been retired in Slice 03.

use crate::errors::{ExError, ExErrorKind, Result};
use crate::model::Constraint;
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
/// Returns `InvalidConstraintFamily` if family is empty.
/// Returns `AlreadyExists` if a constraint with the same ID already exists.
pub fn create_constraint(
    store: &mut Store,
    constraint_id: String,
    family: String,
    kind: String,
    scope: String,
    payload_json: JsonValue,
) -> Result<()> {
    if family.is_empty() {
        return Err(ExError::new(ExErrorKind::InvalidConstraintFamily)
            .with_entity_id(constraint_id.clone())
            .with_message("Constraint family is invalid"));
    }

    if store.constraints.contains_key(&constraint_id) {
        return Err(ExError::new(ExErrorKind::AlreadyExists)
            .with_entity_id(constraint_id.clone())
            .with_message("Constraint already exists"));
    }

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
/// Returns `NotFound` if the constraint doesn't exist,
/// or `Deleted` if it was tombstoned.
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
/// Returns `NotFound` if the constraint doesn't exist,
/// or `Deleted` if it was already tombstoned.
pub fn tombstone_constraint(store: &mut Store, constraint_id: &str) -> Result<()> {
    let constraint = store.get_constraint_mut(constraint_id)?;
    constraint.tombstone();
    Ok(())
}

/// Get a constraint by ID
///
/// Returns a reference to the constraint if found and not deleted.
///
/// # Errors
///
/// Returns `NotFound` if the constraint doesn't exist,
/// or `Deleted` if it was tombstoned.
pub fn get_constraint<'a>(store: &'a Store, constraint_id: &str) -> Result<&'a Constraint> {
    store.get_constraint(constraint_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
            Err(e) if e.kind() == ExErrorKind::NotFound
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
        assert!(result.is_err() && result.as_ref().unwrap_err().kind() == ExErrorKind::Deleted);
    }
}
