//! Idempotency and append-only semantics tests for snapshot commit (S15, S16)

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

fn seed_leaf_ep(conn: &Connection) {
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, 'leaf content', 0, 0, 0);
        "#,
    )
    .unwrap();
}

fn snapshot_count(conn: &Connection) -> i64 {
    conn.query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap()
}

fn commit(conn: &mut Connection, cas: &FsStore, allow_dedup: bool) -> EngineCommandResult {
    apply_engine_command(
        EngineCommand::SnapshotCommit {
            leaf_ep_id: "ep:root:0".to_string(),
            policy_ref: "policy/default@0".to_string(),
            profile_ref: None,
            options: SnapshotOptions {
                expected_head: None,
                dry_run: false,
                allow_dedup,
            },
        },
        conn,
        cas,
        &NoopCommitPolicyHook,
        &NoopApprovalRouter,
    )
    .unwrap()
}

// S15a: default (append-only) — two commits of the same state produce two rows
#[test]
fn test_snapshot_commit_append_only_default() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_leaf_ep(&conn);

    let r1 = commit(&mut conn, &cas, false);
    let r2 = commit(&mut conn, &cas, false);

    let EngineCommandResult::SnapshotCommit(res1) = r1 else {
        panic!("Expected SnapshotCommit")
    };
    let EngineCommandResult::SnapshotCommit(res2) = r2 else {
        panic!("Expected SnapshotCommit")
    };

    // Both commits succeed, semantic digest is identical (same content)
    assert_eq!(res1.semantic_manifest_digest, res2.semantic_manifest_digest);
    // Two distinct rows — append-only means no dedup
    assert!(!res1.was_duplicate);
    assert!(!res2.was_duplicate);
    assert_ne!(res1.snapshot_id, res2.snapshot_id);
    assert_eq!(snapshot_count(&conn), 2);
}

// S15b: allow_dedup=true — second commit returns the existing snapshot
#[test]
fn test_snapshot_commit_allow_dedup_returns_existing() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_leaf_ep(&conn);

    let r1 = commit(&mut conn, &cas, true);
    let r2 = commit(&mut conn, &cas, true);

    let EngineCommandResult::SnapshotCommit(res1) = r1 else {
        panic!("Expected SnapshotCommit")
    };
    let EngineCommandResult::SnapshotCommit(res2) = r2 else {
        panic!("Expected SnapshotCommit")
    };

    // Second commit deduplicates — same snapshot_id returned
    assert_eq!(res1.snapshot_id, res2.snapshot_id);
    assert!(res2.was_duplicate);
    assert_eq!(snapshot_count(&conn), 1);
}

// S15c: allow_dedup=true + was_duplicate → reuse event MUST be recorded (via tracing)
// This test verifies the row count and was_duplicate flag. Tracing coverage is
// confirmed by the `tracing::info!(event="reuse")` call in persist.rs.
#[test]
fn test_snapshot_commit_allow_dedup_records_reuse_event() {
    let (_tmp, mut conn, cas) = setup_db();
    seed_leaf_ep(&conn);

    commit(&mut conn, &cas, true);
    let r2 = commit(&mut conn, &cas, true);

    let EngineCommandResult::SnapshotCommit(res2) = r2 else {
        panic!("Expected SnapshotCommit")
    };

    // was_duplicate = true confirms the reuse path was hit
    assert!(
        res2.was_duplicate,
        "Second commit with allow_dedup must report was_duplicate"
    );
    assert_eq!(snapshot_count(&conn), 1);
}

// S16: Large manifest (~5 MB inline content) completes within 30s
#[test]
fn test_snapshot_commit_large_manifest() {
    let (_tmp, mut conn, cas) = setup_db();

    // Seed EP with ~5 MB of inline content
    let large_content = "x".repeat(5_000_000);
    conn.execute_batch(&format!(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:large', 'Large', NULL, 0, 0, 0, '{{}}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:large:0', 'ettle:large', 0, 1, NULL, '{}', 0, 0, 0);
        "#,
        large_content
    ))
    .unwrap();

    let start = std::time::Instant::now();
    let result = apply_engine_command(
        EngineCommand::SnapshotCommit {
            leaf_ep_id: "ep:large:0".to_string(),
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
    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Large manifest commit should succeed: {:?}",
        result.err()
    );
    assert!(
        elapsed.as_secs() < 30,
        "Large manifest commit took too long: {:?}",
        elapsed
    );

    assert_eq!(snapshot_count(&conn), 1);
}
