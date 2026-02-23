//! CLI snapshot integration tests
//!
//! These tests verify that the CLI snapshot command correctly delegates to
//! the engine layer's action command infrastructure.

use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

fn setup_test_repo(temp_dir: &TempDir) -> (PathBuf, PathBuf) {
    let db_path = temp_dir.path().join("store.db");
    let cas_path = temp_dir.path().join("cas");
    fs::create_dir_all(&cas_path).unwrap();

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    // Seed: Simple tree with one leaf EP
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    (db_path, cas_path)
}

#[test]
fn test_cli_snapshot_commit_with_leaf_flag() {
    // Scenario: CLI snapshot commit delegates to action:commands
    // When: `ettlex snapshot commit --leaf <leaf_ep_id>`
    // Then: calls apply_engine_command(SnapshotCommit{leaf_ep_id}), succeeds

    let temp_dir = TempDir::new().unwrap();
    let (db_path, _cas_path) = setup_test_repo(&temp_dir);

    // Build the CLI binary
    let cli_bin = env!("CARGO_BIN_EXE_ettlex-cli");

    // Execute: CLI snapshot commit with --leaf flag
    let output = Command::new(cli_bin)
        .current_dir(temp_dir.path())
        .args([
            "snapshot",
            "commit",
            "--leaf",
            "ep:root:0",
            "--db",
            db_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute CLI");

    // Assert: Command succeeded
    assert!(
        output.status.success(),
        "CLI command should succeed. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: Output contains snapshot ID
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("snapshot_id") || stdout.contains("Snapshot committed"),
        "Output should confirm snapshot was committed"
    );

    // Assert: Snapshot was written to database
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "Expected one snapshot in database");
}

#[test]
fn test_cli_snapshot_commit_with_root_flag_legacy() {
    // Scenario: CLI snapshot commit with --root delegates to legacy path
    // When: `ettlex snapshot commit --root <root_ettle_id>`
    // Then: calls snapshot_commit_by_root_legacy(), succeeds when one leaf

    let temp_dir = TempDir::new().unwrap();
    let (db_path, _cas_path) = setup_test_repo(&temp_dir);

    let cli_bin = env!("CARGO_BIN_EXE_ettlex-cli");

    // Execute: CLI snapshot commit with --root flag
    let output = Command::new(cli_bin)
        .current_dir(temp_dir.path())
        .args([
            "snapshot",
            "commit",
            "--root",
            "ettle:root",
            "--db",
            db_path.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to execute CLI");

    // Assert: Command succeeded
    assert!(
        output.status.success(),
        "CLI command should succeed. Stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert: Snapshot was written to database
    let conn = Connection::open(&db_path).unwrap();
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "Expected one snapshot in database");
}
