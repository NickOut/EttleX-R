// Test suite for snapshot persistence
// Tests CAS storage, ledger entries, atomic commits, and idempotency

use ettlex_core::snapshot::manifest::generate_manifest;
use ettlex_store::cas::FsStore;
use ettlex_store::snapshot::persist::{commit_snapshot, persist_manifest_to_cas, SnapshotOptions};
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, Connection, FsStore) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();

    // Apply migrations
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();

    let cas = FsStore::new(cas_path);

    (temp_dir, conn, cas)
}

fn create_test_manifest() -> ettlex_core::snapshot::manifest::SnapshotManifest {
    let ept = vec!["ep:root:0".into(), "ep:root:1".into()];
    generate_manifest(
        ept,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
    )
    .unwrap()
}

#[test]
fn test_persist_manifest_to_cas() {
    let (_temp_dir, _conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    // Persist manifest to CAS
    let digest = persist_manifest_to_cas(&cas, &manifest).unwrap();

    // Digest should be 64 characters (SHA256 hex)
    assert_eq!(digest.len(), 64);

    // Should be able to read it back
    let content = cas.read(&digest).unwrap();
    let restored: ettlex_core::snapshot::manifest::SnapshotManifest =
        serde_json::from_slice(&content).unwrap();

    assert_eq!(restored.policy_ref, manifest.policy_ref);
    assert_eq!(restored.root_ettle_id, manifest.root_ettle_id);
}

#[test]
fn test_persist_manifest_to_cas_idempotent() {
    let (_temp_dir, _conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    // Persist twice
    let digest1 = persist_manifest_to_cas(&cas, &manifest).unwrap();
    let digest2 = persist_manifest_to_cas(&cas, &manifest).unwrap();

    // Same digest both times
    assert_eq!(digest1, digest2);
}

#[test]
fn test_commit_snapshot_happy_path() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    let result = commit_snapshot(
        &mut conn,
        &cas,
        manifest.clone(),
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    )
    .unwrap();

    // Should return snapshot ID
    assert!(!result.snapshot_id.is_empty());
    assert!(!result.manifest_digest.is_empty());
    assert_eq!(
        result.semantic_manifest_digest,
        manifest.semantic_manifest_digest
    );

    // Verify ledger entry exists
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE snapshot_id = ?1",
            [&result.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);

    // Verify manifest is in CAS using the returned digest
    let cas_content = cas.read(&result.manifest_digest).unwrap();
    assert!(!cas_content.is_empty());
}

#[test]
fn test_commit_snapshot_idempotent() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    // Commit twice
    let result1 = commit_snapshot(
        &mut conn,
        &cas,
        manifest.clone(),
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    )
    .unwrap();

    // Create a new manifest with different timestamp (same semantic content)
    std::thread::sleep(std::time::Duration::from_millis(10));
    let manifest2 = create_test_manifest();
    assert_ne!(manifest.created_at, manifest2.created_at); // Different timestamps
    assert_eq!(
        manifest.semantic_manifest_digest,
        manifest2.semantic_manifest_digest
    ); // Same semantic digest

    let result2 = commit_snapshot(
        &mut conn,
        &cas,
        manifest2,
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
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
fn test_commit_snapshot_expected_head_success() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest1 = create_test_manifest();

    // First commit
    let result1 = commit_snapshot(
        &mut conn,
        &cas,
        manifest1,
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    )
    .unwrap();

    // Second commit with correct expected head
    std::thread::sleep(std::time::Duration::from_millis(10));
    let ept2 = vec!["ep:root:0".into(), "ep:root:1".into(), "ep:root:2".into()];
    let manifest2 = generate_manifest(
        ept2,
        "policy/default@0".into(),
        "profile/default@0".into(),
        "ettle:root".into(),
        "0001".into(),
        None,
    )
    .unwrap();

    let result2 = commit_snapshot(
        &mut conn,
        &cas,
        manifest2,
        SnapshotOptions {
            expected_head: Some(result1.snapshot_id.clone()),
            dry_run: false,
        },
    )
    .unwrap();

    // Should succeed and create new snapshot
    assert_ne!(result1.snapshot_id, result2.snapshot_id);

    // Second snapshot should reference first as parent
    let parent_id: Option<String> = conn
        .query_row(
            "SELECT parent_snapshot_id FROM snapshots WHERE snapshot_id = ?1",
            [&result2.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(parent_id, Some(result1.snapshot_id));
}

#[test]
fn test_commit_snapshot_expected_head_mismatch() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    let result = commit_snapshot(
        &mut conn,
        &cas,
        manifest,
        SnapshotOptions {
            expected_head: Some("nonexistent-snapshot-id".into()),
            dry_run: false,
        },
    );

    // Should fail with concurrency error
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind(), ettlex_core::ExErrorKind::Concurrency);
}

#[test]
fn test_commit_snapshot_dry_run() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    let result = commit_snapshot(
        &mut conn,
        &cas,
        manifest.clone(),
        SnapshotOptions {
            expected_head: None,
            dry_run: true,
        },
    )
    .unwrap();

    // Should return computed digests
    assert_eq!(result.manifest_digest, manifest.manifest_digest);
    assert_eq!(
        result.semantic_manifest_digest,
        manifest.semantic_manifest_digest
    );

    // Should NOT persist to ledger
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);

    // Should NOT persist to CAS
    assert!(cas.read(&manifest.manifest_digest).is_err());
}

#[test]
fn test_commit_snapshot_atomic() {
    let (_temp_dir, mut conn, cas) = setup_test_env();
    let manifest = create_test_manifest();

    // Commit successfully
    let result = commit_snapshot(
        &mut conn,
        &cas,
        manifest.clone(),
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
        },
    )
    .unwrap();

    // Both CAS and ledger should have entries
    let ledger_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(ledger_count, 1);

    let cas_exists = cas.read(&result.manifest_digest).is_ok();
    assert!(cas_exists);

    // Verify the data matches
    let stored_digest: String = conn
        .query_row(
            "SELECT manifest_digest FROM snapshots WHERE snapshot_id = ?1",
            [&result.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_digest, result.manifest_digest);
}
