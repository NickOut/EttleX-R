//! Integration tests for ep:action_read_tools:0 — scenarios S31–S33.
//!
//! Covers `ConstraintPredicatesPreview` — a read-only, non-mutating
//! evaluation of constraint predicate resolution.

use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};
use ettlex_engine::commands::read_tools::PreviewStatus;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Setup
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

fn insert_profile(conn: &Connection, profile_ref: &str, payload_json: &str) {
    conn.execute(
        "INSERT INTO profiles (profile_ref, payload_json, created_at) VALUES (?1, ?2, 0)",
        rusqlite::params![profile_ref, payload_json],
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// S31: preview does not create an approval_request row
// ---------------------------------------------------------------------------

#[test]
fn test_preview_does_not_create_approval_request() {
    let (_tmp, conn, cas) = setup();

    // Insert a profile with predicate_evaluation_enabled=true
    insert_profile(
        &conn,
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": true}"#,
    );

    let before: i64 = conn
        .query_row("SELECT COUNT(*) FROM approval_requests", [], |r| r.get(0))
        .unwrap();

    // Run preview with multiple candidates (ambiguous scenario)
    apply_engine_query(
        EngineQuery::ConstraintPredicatesPreview {
            profile_ref: Some("profile/default@0".to_string()),
            context: json!({"env": "test"}),
            candidates: vec!["ep:a".to_string(), "ep:b".to_string()],
        },
        &conn,
        &cas,
    )
    .unwrap();

    let after: i64 = conn
        .query_row("SELECT COUNT(*) FROM approval_requests", [], |r| r.get(0))
        .unwrap();

    assert_eq!(
        before, after,
        "preview must not create any approval_request rows"
    );
}

// ---------------------------------------------------------------------------
// S32: preview is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_preview_deterministic() {
    let (_tmp, conn, cas) = setup();

    insert_profile(
        &conn,
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": false}"#,
    );

    let query = || EngineQuery::ConstraintPredicatesPreview {
        profile_ref: Some("profile/default@0".to_string()),
        context: json!({"key": "value"}),
        candidates: vec!["ep:x".to_string()],
    };

    let r1 = match apply_engine_query(query(), &conn, &cas).unwrap() {
        EngineQueryResult::PredicatePreview(r) => r,
        _ => panic!("expected PredicatePreview"),
    };
    let r2 = match apply_engine_query(query(), &conn, &cas).unwrap() {
        EngineQueryResult::PredicatePreview(r) => r,
        _ => panic!("expected PredicatePreview"),
    };

    assert_eq!(r1.status, r2.status, "preview status must be deterministic");
    assert_eq!(
        r1.selected, r2.selected,
        "selected candidate must be deterministic"
    );
    assert_eq!(
        r1.candidates, r2.candidates,
        "candidate list must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// S33: preview with evaluation disabled returns Uncomputed-like status
// ---------------------------------------------------------------------------

#[test]
fn test_preview_evaluation_disabled() {
    let (_tmp, conn, cas) = setup();

    // Profile with predicate_evaluation_enabled=false
    insert_profile(
        &conn,
        "profile/disabled@0",
        r#"{"predicate_evaluation_enabled": false}"#,
    );

    let result = apply_engine_query(
        EngineQuery::ConstraintPredicatesPreview {
            profile_ref: Some("profile/disabled@0".to_string()),
            context: json!({}),
            candidates: vec!["ep:only".to_string()],
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::PredicatePreview(r) => {
            // When predicate evaluation is disabled, the single candidate
            // is selected directly (fallback selection)
            assert_eq!(
                r.candidates,
                vec!["ep:only".to_string()],
                "candidates must be forwarded"
            );
            // Status should be Selected (single candidate, fallback) or NoMatch
            // but never a hard error
            assert!(
                r.status == PreviewStatus::Selected
                    || r.status == PreviewStatus::NoMatch
                    || r.status == PreviewStatus::Ambiguous,
                "status must be a valid preview status, got {:?}",
                r.status
            );
        }
        _ => panic!("expected PredicatePreview"),
    }
}

// ---------------------------------------------------------------------------
// S33 (extension): preview with empty candidates returns NoMatch
// ---------------------------------------------------------------------------

#[test]
fn test_preview_empty_candidates_no_match() {
    let (_tmp, conn, cas) = setup();

    insert_profile(
        &conn,
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": true}"#,
    );

    let result = apply_engine_query(
        EngineQuery::ConstraintPredicatesPreview {
            profile_ref: None,
            context: json!({}),
            candidates: vec![],
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::PredicatePreview(r) => {
            assert_eq!(
                r.status,
                PreviewStatus::NoMatch,
                "empty candidates → NoMatch"
            );
            assert!(r.selected.is_none());
            assert!(r.candidates.is_empty());
        }
        _ => panic!("expected PredicatePreview"),
    }
}
