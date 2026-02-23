//! RED Test: Snapshot commit via action command (leaf-scoped)
//!
//! Scenario: Snapshot commit succeeds via action:commands apply (leaf-scoped)
//! Given: repository with tree + leaf EP exists
//! When: apply_engine_command(EngineCommand::SnapshotCommit{leaf_ep_id})
//! Then: snapshot_id returned, ledger row appended, CAS blob written

use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::SnapshotOptions;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn test_snapshot_commit_succeeds_via_action_command() {
    // Setup: temporary database and CAS
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    // Initialize database with migrations
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Create a simple tree with one leaf EP
    // Root Ettle -> EP0 (leaf, no child)
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP content', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Commit snapshot via EngineCommand
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    };

    let result = apply_engine_command(cmd, &mut conn, &cas);

    // Assert: Command succeeded
    assert!(
        result.is_ok(),
        "Expected command to succeed, got error: {:?}",
        result.err()
    );

    let EngineCommandResult::SnapshotCommit(result) = result.unwrap();

    // Assert: Snapshot ID returned (UUIDv7 format)
    assert!(
        !result.snapshot_id.is_empty(),
        "Expected non-empty snapshot_id"
    );

    // Assert: Manifest digest returned
    assert!(
        !result.manifest_digest.is_empty(),
        "Expected non-empty manifest_digest"
    );

    // Assert: Ledger row appended
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE snapshot_id = ?",
            [&result.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Expected one snapshot row in ledger");

    // Assert: CAS blob written
    let manifest_bytes = cas.read(&result.manifest_digest).unwrap();
    assert!(!manifest_bytes.is_empty(), "Expected CAS blob to exist");
}

#[test]
fn test_snapshot_commit_rejects_non_leaf_ep() {
    // Scenario: SnapshotCommit rejects non-leaf EP id
    // Given: EP exists but has child_ettle_id
    // When: apply_engine_command(SnapshotCommit{leaf_ep_id})
    // Then: ExError with kind=ConstraintViolation, no snapshot committed

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Create tree with non-leaf EP
    // Root Ettle -> EP0 (has child) -> Child Ettle
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:child', 'Child Ettle', 'ettle:root', 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', NULL, 'Non-leaf EP', 0, 0, 0);

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:child:0', 'ettle:child', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Try to commit snapshot for non-leaf EP
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(), // This EP has child_ettle_id!
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    };

    let result = apply_engine_command(cmd, &mut conn, &cas);

    // Assert: Command failed
    assert!(result.is_err(), "Expected command to fail for non-leaf EP");

    // Assert: No snapshot committed
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "Expected no snapshot to be committed");
}

#[test]
fn test_snapshot_commit_rejects_unknown_ep() {
    // Scenario: SnapshotCommit rejects unknown EP id
    // When: apply_engine_command(SnapshotCommit{leaf_ep_id="ep:missing"})
    // Then: ExError with kind=NotFound, no snapshot committed

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // No seed data - database is empty

    // Execute: Try to commit snapshot for unknown EP
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:missing:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    };

    let result = apply_engine_command(cmd, &mut conn, &cas);

    // Assert: Command failed
    assert!(result.is_err(), "Expected command to fail for unknown EP");

    // Assert: Error is NotFound
    // TODO: Add proper error kind checking when store::errors::Result exposes ExError

    // Assert: No snapshot committed
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "Expected no snapshot to be committed");
}
