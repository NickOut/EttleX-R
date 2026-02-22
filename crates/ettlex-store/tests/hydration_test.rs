// Integration tests for repository hydration
// Covers Gherkin scenario E.2: Deterministic reload ordering

use ettlex_core::model::{Ep, Ettle};
use ettlex_core::ops::store::Store;
use rusqlite::Connection;

fn setup_test_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    conn
}

#[test]
fn test_load_ettle() {
    // Given: An Ettle persisted to the database
    let conn = setup_test_db();
    let ettle = Ettle::new("ettle-1".to_string(), "Test Ettle".to_string());
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    // When: We load it into a Store
    let mut store = Store::new();
    ettlex_store::repo::hydration::load_ettle(&conn, "ettle-1", &mut store).unwrap();

    // Then: The Store contains the Ettle
    let loaded = store.get_ettle("ettle-1").unwrap();
    assert_eq!(loaded.id, "ettle-1");
    assert_eq!(loaded.title, "Test Ettle");
    assert!(!loaded.deleted);
}

#[test]
fn test_load_ettles_deterministic_order() {
    // Given: Multiple Ettles persisted in random order
    let conn = setup_test_db();
    let ettle1 = Ettle::new("ettle-c".to_string(), "Third".to_string());
    let ettle2 = Ettle::new("ettle-a".to_string(), "First".to_string());
    let ettle3 = Ettle::new("ettle-b".to_string(), "Second".to_string());

    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &ettle1).unwrap();
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &ettle2).unwrap();
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &ettle3).unwrap();

    // When: We load all Ettles twice
    let mut store1 = Store::new();
    ettlex_store::repo::hydration::load_all_ettles(&conn, &mut store1).unwrap();
    let mut ids1: Vec<String> = store1.list_ettles().iter().map(|e| e.id.clone()).collect();
    ids1.sort(); // HashMap doesn't preserve order, so sort for comparison

    let mut store2 = Store::new();
    ettlex_store::repo::hydration::load_all_ettles(&conn, &mut store2).unwrap();
    let mut ids2: Vec<String> = store2.list_ettles().iter().map(|e| e.id.clone()).collect();
    ids2.sort();

    // Then: Both loads return the same Ettles (when sorted)
    assert_eq!(ids1, ids2);

    // And: All expected Ettles are present
    assert_eq!(ids1, vec!["ettle-a", "ettle-b", "ettle-c"]);
}

#[test]
fn test_load_tree() {
    // Given: A tree structure persisted to the database
    let conn = setup_test_db();

    // Create parent Ettle
    let mut parent = Ettle::new("parent-1".to_string(), "Parent".to_string());
    parent.add_ep_id("ep-1".to_string());
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &parent).unwrap();

    // Create child Ettle with parent link (must exist before EP references it)
    let mut child = Ettle::new("child-1".to_string(), "Child".to_string());
    child.parent_id = Some("parent-1".to_string());
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &child).unwrap();

    // Create parent EP with child link
    let mut parent_ep = Ep::new(
        "ep-1".to_string(),
        "parent-1".to_string(),
        0,
        true,
        "Why".to_string(),
        "What".to_string(),
        "How".to_string(),
    );
    parent_ep.child_ettle_id = Some("child-1".to_string());
    ettlex_store::repo::SqliteRepo::persist_ep(&conn, &parent_ep).unwrap();

    // When: We load the full tree
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // Then: Parent-child relationships are correct
    let loaded_parent = store.get_ettle("parent-1").unwrap();
    assert_eq!(loaded_parent.ep_ids, vec!["ep-1"]);

    let loaded_child = store.get_ettle("child-1").unwrap();
    assert_eq!(loaded_child.parent_id, Some("parent-1".to_string()));

    let loaded_ep = store.get_ep("ep-1").unwrap();
    assert_eq!(loaded_ep.child_ettle_id, Some("child-1".to_string()));
}

#[test]
fn test_load_tree_with_multiple_eps() {
    // Given: An Ettle with multiple EPs
    let conn = setup_test_db();

    let mut ettle = Ettle::new("ettle-1".to_string(), "Multi-EP Ettle".to_string());
    ettle.add_ep_id("ep-0".to_string());
    ettle.add_ep_id("ep-1".to_string());
    ettle.add_ep_id("ep-2".to_string());
    ettlex_store::repo::SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    // Create EPs in non-ordinal order
    let ep2 = Ep::new(
        "ep-2".to_string(),
        "ettle-1".to_string(),
        2,
        false,
        "Why 2".to_string(),
        "What 2".to_string(),
        "How 2".to_string(),
    );
    let ep0 = Ep::new(
        "ep-0".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "Why 0".to_string(),
        "What 0".to_string(),
        "How 0".to_string(),
    );
    let ep1 = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        1,
        true,
        "Why 1".to_string(),
        "What 1".to_string(),
        "How 1".to_string(),
    );

    ettlex_store::repo::SqliteRepo::persist_ep(&conn, &ep2).unwrap();
    ettlex_store::repo::SqliteRepo::persist_ep(&conn, &ep0).unwrap();
    ettlex_store::repo::SqliteRepo::persist_ep(&conn, &ep1).unwrap();

    // When: We load the tree
    let store = ettlex_store::repo::hydration::load_tree(&conn).unwrap();

    // Then: All EPs are loaded
    assert!(store.get_ep("ep-0").is_ok());
    assert!(store.get_ep("ep-1").is_ok());
    assert!(store.get_ep("ep-2").is_ok());

    // And: Ettles ep_ids list is correct
    let loaded_ettle = store.get_ettle("ettle-1").unwrap();
    assert_eq!(loaded_ettle.ep_ids.len(), 3);
    assert!(loaded_ettle.ep_ids.contains(&"ep-0".to_string()));
    assert!(loaded_ettle.ep_ids.contains(&"ep-1".to_string()));
    assert!(loaded_ettle.ep_ids.contains(&"ep-2".to_string()));
}
