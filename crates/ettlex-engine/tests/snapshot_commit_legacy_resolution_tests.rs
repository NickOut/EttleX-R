//! Tests: Legacy root resolution for snapshot commit
//!
//! Since `snapshot_commit_by_root_legacy` is pub(crate), these tests exercise
//! root resolution via the public `resolve_root_to_leaf_ep` function and the
//! `apply_engine_command` path.

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy::NoopCommitPolicyHook;
use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::{resolve_root_to_leaf_ep, SnapshotOptions};
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

// S6: Legacy root resolves to one leaf → commit succeeds via apply_engine_command
#[test]
fn test_legacy_root_resolves_when_exactly_one_leaf() {
    let (_tmp, mut conn, cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP', 0, 0, 0);
    "#).unwrap();

    // Resolve root → leaf
    let leaf_ep_id = resolve_root_to_leaf_ep(&mut conn, "ettle:root")
        .expect("Should resolve root to exactly one leaf");

    // Commit via canonical path
    let result = apply_engine_command(
        EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref: "policy/default@0".to_string(),
            profile_ref: None,
            options: SnapshotOptions {
                expected_head: None,
                dry_run: false,
                allow_dedup: false,
            },
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
    let EngineCommandResult::SnapshotCommit(r) = result.unwrap() else {
        panic!("Expected Committed")
    };
    assert!(!r.snapshot_id.is_empty());
}

// S7: RootEttleAmbiguous includes structured candidate leaf EP ids
#[test]
fn test_legacy_root_fails_when_multiple_leaves() {
    let (_tmp, mut conn, _cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP 0', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:1', 'ettle:root', 1, 1, NULL, NULL, 'Leaf EP 1', 0, 0, 0);
    "#).unwrap();

    let result = resolve_root_to_leaf_ep(&mut conn, "ettle:root");

    assert!(result.is_err(), "Expected failure with multiple leaves");
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ExErrorKind::RootEttleAmbiguous,
        "Expected RootEttleAmbiguous, got: {:?}",
        err.kind()
    );

    // S7: structured candidates field contains both leaf EP ids
    let candidates = err
        .candidates()
        .expect("candidates must be populated for RootEttleAmbiguous");
    assert!(
        candidates.contains(&"ep:root:0".to_string()),
        "candidates should include ep:root:0, got: {:?}",
        candidates
    );
    assert!(
        candidates.contains(&"ep:root:1".to_string()),
        "candidates should include ep:root:1, got: {:?}",
        candidates
    );
}

// S8: No leaves → RootEttleInvalid (not NotFound)
#[test]
fn test_legacy_root_fails_when_no_leaves() {
    let (_tmp, mut conn, _cas) = setup_db();

    conn.execute_batch(r#"
        INSERT INTO ettles VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles VALUES ('ettle:child', 'Child', 'ettle:root', 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', 'Non-leaf EP', 0, 0, 0);
    "#).unwrap();

    let result = resolve_root_to_leaf_ep(&mut conn, "ettle:root");

    assert!(result.is_err(), "Expected failure with no leaves");
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ExErrorKind::RootEttleInvalid,
        "Expected RootEttleInvalid, got: {:?}",
        err.kind()
    );
}
