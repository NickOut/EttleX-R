//! Tests: Legacy root resolution for snapshot commit

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy::NoopCommitPolicyHook;
use ettlex_engine::commands::snapshot::{
    snapshot_commit_by_root_legacy, SnapshotCommitOutcome, SnapshotOptions,
};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_db() -> (TempDir, Connection, FsStore) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_dir = temp_dir.path().join("cas");
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_dir);
    (temp_dir, conn, cas)
}

#[test]
fn test_legacy_root_resolves_when_exactly_one_leaf() {
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
    "#).unwrap();

    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        None,
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );

    assert!(
        result.is_ok(),
        "Expected legacy root resolution to succeed: {:?}",
        result.err()
    );
    let SnapshotCommitOutcome::Committed(r) = result.unwrap() else {
        panic!("Expected Committed")
    };
    assert!(!r.snapshot_id.is_empty());
}

#[test]
fn test_legacy_root_fails_when_multiple_leaves() {
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP 0', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:1', 'ettle:root', 1, 1, NULL, NULL, 'Leaf EP 1', 0, 0, 0);
    "#).unwrap();

    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        None,
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "Expected failure with multiple leaves");
    // Updated: error is RootEttleAmbiguous (not AmbiguousSelection)
    assert_eq!(
        result.unwrap_err().kind(),
        ettlex_core::errors::ExErrorKind::RootEttleAmbiguous
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_legacy_root_fails_when_no_leaves() {
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles VALUES ('ettle:child', 'Child', 'ettle:root', 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', 'Non-leaf EP', 0, 0, 0);
    "#).unwrap();

    let result = snapshot_commit_by_root_legacy(
        "ettle:root",
        "policy/default@0",
        None,
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "Expected failure with no leaves");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}
