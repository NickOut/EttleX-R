mod common;

use common::new_store;
use ettlex_core::{
    ops::{ep_ops, ettle_ops},
    EttleXError,
};

// ===== CREATE EP TESTS =====

#[test]
fn test_create_ep_succeeds_with_valid_params() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test Ettle".to_string(), None, None, None, None)
            .unwrap();

    // Create EP with ordinal 1 (EP0 already exists)
    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        "Why text".to_string(),
        "What text".to_string(),
        "How text".to_string(),
    )
    .unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert_eq!(ep.ettle_id, ettle_id);
    assert_eq!(ep.ordinal, 1);
    assert!(ep.normative);
    assert_eq!(ep.why, "Why text");
    assert_eq!(ep.what, "What text");
    assert_eq!(ep.how, "How text");
    assert!(ep.is_leaf());
}

#[test]
fn test_create_ep_fails_on_nonexistent_ettle() {
    let mut store = new_store();

    let result = ep_ops::create_ep(
        &mut store,
        "nonexistent-ettle",
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    );

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EttleNotFound { .. })));
}

#[test]
fn test_create_ep_fails_on_duplicate_ordinal() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    // Try to create another EP with ordinal 0 (EP0 already exists)
    let result = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        0,
        true,
        String::new(),
        String::new(),
        String::new(),
    );

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::OrdinalAlreadyExists { .. })
    ));
}

// ===== READ EP TESTS =====

#[test]
fn test_read_ep_returns_ep() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        false,
        "Why".to_string(),
        "What".to_string(),
        "How".to_string(),
    )
    .unwrap();

    let ep = ep_ops::read_ep(&store, &ep_id).unwrap();
    assert_eq!(ep.id, ep_id);
    assert_eq!(ep.ordinal, 1);
    assert!(!ep.normative);
}

#[test]
fn test_read_ep_fails_on_nonexistent() {
    let store = new_store();
    let result = ep_ops::read_ep(&store, "nonexistent-ep");

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpNotFound { .. })));
}

#[test]
fn test_read_ep_fails_on_deleted() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    ep_ops::delete_ep(&mut store, &ep_id).unwrap();

    let result = ep_ops::read_ep(&store, &ep_id);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpDeleted { .. })));
}

// ===== UPDATE EP TESTS =====

#[test]
fn test_update_ep_updates_text_fields() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        "Original Why".to_string(),
        "Original What".to_string(),
        "Original How".to_string(),
    )
    .unwrap();

    ep_ops::update_ep(
        &mut store,
        &ep_id,
        Some("Updated Why".to_string()),
        Some("Updated What".to_string()),
        Some("Updated How".to_string()),
        None,
    )
    .unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert_eq!(ep.why, "Updated Why");
    assert_eq!(ep.what, "Updated What");
    assert_eq!(ep.how, "Updated How");
}

#[test]
fn test_update_ep_updates_normative_flag() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    ep_ops::update_ep(&mut store, &ep_id, None, None, None, Some(false)).unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert!(!ep.normative);
}

#[test]
fn test_update_ep_fails_on_ordinal_change() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // EP ordinals are immutable - we don't expose ordinal in update_ep signature
    // This test verifies that the ordinal remains unchanged
    ep_ops::update_ep(&mut store, &ep_id, None, None, None, None).unwrap();

    let ep = store.get_ep(&ep_id).unwrap();
    assert_eq!(ep.ordinal, 1); // unchanged
}

// ===== DELETE EP TESTS =====

#[test]
fn test_delete_ep_tombstones_when_no_child() {
    let mut store = new_store();
    let ettle_id =
        ettle_ops::create_ettle(&mut store, "Test".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &ettle_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    ep_ops::delete_ep(&mut store, &ep_id).unwrap();

    // Verify tombstoned
    let result = store.get_ep(&ep_id);
    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpDeleted { .. })));
}

#[test]
fn test_delete_ep_fails_when_ep_has_child() {
    let mut store = new_store();
    let parent_id =
        ettle_ops::create_ettle(&mut store, "Parent".to_string(), None, None, None, None).unwrap();
    let child_id =
        ettle_ops::create_ettle(&mut store, "Child".to_string(), None, None, None, None).unwrap();

    let ep_id = ep_ops::create_ep(
        &mut store,
        &parent_id,
        1,
        true,
        String::new(),
        String::new(),
        String::new(),
    )
    .unwrap();

    // Link child to EP (direct manipulation for now - will use refinement_ops later)
    let ep = store.get_ep_mut(&ep_id).unwrap();
    ep.child_ettle_id = Some(child_id.clone());

    let child = store.get_ettle_mut(&child_id).unwrap();
    child.parent_id = Some(parent_id.clone());

    // Try to delete EP - should fail because it's the only mapping to child
    let result = ep_ops::delete_ep(&mut store, &ep_id);

    assert!(result.is_err());
    assert!(matches!(
        result,
        Err(EttleXError::TombstoneStrandsChild { .. })
    ));
}

#[test]
fn test_delete_ep_fails_on_nonexistent() {
    let mut store = new_store();
    let result = ep_ops::delete_ep(&mut store, "nonexistent");

    assert!(result.is_err());
    assert!(matches!(result, Err(EttleXError::EpNotFound { .. })));
}
