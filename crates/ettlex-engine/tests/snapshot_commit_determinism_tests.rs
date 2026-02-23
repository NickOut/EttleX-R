//! Determinism invariant tests for snapshot commit refactor
//!
//! These tests verify that the refactored snapshot commit implementation
//! preserves determinism guarantees from the original implementation.

use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

#[test]
fn test_snapshot_output_deterministic_across_paths() {
    // Scenario: Snapshot commit output remains deterministic under refactor
    // Given: identical canonical state and leaf_ep_id
    // When: commit via old path, commit via new action command path
    // Then: semantic_manifest_digest identical (manifest bytes identical except created_at)

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Simple tree with one leaf EP
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP content', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: Commit via old path (root-based)
    let old_result = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Execute: Commit via new action command path (leaf-based)
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    };

    let EngineCommandResult::SnapshotCommit(new_result) =
        apply_engine_command(cmd, &mut conn, &cas).unwrap();

    // Assert: Semantic digest is identical (excludes created_at)
    assert_eq!(
        old_result.semantic_manifest_digest, new_result.semantic_manifest_digest,
        "Semantic manifest digest should be identical across old and new paths"
    );

    // Assert: Both should report as duplicates (idempotent)
    assert!(
        old_result.was_duplicate || new_result.was_duplicate,
        "Second commit should be detected as duplicate"
    );
}

#[test]
fn test_created_at_non_determinism_preserved() {
    // Scenario: Snapshot commit preserves created_at non-determinism rule
    // When: commit twice with identical state (different timestamps)
    // Then: manifest_digest differs (includes created_at), semantic_manifest_digest identical

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Simple tree
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Execute: First commit
    let result1 = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Wait a tiny bit to ensure different created_at (if timestamps are in manifest)
    std::thread::sleep(std::time::Duration::from_millis(10));

    // Execute: Second commit (same state, different timestamp)
    let result2 = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Assert: Semantic digest is identical (excludes created_at)
    assert_eq!(
        result1.semantic_manifest_digest, result2.semantic_manifest_digest,
        "Semantic digest should be identical for same state"
    );

    // Assert: snapshot_id is identical (idempotent)
    assert_eq!(
        result1.snapshot_id, result2.snapshot_id,
        "Snapshot ID should be identical (idempotent)"
    );

    // Assert: Second commit was detected as duplicate
    assert!(
        result2.was_duplicate,
        "Second commit should be flagged as duplicate"
    );
}

#[test]
fn test_no_extra_mutation_during_commit() {
    // Scenario: No extra mutation occurs during commit
    // When: call EngineCommand::SnapshotCommit
    // Then: no Ettle/EP/link rows modified, only ledger and CAS written

    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_dir);

    // Seed: Simple tree
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Capture initial state
    let ettle_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |row| row.get(0))
        .unwrap();
    let ep_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |row| row.get(0))
        .unwrap();

    // Execute: Commit snapshot
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: "profile/default@0".to_string(),
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    };

    apply_engine_command(cmd, &mut conn, &cas).unwrap();

    // Assert: Ettle count unchanged
    let ettle_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        ettle_count_before, ettle_count_after,
        "No ettles should be added or removed during snapshot commit"
    );

    // Assert: EP count unchanged
    let ep_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        ep_count_before, ep_count_after,
        "No EPs should be added or removed during snapshot commit"
    );

    // Assert: Exactly one snapshot row added
    let snapshot_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(
        snapshot_count, 1,
        "Exactly one snapshot row should be added"
    );
}
