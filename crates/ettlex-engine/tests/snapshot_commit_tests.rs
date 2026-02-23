// Test suite for snapshot commit orchestration
// Tests happy path, dry-run, logging boundaries, and error propagation

use ettlex_core::logging_facility::test_capture::init_test_capture;
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use ettlex_store::repo::hydration::load_tree;
use ettlex_store::repo::SqliteRepo;
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

#[test]
fn test_snapshot_commit_with_constraints() {
    use ettlex_core::ops::constraint_ops;
    use serde_json::json;

    let (_temp_dir, mut conn, cas, root_ettle_id) = setup_test_repo();

    // Load store
    let mut store = load_tree(&conn).unwrap();

    // Get EP0 of root ettle
    let ettle = store.get_ettle(&root_ettle_id).unwrap();
    let ep0_id = ettle.ep_ids[0].clone();

    // Create and attach a constraint
    constraint_ops::create_constraint(
        &mut store,
        "c1".to_string(),
        "ABB".to_string(),
        "Rule".to_string(),
        "EP".to_string(),
        json!({"rule": "test_rule"}),
    )
    .unwrap();

    constraint_ops::attach_constraint_to_ep(&mut store, ep0_id.clone(), "c1".to_string(), 0)
        .unwrap();

    // Persist constraint to database
    let constraint = store.get_constraint("c1").unwrap();
    SqliteRepo::persist_constraint(&conn, constraint).unwrap();

    let refs = store.list_ep_constraint_refs(&ep0_id);
    for ref_record in refs {
        SqliteRepo::persist_ep_constraint_ref(&conn, ref_record).unwrap();
    }

    // Commit snapshot
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

    // Read manifest from CAS
    let manifest_json = cas.read(&result.manifest_digest).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_json).unwrap();

    // Verify constraints envelope exists and contains our constraint
    assert!(manifest.get("constraints").is_some());
    let constraints = manifest.get("constraints").unwrap();

    // Should have declared_refs
    let declared_refs = constraints
        .get("declared_refs")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(!declared_refs.is_empty());

    // Should have families with ABB
    let families = constraints.get("families").unwrap().as_object().unwrap();
    assert!(families.contains_key("ABB"));

    // ABB family should have our constraint ref
    let abb_family = families.get("ABB").unwrap();
    let active_refs = abb_family.get("active_refs").unwrap().as_array().unwrap();
    assert_eq!(active_refs.len(), 1);
    assert!(active_refs[0].as_str().unwrap().contains("c1"));
}

#[test]
fn test_snapshot_commit_single_ettle_no_children() {
    // Behavioral: snapshot_commit should handle a single ettle with no refinement
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    // Insert just a root ettle with EP0, no children
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:solo', 'Solo Ettle', NULL, 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:solo:0', 'ettle:solo', 0, 1, NULL, NULL, 'Solo content', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = snapshot_commit(
        "ettle:solo",
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

    assert!(!result.snapshot_id.is_empty());

    // Verify manifest has single EP
    let manifest_json = cas.read(&result.manifest_digest).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_json).unwrap();
    let ept = manifest.get("ept").unwrap().as_array().unwrap();
    assert_eq!(ept.len(), 1); // Only EP0
}

#[test]
fn test_snapshot_commit_deep_tree_three_levels() {
    // Behavioral: snapshot_commit should handle deep hierarchies
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    // Create 3-level tree: root → level1 → level2
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES
            ('ettle:root', 'Root', NULL, 0, 0, 0, '{}'),
            ('ettle:level1', 'Level1', 'ettle:root', 0, 0, 0, '{}'),
            ('ettle:level2', 'Level2', 'ettle:level1', 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES
            ('ep:root:0', 'ettle:root', 0, 1, 'ettle:level1', NULL, 'Root EP0', 0, 0, 0),
            ('ep:level1:0', 'ettle:level1', 0, 1, 'ettle:level2', NULL, 'Level1 EP0', 0, 0, 0),
            ('ep:level2:0', 'ettle:level2', 0, 1, NULL, NULL, 'Level2 EP0', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = snapshot_commit(
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

    assert!(!result.snapshot_id.is_empty());

    // Verify manifest was created successfully
    let manifest_json = cas.read(&result.manifest_digest).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_json).unwrap();
    let ept = manifest.get("ept").unwrap().as_array().unwrap();

    // Written to code: compute_ept returns EPT for the leaf ettle found from root
    // In a 3-level tree with single leaf path, EPT includes all EPs from root to leaf
    assert!(!ept.is_empty(), "EPT should contain at least one EP");

    // Verify root_ettle_id matches
    assert_eq!(
        manifest.get("root_ettle_id").unwrap().as_str().unwrap(),
        "ettle:root"
    );
}

#[test]
fn test_snapshot_commit_error_deleted_root() {
    // Behavioral: snapshot_commit should fail if root ettle is deleted
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    // Insert deleted root ettle
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:deleted', 'Deleted', NULL, 1, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:deleted:0', 'ettle:deleted', 0, 1, NULL, NULL, 'Content', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = snapshot_commit(
        "ettle:deleted",
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
}

#[test]
fn test_snapshot_commit_multiple_children() {
    // Behavioral: snapshot_commit should handle root with multiple children
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    // Root with 2 EPs mapping to 2 different children
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES
            ('ettle:root', 'Root', NULL, 0, 0, 0, '{}'),
            ('ettle:child1', 'Child1', 'ettle:root', 0, 0, 0, '{}'),
            ('ettle:child2', 'Child2', 'ettle:root', 0, 0, 0, '{}');

        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
        VALUES
            ('ep:root:0', 'ettle:root', 0, 1, 'ettle:child1', NULL, 'Root EP0', 0, 0, 0),
            ('ep:root:1', 'ettle:root', 1, 1, 'ettle:child2', NULL, 'Root EP1', 0, 0, 0),
            ('ep:child1:0', 'ettle:child1', 0, 1, NULL, NULL, 'Child1 EP0', 0, 0, 0),
            ('ep:child2:0', 'ettle:child2', 0, 1, NULL, NULL, 'Child2 EP0', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Should fail - multiple leaves without specifying which one
    let result = snapshot_commit(
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

    // compute_ept should fail with AmbiguousLeafSelection
    assert!(result.is_err());
}

#[test]
fn test_snapshot_commit_root_with_no_eps() {
    // Behavioral: snapshot_commit should fail if ettle has no EPs (validation error)
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    // Insert ettle with no EPs (violates invariant - every ettle must have EP0)
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:no-eps', 'No EPs', NULL, 0, 0, 0, '{}');
        "#,
    )
    .unwrap();

    let result = snapshot_commit(
        "ettle:no-eps",
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
        &mut conn,
        &cas,
    );

    // Should fail during EPT computation or validation
    assert!(result.is_err());
}
