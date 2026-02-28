//! Determinism invariant tests for snapshot commit

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy::NoopCommitPolicyHook;
use ettlex_engine::commands::engine_command::{
    apply_engine_command, EngineCommand, EngineCommandResult,
};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
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

fn seed_simple_tree(conn: &Connection) {
    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'Leaf EP content', 0, 0, 0);
    "#).unwrap();
}

// S9: Semantic manifest digest is identical across the legacy and action-command paths
#[test]
fn test_snapshot_output_deterministic_across_paths() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_simple_tree(&conn);

    // Legacy path (append-only: no dedup)
    let old_result = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Action-command path (allow_dedup=true: deduplicates against the row just written)
    let cmd = EngineCommand::SnapshotCommit {
        leaf_ep_id: "ep:root:0".to_string(),
        policy_ref: "policy/default@0".to_string(),
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: true,
        },
    };

    let EngineCommandResult::SnapshotCommit(new_result) = apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    )
    .unwrap() else {
        panic!("Expected SnapshotCommit")
    };

    assert_eq!(
        old_result.semantic_manifest_digest, new_result.semantic_manifest_digest,
        "Semantic manifest digest should be identical across paths"
    );
    assert!(
        old_result.was_duplicate || new_result.was_duplicate,
        "Second commit with allow_dedup should be duplicate"
    );
}

// S10: created_at non-determinism is preserved — append-only default creates two rows
#[test]
fn test_created_at_non_determinism_preserved() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_simple_tree(&conn);

    let result1 = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let result2 = snapshot_commit(
        "ettle:root",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Semantic digest is identical (same content)
    assert_eq!(
        result1.semantic_manifest_digest, result2.semantic_manifest_digest,
        "Semantic digest must be equal (same content)"
    );
    // Append-only: two distinct rows — snapshot_ids differ
    assert_ne!(
        result1.snapshot_id, result2.snapshot_id,
        "Append-only default must produce distinct snapshot_ids"
    );
    assert!(!result1.was_duplicate);
    assert!(!result2.was_duplicate);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "Append-only default must produce 2 rows");
}

#[test]
fn test_no_extra_mutation_during_commit() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_simple_tree(&conn);

    let ettle_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |r| r.get(0))
        .unwrap();
    let ep_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |r| r.get(0))
        .unwrap();

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

    apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    )
    .unwrap();

    let ettle_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |r| r.get(0))
        .unwrap();
    let ep_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ettle_count_before, ettle_count_after);
    assert_eq!(ep_count_before, ep_count_after);

    let snapshot_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();
    assert_eq!(snapshot_count, 1);
}
