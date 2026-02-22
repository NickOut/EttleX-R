// Test suite for snapshot commit orchestration
// Tests happy path, dry-run, logging boundaries, and error propagation

use ettlex_core::logging_facility::test_capture::init_test_capture;
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_test_repo() -> (TempDir, Connection, FsStore, String) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();

    // Apply migrations
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_path);

    // Insert test data directly using SQL (order matters for foreign keys)
    conn.execute_batch(
        r#"
        -- Insert root ettle
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root Ettle', NULL, 0, 0, 0, '{}');

        -- Insert child ettle first (needed for FK)
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:child', 'Child Ettle', 'ettle:root', 0, 0, 0, '{}');

        -- Insert root EP (with child mapping)
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child', NULL, 'Root content', 0, 0, 0);

        -- Insert child EP
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:child:0', 'ettle:child', 0, 1, NULL, NULL, 'Child content', 0, 0, 0);
        "#,
    )
    .unwrap();

    (temp_dir, conn, cas, "ettle:root".into())
}

#[test]
fn test_snapshot_commit_happy_path() {
    let (_temp_dir, mut conn, cas, root_ettle_id) = setup_test_repo();

    let result = snapshot_commit(
        &root_ettle_id,
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

    // Should return snapshot ID and digests
    assert!(!result.snapshot_id.is_empty());
    assert!(!result.manifest_digest.is_empty());
    assert!(!result.semantic_manifest_digest.is_empty());
    assert!(!result.was_duplicate);

    // Verify in database
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE snapshot_id = ?1",
            [&result.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Verify manifest in CAS
    let cas_content = cas.read(&result.manifest_digest).unwrap();
    assert!(!cas_content.is_empty());
}

#[test]
fn test_snapshot_commit_dry_run() {
    let (_temp_dir, mut conn, cas, root_ettle_id) = setup_test_repo();

    let result = snapshot_commit(
        &root_ettle_id,
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: true,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Should return computed digests but no snapshot_id
    assert!(result.snapshot_id.is_empty());
    assert!(!result.semantic_manifest_digest.is_empty());

    // Should NOT persist to database
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn test_snapshot_commit_logging() {
    let capture = init_test_capture();
    capture.clear(); // Clear events from previous tests

    let (_temp_dir, mut conn, cas, root_ettle_id) = setup_test_repo();

    snapshot_commit(
        &root_ettle_id,
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

    let events = capture.events();

    // Should have exactly one start and one end event
    capture.assert_event_exists("snapshot_commit", "start");
    capture.assert_event_exists("snapshot_commit", "end");

    // Should NOT have error events
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| e.event.as_deref() == Some("end_error"))
        .collect();
    assert_eq!(error_events.len(), 0);

    // Start event should have root_ettle_id
    let start_events: Vec<_> = events
        .iter()
        .filter(|e| e.event.as_deref() == Some("start"))
        .collect();
    assert_eq!(start_events.len(), 1);
    // Verify root_ettle_id field exists
    assert!(start_events[0].fields.contains_key("root_ettle_id"));

    // End event should have duration and snapshot_id
    let end_events: Vec<_> = events
        .iter()
        .filter(|e| e.event.as_deref() == Some("end"))
        .collect();
    assert_eq!(end_events.len(), 1);
    assert!(end_events[0].fields.contains_key("duration_ms"));
    assert!(end_events[0].fields.contains_key("snapshot_id"));
}

#[test]
fn test_snapshot_commit_error_logging() {
    let capture = init_test_capture();
    capture.clear(); // Clear events from previous tests

    let (_temp_dir, mut conn, cas, _root_ettle_id) = setup_test_repo();

    // Try to commit with nonexistent root ettle
    let result = snapshot_commit(
        "ettle:nonexistent",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    );

    assert!(result.is_err());

    let events = capture.events();

    // Should have start and error events
    capture.assert_event_exists("snapshot_commit", "start");
    capture.assert_event_exists("snapshot_commit", "end_error");

    // Should NOT have success end event
    let end_events: Vec<_> = events
        .iter()
        .filter(|e| e.event.as_deref() == Some("end"))
        .collect();
    assert_eq!(end_events.len(), 0);

    // Error event should have duration and error kind
    let error_events: Vec<_> = events
        .iter()
        .filter(|e| e.event.as_deref() == Some("end_error"))
        .collect();
    assert_eq!(error_events.len(), 1);
    assert!(error_events[0].fields.contains_key("duration_ms"));
    assert!(error_events[0].fields.contains_key("err.kind"));
}

#[test]
fn test_snapshot_commit_idempotent_across_calls() {
    let (_temp_dir, mut conn, cas, root_ettle_id) = setup_test_repo();

    // First commit
    let result1 = snapshot_commit(
        &root_ettle_id,
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

    // Second commit (same state)
    let result2 = snapshot_commit(
        &root_ettle_id,
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

    // Should return same snapshot ID (idempotent)
    assert_eq!(result1.snapshot_id, result2.snapshot_id);
    assert!(result2.was_duplicate);

    // Should only have one row in database
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);
}
