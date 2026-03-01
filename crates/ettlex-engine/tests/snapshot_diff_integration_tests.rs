//! Integration tests for ep:snapshot_diff:0 — 5 scenarios.
//!
//! All tests use a real SQLite DB + FsStore (via TempDir).

use ettlex_engine::commands::engine_query::{
    apply_engine_query, EngineQuery, EngineQueryResult, SnapshotRef,
};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn setup() -> (TempDir, Connection, FsStore) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");

    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);

    (temp_dir, conn, cas)
}

/// Insert a minimal flat store (one ettle, one EP) and commit a snapshot.
/// Returns `(snapshot_id, manifest_digest)`.
fn commit_one_snapshot(conn: &mut Connection, cas: &FsStore) -> (String, String) {
    conn.execute_batch(
        r#"
        INSERT OR IGNORE INTO ettles
            (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');

        INSERT OR IGNORE INTO eps
            (id, ettle_id, ordinal, normative, child_ettle_id, content_digest,
             content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, NULL, 'content', 0, 0, 0);
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
            allow_dedup: false,
        },
        conn,
        cas,
    )
    .unwrap();

    (result.snapshot_id, result.manifest_digest)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

// I1: action:query is read-only (ledger unchanged after diff)
#[test]
fn test_snapshot_diff_is_read_only() {
    let (_tmp, mut conn, cas) = setup();
    let (snap_id, _) = commit_one_snapshot(&mut conn, &cas);

    let before: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::SnapshotId(snap_id.clone()),
        b_ref: SnapshotRef::SnapshotId(snap_id.clone()),
    };
    apply_engine_query(query, &conn, &cas).unwrap();

    let after: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();

    assert_eq!(
        before, after,
        "ledger row count must be unchanged after diff"
    );
}

// I2: Resolves snapshot_id → manifest_digest deterministically
#[test]
fn test_snapshot_diff_resolves_snapshot_id() {
    let (_tmp, mut conn, cas) = setup();
    let (snap_id, manifest_digest) = commit_one_snapshot(&mut conn, &cas);

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::SnapshotId(snap_id.clone()),
        b_ref: SnapshotRef::SnapshotId(snap_id.clone()),
    };
    let result = apply_engine_query(query, &conn, &cas).unwrap();

    match result {
        EngineQueryResult::SnapshotDiff(r) => {
            // Both sides resolve to the same manifest → Identical
            use ettlex_core::diff::model::DiffClassification;
            assert_eq!(
                r.structured_diff.classification,
                DiffClassification::Identical,
                "diffing a snapshot against itself must be Identical"
            );
            // The identity digests must match (both sides are the same snapshot)
            assert_eq!(
                r.structured_diff.identity.a_manifest_digest,
                r.structured_diff.identity.b_manifest_digest
            );
            // The manifest_digest recorded in the identity must be non-empty
            assert!(!r.structured_diff.identity.a_manifest_digest.is_empty());
            // The commit's manifest_digest is the CAS key, which is used to resolve;
            // verify it is non-empty and consistent
            assert!(!manifest_digest.is_empty());
            assert!(!r.human_summary.is_empty());
        }
        _ => panic!("expected EngineQueryResult::SnapshotDiff"),
    }
}

// I3: Missing snapshot_id → NotFound
#[test]
fn test_snapshot_diff_missing_snapshot_id() {
    let (_tmp, conn, cas) = setup();

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::SnapshotId("does-not-exist".to_string()),
        b_ref: SnapshotRef::SnapshotId("also-does-not-exist".to_string()),
    };
    let err = apply_engine_query(query, &conn, &cas).unwrap_err();
    assert_eq!(err.kind(), ettlex_core::errors::ExErrorKind::NotFound);
}

// I4: Missing manifest_digest in CAS → MissingBlob
#[test]
fn test_snapshot_diff_missing_manifest_digest_in_cas() {
    let (_tmp, conn, cas) = setup();
    let fake_digest = "a".repeat(64);

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::ManifestDigest(fake_digest.clone()),
        b_ref: SnapshotRef::ManifestDigest(fake_digest),
    };
    let err = apply_engine_query(query, &conn, &cas).unwrap_err();
    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::MissingBlob,
        "expected MissingBlob, got {:?}",
        err.kind()
    );
}

// I5: Malformed manifest bytes → InvalidManifest
#[test]
fn test_snapshot_diff_malformed_manifest_bytes() {
    let (_tmp, conn, cas) = setup();

    // Write garbage bytes directly to CAS
    let garbage = b"this is not json at all!!!";
    let digest = cas.write(garbage, "json").unwrap();

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::ManifestDigest(digest.clone()),
        b_ref: SnapshotRef::ManifestDigest(digest),
    };
    let err = apply_engine_query(query, &conn, &cas).unwrap_err();
    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::InvalidManifest,
        "expected InvalidManifest, got {:?}",
        err.kind()
    );
}
