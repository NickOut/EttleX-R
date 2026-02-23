//! Integration tests for constraint persistence
//!
//! Tests scenarios 1-3 from seed_constraint_schema_stubs_v9.yaml:
//! 1. Arbitrary family support
//! 2. Stable ordering
//! 3. Historical preservation (tombstoning)

use ettlex_core::model::{Constraint, Ep, EpConstraintRef, Ettle};
use ettlex_store::repo::{hydration, SqliteRepo};
use rusqlite::Connection;
use serde_json::json;

fn setup_test_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    conn
}

#[test]
fn test_persist_and_get_constraint_roundtrip() {
    let conn = setup_test_db();

    let payload = json!({"rule": "owner_must_exist", "severity": "error"});
    let constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "OwnershipRule".to_string(),
        "EP".to_string(),
        payload.clone(),
    );

    // Persist
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Get
    let retrieved = SqliteRepo::get_constraint(&conn, "c1")
        .unwrap()
        .expect("Constraint should exist");

    assert_eq!(retrieved.constraint_id, "c1");
    assert_eq!(retrieved.family, "ABB");
    assert_eq!(retrieved.kind, "OwnershipRule");
    assert_eq!(retrieved.scope, "EP");
    assert_eq!(retrieved.payload_json, payload);
    assert!(!retrieved.is_deleted());
}

#[test]
fn test_upsert_constraint_behavior() {
    let conn = setup_test_db();

    let payload1 = json!({"rule": "old"});
    let mut constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        payload1,
    );

    // First persist
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Update and persist again
    let payload2 = json!({"rule": "new"});
    constraint.update_payload(payload2.clone());
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Should have updated, not inserted duplicate
    let retrieved = SqliteRepo::get_constraint(&conn, "c1")
        .unwrap()
        .expect("Constraint should exist");

    assert_eq!(retrieved.payload_json, payload2);
}

#[test]
fn test_constraint_tombstoning_preserves_history() {
    let conn = setup_test_db();

    let payload = json!({"rule": "test"});
    let mut constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        payload,
    );

    // Persist active constraint
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Tombstone it
    constraint.tombstone();
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Should still exist in storage with deleted_at set
    let retrieved = SqliteRepo::get_constraint(&conn, "c1")
        .unwrap()
        .expect("Tombstoned constraint should still exist in DB");

    assert!(retrieved.is_deleted());
    assert!(retrieved.deleted_at.is_some());
}

#[test]
fn test_ep_constraint_ref_persistence() {
    let conn = setup_test_db();

    // Create parent ettle and EP first (FK requirements)
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    // Create constraint
    let payload = json!({"rule": "test"});
    let constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        payload,
    );
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Create attachment record
    let ref_record = EpConstraintRef::new("ep-1".to_string(), "c1".to_string(), 0);
    SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record).unwrap();

    // Retrieve
    let refs = SqliteRepo::list_ep_constraint_refs(&conn, "ep-1").unwrap();

    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].ep_id, "ep-1");
    assert_eq!(refs[0].constraint_id, "c1");
    assert_eq!(refs[0].ordinal, 0);
}

// Scenario 1: Arbitrary Family Support
#[test]
fn test_scenario_1_arbitrary_family_support() {
    let conn = setup_test_db();

    // Create constraints with various families (not limited to ABB/SBB)
    let families = ["ABB", "SBB", "Custom", "ZArchitect", "ArchiMate"];

    for (i, family) in families.iter().enumerate() {
        let payload = json!({"family_specific": format!("config_{}", i)});
        let constraint = Constraint::new(
            format!("c{}", i),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            payload,
        );
        SqliteRepo::persist_constraint(&conn, &constraint).unwrap();
    }

    // Verify all families were stored
    let all_constraints = SqliteRepo::list_constraints(&conn).unwrap();
    assert_eq!(all_constraints.len(), 5);

    // Check each family
    for (i, family) in families.iter().enumerate() {
        let constraint = SqliteRepo::get_constraint(&conn, &format!("c{}", i))
            .unwrap()
            .expect("Constraint should exist");
        assert_eq!(constraint.family, *family);
    }
}

// Scenario 2: Stable Ordering
#[test]
fn test_scenario_2_stable_ordinal_ordering() {
    let conn = setup_test_db();

    // Set up ettle and EP
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    // Create multiple constraints with explicit ordinals
    let ordinals = vec![2, 0, 1, 3]; // Insert in non-sequential order

    for ordinal in &ordinals {
        let constraint = Constraint::new(
            format!("c{}", ordinal),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"ordinal": ordinal}),
        );
        SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

        let ref_record =
            EpConstraintRef::new("ep-1".to_string(), format!("c{}", ordinal), *ordinal);
        SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record).unwrap();
    }

    // Retrieve in deterministic order
    let refs = SqliteRepo::list_ep_constraint_refs(&conn, "ep-1").unwrap();

    // Should be ordered by ordinal (0, 1, 2, 3)
    assert_eq!(refs.len(), 4);
    assert_eq!(refs[0].ordinal, 0);
    assert_eq!(refs[1].ordinal, 1);
    assert_eq!(refs[2].ordinal, 2);
    assert_eq!(refs[3].ordinal, 3);
}

// Scenario 3: Historical Preservation
#[test]
fn test_scenario_3_historical_preservation() {
    let conn = setup_test_db();

    // Create and tombstone a constraint
    let payload = json!({"rule": "historical"});
    let mut constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        payload,
    );
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Tombstone it
    constraint.tombstone();
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Verify it's still in database (not hard deleted)
    let retrieved = SqliteRepo::get_constraint(&conn, "c1")
        .unwrap()
        .expect("Historical constraint should be preserved");

    assert!(retrieved.is_deleted());
    assert!(retrieved.deleted_at.is_some());

    // It should appear in list_constraints query (which returns all, including tombstoned)
    let all_constraints = SqliteRepo::list_constraints(&conn).unwrap();
    assert_eq!(all_constraints.len(), 1);
    assert!(all_constraints[0].is_deleted());
}

#[test]
fn test_foreign_key_constraint_ep_id() {
    let conn = setup_test_db();

    // Try to create ep_constraint_ref with non-existent EP
    let ref_record = EpConstraintRef::new("nonexistent-ep".to_string(), "c1".to_string(), 0);
    let result = SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record);

    // Should fail due to FK constraint
    assert!(result.is_err());
}

#[test]
fn test_foreign_key_constraint_constraint_id() {
    let conn = setup_test_db();

    // Create ettle and EP but no constraint
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    // Try to create ep_constraint_ref with non-existent constraint
    let ref_record =
        EpConstraintRef::new("ep-1".to_string(), "nonexistent-constraint".to_string(), 0);
    let result = SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record);

    // Should fail due to FK constraint
    assert!(result.is_err());
}

#[test]
fn test_hydration_loads_constraints() {
    let conn = setup_test_db();

    // Create ettle, EP, constraint, and attachment
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    let payload = json!({"rule": "test"});
    let constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        payload.clone(),
    );
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    let ref_record = EpConstraintRef::new("ep-1".to_string(), "c1".to_string(), 0);
    SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record).unwrap();

    // Load into Store
    let store = hydration::load_tree(&conn).unwrap();

    // Verify constraint was loaded
    let loaded_constraint = store.get_constraint("c1").unwrap();
    assert_eq!(loaded_constraint.constraint_id, "c1");
    assert_eq!(loaded_constraint.family, "ABB");
    assert_eq!(loaded_constraint.payload_json, payload);

    // Verify attachment was loaded
    assert!(store.is_constraint_attached_to_ep("ep-1", "c1"));
}

#[test]
fn test_hydration_loads_multiple_constraints_with_different_families() {
    let conn = setup_test_db();

    // Create ettle and EP
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    // Create constraints with different families
    let families = ["ABB", "SBB", "Observability"];
    for (i, family) in families.iter().enumerate() {
        let constraint = Constraint::new(
            format!("c{}", i),
            family.to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"index": i}),
        );
        SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

        let ref_record = EpConstraintRef::new("ep-1".to_string(), format!("c{}", i), i as i32);
        SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record).unwrap();
    }

    // Load into Store
    let store = hydration::load_tree(&conn).unwrap();

    // Verify all constraints were loaded
    for (i, family) in families.iter().enumerate() {
        let constraint = store.get_constraint(&format!("c{}", i)).unwrap();
        assert_eq!(constraint.family, *family);
        assert!(store.is_constraint_attached_to_ep("ep-1", &format!("c{}", i)));
    }
}

#[test]
fn test_hydration_loads_deleted_constraints() {
    let conn = setup_test_db();

    // Create constraint and mark as deleted
    let mut constraint = Constraint::new(
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({}),
    );
    constraint.tombstone();
    SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

    // Load into Store
    let store = hydration::load_tree(&conn).unwrap();

    // Verify deleted constraint was loaded
    let loaded = store.get_constraint("c1");
    // Should return error because constraint is deleted
    assert!(loaded.is_err());
    assert!(matches!(
        loaded,
        Err(ettlex_core::errors::EttleXError::ConstraintDeleted { .. })
    ));
}

#[test]
fn test_hydration_loads_multiple_refs_for_same_ep() {
    let conn = setup_test_db();

    // Create ettle and EP
    let ettle = Ettle::new("ettle-1".to_string(), "Test".to_string());
    SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

    let ep = Ep::new(
        "ep-1".to_string(),
        "ettle-1".to_string(),
        0,
        true,
        "why".to_string(),
        "what".to_string(),
        "how".to_string(),
    );
    SqliteRepo::persist_ep(&conn, &ep).unwrap();

    // Create multiple constraints attached to same EP
    for i in 0..5 {
        let constraint = Constraint::new(
            format!("c{}", i),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"ordinal": i}),
        );
        SqliteRepo::persist_constraint(&conn, &constraint).unwrap();

        let ref_record = EpConstraintRef::new("ep-1".to_string(), format!("c{}", i), i);
        SqliteRepo::persist_ep_constraint_ref(&conn, &ref_record).unwrap();
    }

    // Load into Store
    let store = hydration::load_tree(&conn).unwrap();

    // Verify all refs were loaded with correct ordinals
    let refs = store.list_ep_constraint_refs("ep-1");
    assert_eq!(refs.len(), 5);

    // Verify ordinal ordering
    let mut ordinals: Vec<_> = refs.iter().map(|r| r.ordinal).collect();
    ordinals.sort();
    assert_eq!(ordinals, vec![0, 1, 2, 3, 4]);
}
