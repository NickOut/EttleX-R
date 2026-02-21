/// Scenario 1: Create Ettle With Metadata
///
/// Tests creating an Ettle with optional metadata according to Phase 0.5 spec.
/// Covers happy path and error cases for metadata validation.
use ettlex_core::model::Metadata;
use ettlex_core::ops::{ettle_ops, Store};

#[test]
fn test_scenario_01_happy_create_ettle_with_valid_metadata() {
    // GIVEN an empty store
    let mut store = Store::new();

    // AND valid metadata
    let mut metadata = Metadata::new();
    metadata.set("category".to_string(), "Architecture".into());
    metadata.set("priority".to_string(), 1.into());
    metadata.set("tags".to_string(), vec!["core", "api"].into());

    // WHEN creating an Ettle with metadata
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "System Design".to_string(),
        Some(metadata.clone()),
        None,
        None,
        None,
    )
    .expect("Should create Ettle with metadata");

    // THEN the Ettle exists with the metadata
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    assert_eq!(ettle.title, "System Design");
    assert_eq!(ettle.metadata.len(), 3);
    assert_eq!(ettle.metadata.get("category"), Some(&"Architecture".into()));
    assert_eq!(ettle.metadata.get("priority"), Some(&1.into()));

    // AND EP0 exists and is linked
    assert_eq!(ettle.ep_ids.len(), 1);
    let ep0 = store.get_ep(&ettle.ep_ids[0]).expect("EP0 should exist");
    assert_eq!(ep0.ordinal, 0);
    assert_eq!(ep0.ettle_id, ettle_id);
}

#[test]
fn test_scenario_01_create_ettle_without_metadata() {
    // GIVEN an empty store
    let mut store = Store::new();

    // WHEN creating an Ettle without metadata
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Simple Ettle".to_string(),
        None,
        None,
        None,
        None,
    )
    .expect("Should create Ettle without metadata");

    // THEN the Ettle exists with empty metadata
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    assert_eq!(ettle.title, "Simple Ettle");
    assert!(ettle.metadata.is_empty());
}

// NOTE: Per architectural decisions, invalid metadata types and duplicate titles
// are deferred to Phase 1. Current implementation accepts any JSON-serializable
// metadata and allows duplicate titles.

#[test]
fn test_scenario_01_metadata_accepts_any_json_value() {
    // GIVEN an empty store
    let mut store = Store::new();

    // AND metadata with various JSON types
    let mut metadata = Metadata::new();
    metadata.set("string".to_string(), "value".into());
    metadata.set("number".to_string(), 42.into());
    metadata.set("boolean".to_string(), true.into());
    metadata.set("null".to_string(), serde_json::Value::Null);
    metadata.set("array".to_string(), vec![1, 2, 3].into());

    // WHEN creating an Ettle with complex metadata
    let ettle_id = ettle_ops::create_ettle(
        &mut store,
        "Complex Metadata".to_string(),
        Some(metadata.clone()),
        None,
        None,
        None,
    )
    .expect("Should accept any JSON-serializable metadata");

    // THEN the metadata is stored correctly
    let ettle = store.get_ettle(&ettle_id).expect("Ettle should exist");
    assert_eq!(ettle.metadata.len(), 5);
}

#[test]
fn test_scenario_01_duplicate_titles_are_allowed() {
    // GIVEN an empty store with an existing Ettle
    let mut store = Store::new();
    let _first_id = ettle_ops::create_ettle(
        &mut store,
        "Duplicate Title".to_string(),
        None,
        None,
        None,
        None,
    )
    .expect("Should create first Ettle");

    // WHEN creating another Ettle with the same title
    let second_id = ettle_ops::create_ettle(
        &mut store,
        "Duplicate Title".to_string(),
        None,
        None,
        None,
        None,
    )
    .expect("Should allow duplicate titles");

    // THEN both Ettles exist
    let second_ettle = store
        .get_ettle(&second_id)
        .expect("Second Ettle should exist");
    assert_eq!(second_ettle.title, "Duplicate Title");

    // AND there are 2 Ettles in the store
    assert_eq!(store.list_ettles().len(), 2);
}
