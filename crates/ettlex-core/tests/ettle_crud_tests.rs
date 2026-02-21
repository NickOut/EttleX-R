mod common;

use common::new_store;
use ettlex_core::{ops::ettle_ops, EttleXError};

// ===== CREATE ETTLE TESTS =====

#[test]
fn test_create_ettle_fails_on_empty_title() {
    let mut store = new_store();
    let result = ettle_ops::create_ettle(&mut store, "".to_string(), None, None, None, None);

    assert!(result.is_err());
    match result {
        Err(EttleXError::InvalidTitle { reason }) => {
            assert!(reason.contains("empty") || reason.contains("blank"));
        }
        _ => panic!("Expected InvalidTitle error"),
    }
}

#[test]
fn test_create_ettle_fails_on_whitespace_only_title() {
    let mut store = new_store();
    let result =
        ettle_ops::create_ettle(&mut store, "   \t\n  ".to_string(), None, None, None, None);

    assert!(result.is_err());
    match result {
        Err(EttleXError::InvalidTitle { .. }) => {}
        _ => panic!("Expected InvalidTitle error"),
    }
}

#[test]
fn test_create_ettle_creates_ep0_and_is_readable() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .unwrap();

    // Verify Ettle was created
    let ettle = store.get_ettle(&ettle_id).unwrap();
    assert_eq!(ettle.title, "Test Ettle");
    assert!(ettle.is_root());
    assert!(!ettle.is_deleted());

    // Verify EP0 was created
    assert_eq!(ettle.ep_ids.len(), 1);
    let ep0_id = &ettle.ep_ids[0];
    let ep0 = store.get_ep(ep0_id).unwrap();
    assert_eq!(ep0.ordinal, 0);
    assert_eq!(ep0.ettle_id, ettle_id);
    assert!(ep0.normative);
}

#[test]
fn test_create_ettle_generates_unique_ids() {
    let mut store = new_store();

    let id1 =
        ettle_ops::create_ettle(&mut store, "Ettle 1".to_string(), None, None, None, None).unwrap();
    let id2 =
        ettle_ops::create_ettle(&mut store, "Ettle 2".to_string(), None, None, None, None).unwrap();

    assert_ne!(id1, id2);
}

// ===== READ ETTLE TESTS =====

#[test]
fn test_read_ettle_returns_ettle() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ettle = ettle_ops::read_ettle(&store, &ettle_id).unwrap();
    assert_eq!(ettle.id, ettle_id);
    assert_eq!(ettle.title, "Test");
}

#[test]
fn test_read_ettle_fails_on_nonexistent() {
    let store = new_store();
    let result = ettle_ops::read_ettle(&store, "nonexistent-id");

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
}

#[test]
fn test_read_ettle_fails_on_deleted() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    // Delete the ettle
    ettle_ops::delete_ettle(&mut store, &ettle_id).unwrap();

    // Try to read it
    let result = ettle_ops::read_ettle(&store, &ettle_id);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
}

// ===== UPDATE ETTLE TESTS =====

#[test]
fn test_update_ettle_title_changes_only_title_and_updated_at() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Original".to_string(), None, None, None, None)
            .unwrap();

    let original = store.get_ettle(&ettle_id).unwrap().clone();

    // Wait a tiny bit to ensure timestamp changes
    std::thread::sleep(std::time::Duration::from_millis(10));

    ettle_ops::update_ettle(&mut store, &ettle_id, Some("Updated".to_string()), None).unwrap();

    let updated = store.get_ettle(&ettle_id).unwrap();
    assert_eq!(updated.title, "Updated");
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.parent_id, original.parent_id);
    assert_eq!(updated.ep_ids, original.ep_ids);
    assert_eq!(updated.created_at, original.created_at);
    assert!(updated.updated_at > original.updated_at);
}

#[test]
fn test_update_ettle_metadata_updates_metadata() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let mut metadata = std::collections::HashMap::new();
    metadata.insert("key1".to_string(), serde_json::json!("value1"));

    ettle_ops::update_ettle(&mut store, &ettle_id, None, Some(metadata.into())).unwrap();

    let updated = store.get_ettle(&ettle_id).unwrap();
    assert_eq!(
        updated.metadata.get("key1"),
        Some(&serde_json::json!("value1"))
    );
}

#[test]
fn test_update_ettle_fails_on_empty_title() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let result = ettle_ops::update_ettle(&mut store, &ettle_id, Some("".to_string()), None);

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::InvalidTitle { .. })));
}

// ===== DELETE ETTLE TESTS =====

#[test]
fn test_delete_ettle_tombstones_when_no_children() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    ettle_ops::delete_ettle(&mut store, &ettle_id).unwrap();

    // Verify it's tombstoned (can't read, but still exists in store)
    let result = store.get_ettle(&ettle_id);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));

    // Verify it's not in list_ettles
    let ettles = store.list_ettles();
    assert_eq!(ettles.len(), 0);
}

#[test]
fn test_delete_ettle_fails_when_ettle_has_children() {
    let mut store = new_store();

    // Create parent and child
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    // Link them (using direct manipulation for now - will use refinement_ops later)
    let parent = store.get_ettle_mut(&parent_id).unwrap();
    let ep1_id = parent.ep_ids[0].clone();

    let ep1 = store.get_ep_mut(&ep1_id).unwrap();
    ep1.child_ettle_id = Some(child_id.clone());

    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());

    // Try to delete parent
    let result = ettle_ops::delete_ettle(&mut store, &parent_id);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::DeleteWithChildren { .. })
    ));
}

#[test]
fn test_delete_ettle_fails_on_nonexistent() {
    let mut store = new_store();
    let result = ettle_ops::delete_ettle(&mut store, "nonexistent");

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
}

#[test]
fn test_delete_ettle_fails_on_already_deleted() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    ettle_ops::delete_ettle(&mut store, &ettle_id).unwrap();

    // Try to delete again
    let result = ettle_ops::delete_ettle(&mut store, &ettle_id);

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleDeleted { .. })));
}
