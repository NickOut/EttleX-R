//! CLI integration tests for `action_ep_update` Ettle.
//!
//! Written from spec only.
//!
//! Scenario → test mapping:
//!   S-AU-5  test_cli_ep_update_delegates_to_action_layer

#![allow(clippy::unwrap_used)]

use ettlex_cli::commands::ep::UpdateArgs;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_db() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db").to_string_lossy().to_string();
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    // Seed an ettle + EP
    conn.execute_batch(
        "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('ettle:cli1', 'CLI Test Ettle', NULL, 0, 0, 0, '{}');
         INSERT INTO eps (id, ettle_id, ordinal, normative, content_inline, deleted, created_at, updated_at)
         VALUES ('ep:cli1:0', 'ettle:cli1', 0, 1,
                 '{\"why\":\"original why\",\"what\":\"w\",\"how\":\"h\"}',
                 0, 0, 0);",
    )
    .unwrap();

    (dir, db_path)
}

// ---------------------------------------------------------------------------
// S-AU-5: CLI ep update delegates to the action layer (apply_mcp_command)
// ---------------------------------------------------------------------------

#[test]
fn test_cli_ep_update_delegates_to_action_layer() {
    let (_dir, db_path) = setup_db();
    let cas_path = _dir.path().join("cas").to_string_lossy().to_string();

    let result = ettlex_cli::commands::ep::execute_update(UpdateArgs {
        ep_id: "ep:cli1:0".to_string(),
        why: Some("new why via cli".to_string()),
        what: None,
        how: None,
        title: None,
        db: db_path.clone(),
        cas: cas_path,
    });

    assert!(result.is_ok(), "CLI ep update must succeed: {:?}", result);

    // Verify the EP was actually updated in the DB
    let conn = Connection::open(&db_path).unwrap();
    let ep = ettlex_store::repo::SqliteRepo::get_ep(&conn, "ep:cli1:0")
        .unwrap()
        .unwrap();
    assert_eq!(ep.why, "new why via cli", "why must be updated in the DB");
    assert_eq!(ep.what, "w", "what must be preserved");
}
