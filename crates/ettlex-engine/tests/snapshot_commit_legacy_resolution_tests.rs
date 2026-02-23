//! RED Tests: Legacy root resolution for snapshot commit
//!
//! These tests verify deterministic root-to-leaf resolution for backward compatibility.

use ettlex_engine::commands::snapshot::{snapshot_commit_by_root_legacy, SnapshotOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn test_legacy_root_resolves_when_exactly_one_leaf() {
    // Scenario: Legacy root selector resolves deterministically when exactly one leaf exists
    // Given: Ettle has exactly one leaf EP
    // When: snapshot_commit_by_root_legacy(root_ettle_id)
    // Then: resolves to leaf_ep_id, snapshot committed successfully

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Root Ettle -> EP0 (leaf)
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Commit via legacy root path
    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    );

    // Assert: Command succeeded
    assert!(
        result.is_ok(),
        "Expected legacy root resolution to succeed with one leaf, got: {:?}",
        result.err()
    );

    let result = result.unwrap();
    assert!(!result.snapshot_id.is_empty());
}

#[test]
fn test_legacy_root_fails_when_multiple_leaves() {
    // Scenario: Legacy root selector fails when multiple leaves exist
    // Given: Ettle has >1 leaf EP
    // When: snapshot_commit_by_root_legacy(root_ettle_id)
    // Then: ExError with kind=AmbiguousSelection, includes candidate leaf IDs

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Root Ettle -> EP0 (leaf), EP1 (leaf)
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP 0', 0, 0, 0);

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:1', 'ettle:root', 1, 1, NULL, NULL, 'Leaf EP 1', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Commit via legacy root path
    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    );

    // Assert: Command failed
    assert!(
        result.is_err(),
        "Expected legacy root resolution to fail with multiple leaves"
    );

    // Assert: Error kind is AmbiguousSelection
    // TODO: Check error kind when store::errors::Result exposes ExError

    // Assert: No snapshot committed
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "Expected no snapshot to be committed");
}

#[test]
fn test_legacy_root_fails_when_no_leaves() {
    // Scenario: Legacy root selector fails when no leaves exist
    // Given: Ettle has no leaf EPs
    // When: snapshot_commit_by_root_legacy(root_ettle_id)
    // Then: ExError with kind=NotFound

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Root Ettle -> EP0 (has child, not a leaf) -> Child Ettle
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:child', 'Child Ettle', 'ettle:root', 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', NULL, 'Non-leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Commit via legacy root path
    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    );

    // Assert: Command failed
    assert!(
        result.is_err(),
        "Expected legacy root resolution to fail with no leaves"
    );

    // Assert: Error kind is NotFound
    // TODO: Check error kind when store::errors::Result exposes ExError

    // Assert: No snapshot committed
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0, "Expected no snapshot to be committed");
}
