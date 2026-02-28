//! Tests: Snapshot commit via action command (leaf-scoped)

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy::NoopCommitPolicyHook;
use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::SnapshotOptions;
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
fn test_snapshot_commit_succeeds_via_action_command() {
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP content', 0, 0, 0);
        "#,
    ).unwrap();

    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
    };

    let result = apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "Expected command to succeed, got: {:?}",
        result.err()
    );

    let EngineCommandResult::SnapshotCommit(r) = result.unwrap() else {
        panic!("Expected SnapshotCommit result");
    };
    assert!(!r.snapshot_id.is_empty());
    assert!(!r.manifest_digest.is_empty());
    assert_eq!(
        r.head_after, r.manifest_digest,
        "head_after must equal manifest_digest"
    );

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE snapshot_id = ?",
            [&r.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    let manifest_bytes = cas.read(&r.manifest_digest).unwrap();
    assert!(!manifest_bytes.is_empty());
}

#[test]
fn test_snapshot_commit_rejects_non_leaf_ep() {
    // NotALeaf (not ConstraintViolation) — updated per ep:snapshot_commit_policy:0
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles VALUES ('ettle:child', 'Child', 'ettle:root', 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', 'Non-leaf', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:child:0', 'ettle:child', 0, 1, NULL, 'Leaf', 0, 0, 0);
    "#).unwrap();

    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
    };

    let result = apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "Expected failure for non-leaf EP");

    // Verify the error is NotALeaf
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::NotALeaf,
        "Expected NotALeaf error, got: {:?}",
        err.kind()
    );

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_snapshot_commit_rejects_unknown_ep() {
    let (_tmp, mut conn, cas) = setup_db();

    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:missing:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
    };

    let result = apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    );
    assert!(result.is_err());

    let err = result.unwrap_err();
    assert_eq!(err.kind(), ettlex_core::errors::ExErrorKind::NotFound);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}
