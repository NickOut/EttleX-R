//! Slice 02 migration tests (SC-S02-01 through SC-S02-06, SC-S02-07).
//!
//! Tests that migration 014 applies cleanly and produces the expected schema.

use ettlex_store::migrations::apply_migrations;
use rusqlite::Connection;

fn setup_db() -> Connection {
    let mut conn = Connection::open_in_memory().expect("in-memory db");
    apply_migrations(&mut conn).expect("migrations should apply");
    conn
}

// SC-S02-01: Migration 014 applies cleanly
#[test]
fn test_migration_014_applies_cleanly() {
    // If apply_migrations panics, this test fails. We just need it to succeed.
    let conn = setup_db();
    // Verify basic table existence
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='command_log'",
            [],
            |r| r.get(0),
        )
        .expect("query should work");
    assert_eq!(
        count, 1,
        "command_log table should exist after migration 014"
    );
}

// SC-S02-02: command_log table exists after rename
#[test]
fn test_command_log_table_exists_after_rename() {
    let conn = setup_db();
    let exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='command_log'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c == 1)
        .expect("query should work");
    assert!(exists, "command_log table should exist");

    // Old name should NOT exist
    let old_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mcp_command_log'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .map(|c| c == 1)
        .expect("query should work");
    assert!(!old_exists, "mcp_command_log should not exist after rename");
}

// SC-S02-03: provenance_events has occurred_at column after migration
#[test]
fn test_provenance_events_occurred_at_after_migration() {
    let conn = setup_db();
    // Try to insert using occurred_at column
    let result = conn.execute(
        "INSERT INTO provenance_events (kind, correlation_id, occurred_at) VALUES ('test', 'test-id', '2026-01-01T00:00:00Z')",
        [],
    );
    assert!(
        result.is_ok(),
        "should be able to insert occurred_at: {:?}",
        result.err()
    );

    // Verify timestamp column is gone
    let old_col_err = conn.execute(
        "INSERT INTO provenance_events (kind, correlation_id, timestamp) VALUES ('test2', 'test-id-2', 1234567890)",
        [],
    );
    assert!(
        old_col_err.is_err(),
        "old 'timestamp' column should not exist (got success when error expected)"
    );
}

// SC-S02-04: relation_type_registry seeded with 4 entries
#[test]
fn test_relation_type_registry_seeded_by_migration() {
    let conn = setup_db();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM relation_type_registry", [], |r| {
            r.get(0)
        })
        .expect("query should work");
    assert_eq!(count, 4, "relation_type_registry should have 4 entries");

    // Check specific types exist
    for rt in &["refinement", "option", "semantic_peer", "constraint"] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM relation_type_registry WHERE relation_type = ?1",
                [rt],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c == 1)
            .expect("query should work");
        assert!(exists, "relation_type_registry should contain '{}'", rt);
    }
}

// SC-S02-05: constraint type entry has cycle_check true in properties_json
#[test]
fn test_constraint_registry_entry_has_cycle_check() {
    let conn = setup_db();
    let props: String = conn
        .query_row(
            "SELECT properties_json FROM relation_type_registry WHERE relation_type = 'constraint'",
            [],
            |r| r.get(0),
        )
        .expect("constraint entry should exist");

    let val: serde_json::Value = serde_json::from_str(&props).expect("valid JSON");
    let cycle_check = val
        .get("cycle_check")
        .and_then(|v| v.as_bool())
        .expect("cycle_check field should exist");
    assert!(
        cycle_check,
        "constraint relation type should have cycle_check = true"
    );

    // Also verify semantic_peer does NOT have cycle_check
    let peer_props: String = conn
        .query_row(
            "SELECT properties_json FROM relation_type_registry WHERE relation_type = 'semantic_peer'",
            [],
            |r| r.get(0),
        )
        .expect("semantic_peer entry should exist");
    let peer_val: serde_json::Value = serde_json::from_str(&peer_props).expect("valid JSON");
    let peer_cycle = peer_val
        .get("cycle_check")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    assert!(
        !peer_cycle,
        "semantic_peer should not have cycle_check = true"
    );
}

// SC-S02-06: legacy constraint tables absent after migration
#[test]
fn test_legacy_constraint_tables_absent() {
    let conn = setup_db();
    for table in &[
        "constraints",
        "ep_constraint_refs",
        "constraint_sets",
        "constraint_set_members",
        "constraint_associations",
    ] {
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                [table],
                |r| r.get::<_, i64>(0),
            )
            .map(|c| c == 1)
            .expect("query should work");
        assert!(
            !exists,
            "legacy table '{}' should not exist after migration 014",
            table
        );
    }
}
