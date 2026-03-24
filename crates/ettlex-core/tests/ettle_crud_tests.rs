mod common;

use common::new_store;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::ops::ettle_ops;

// ===== CREATE ETTLE TESTS =====

#[test]
fn test_create_ettle_fails_on_empty_title() {
    let mut store = new_store();
    let result = ettle_ops::create_ettle(&mut store, "".to_string());

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ExErrorKind::InvalidTitle,
        "Expected InvalidTitle error"
    );
}

#[test]
fn test_create_ettle_fails_on_whitespace_only_title() {
    let mut store = new_store();
    let result = ettle_ops::create_ettle(&mut store, "   \t\n  ".to_string());

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err().kind(),
        ExErrorKind::InvalidTitle,
        "Expected InvalidTitle error"
    );
}

#[test]
fn test_create_ettle_is_readable() {
    let mut store = new_store();
    let ettle_id = ettle_ops::create_ettle(&mut store, "Test Ettle".to_string()).unwrap();

    // Verify Ettle was created and is readable
    let ettle = store.get_ettle(&ettle_id).unwrap();
    assert_eq!(ettle.title, "Test Ettle");
    assert_eq!(ettle.id, ettle_id);
}

#[test]
fn test_create_ettle_generates_unique_ids() {
    let mut store = new_store();

    let id1 = ettle_ops::create_ettle(&mut store, "Ettle 1".to_string()).unwrap();
    let id2 = ettle_ops::create_ettle(&mut store, "Ettle 2".to_string()).unwrap();

    assert_ne!(id1, id2);
}

// ===== READ ETTLE TESTS =====

#[test]
fn test_read_ettle_returns_ettle() {
    let mut store = new_store();
    let ettle_id = ettle_ops::create_ettle(&mut store, "Test".to_string()).unwrap();

    let ettle = ettle_ops::read_ettle(&store, &ettle_id).unwrap();
    assert_eq!(ettle.id, ettle_id);
    assert_eq!(ettle.title, "Test");
}

#[test]
fn test_read_ettle_fails_on_nonexistent() {
    let store = new_store();
    let result = ettle_ops::read_ettle(&store, "nonexistent-id");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ===== DELETE ETTLE TESTS =====

#[test]
fn test_delete_ettle_fails_on_nonexistent() {
    let mut store = new_store();
    let result = ettle_ops::delete_ettle(&mut store, "nonexistent");

    assert!(result.is_err());
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}
