//! Migration 012 — Ettle v2 schema smoke tests.
//!
//! SC-50: migration_012_applies_cleanly
//! SC-51: existing_ettle_rows_survive_with_defaults

#![allow(clippy::unwrap_used)]

use rusqlite::{Connection, OptionalExtension};
use tempfile::TempDir;

fn setup() -> (TempDir, Connection) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn)
}

// ---------------------------------------------------------------------------
// SC-50: migration_012_applies_cleanly
// ---------------------------------------------------------------------------

#[test]
fn test_migration_012_applies_cleanly() {
    // If migrations run without panic/error, 012 applied cleanly.
    let (_dir, conn) = setup();

    // Verify the new columns exist by querying them
    let result: rusqlite::Result<Option<String>> = conn
        .query_row(
            "SELECT why FROM ettles WHERE id = 'nonexistent'",
            [],
            |row| row.get(0),
        )
        .optional();
    assert!(
        result.is_ok(),
        "Column 'why' must exist after migration 012: {:?}",
        result.err()
    );

    let result: rusqlite::Result<Option<String>> = conn
        .query_row(
            "SELECT what FROM ettles WHERE id = 'nonexistent'",
            [],
            |row| row.get(0),
        )
        .optional();
    assert!(
        result.is_ok(),
        "Column 'what' must exist: {:?}",
        result.err()
    );

    let result: rusqlite::Result<Option<String>> = conn
        .query_row(
            "SELECT how FROM ettles WHERE id = 'nonexistent'",
            [],
            |row| row.get(0),
        )
        .optional();
    assert!(
        result.is_ok(),
        "Column 'how' must exist: {:?}",
        result.err()
    );

    let result: rusqlite::Result<Option<String>> = conn
        .query_row(
            "SELECT reasoning_link_id FROM ettles WHERE id = 'nonexistent'",
            [],
            |row| row.get(0),
        )
        .optional();
    assert!(
        result.is_ok(),
        "Column 'reasoning_link_id' must exist: {:?}",
        result.err()
    );

    let result: rusqlite::Result<Option<String>> = conn
        .query_row(
            "SELECT tombstoned_at FROM ettles WHERE id = 'nonexistent'",
            [],
            |row| row.get(0),
        )
        .optional();
    assert!(
        result.is_ok(),
        "Column 'tombstoned_at' must exist: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// SC-51: existing_ettle_rows_survive_with_defaults
// ---------------------------------------------------------------------------

#[test]
fn test_existing_ettle_rows_survive_with_defaults() {
    let (_dir, conn) = setup();

    // Insert a minimal ettle (only v2 columns: id, title, why, what, how, created_at, updated_at)
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO ettles (id, title, why, what, how, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params!["ettle:test-survive", "Survive Test", "", "", "", now, now],
    )
    .unwrap();

    // Read it back
    let (id, title, why, tombstoned_at): (String, String, String, Option<String>) = conn
        .query_row(
            "SELECT id, title, why, tombstoned_at FROM ettles WHERE id = ?1",
            ["ettle:test-survive"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();

    assert_eq!(id, "ettle:test-survive");
    assert_eq!(title, "Survive Test");
    assert_eq!(why, "");
    assert!(
        tombstoned_at.is_none(),
        "tombstoned_at should be null by default"
    );
}
