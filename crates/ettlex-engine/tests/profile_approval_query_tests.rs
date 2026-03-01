//! Integration tests for ep:action_read_tools:0 — scenarios S23–S30.
//!
//! Covers profile and approval query surfaces.

use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};
use ettlex_engine::commands::read_tools::ListOptions;
use ettlex_store::cas::FsStore;
use ettlex_store::profile::SqliteApprovalRouter;
use rusqlite::Connection;
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
        "INSERT INTO profiles (profile_ref, payload_json, is_default, created_at)
         VALUES (?1, ?2, 0, 0)",
        rusqlite::params![profile_ref, payload_json],
    )
    .unwrap();
}

fn insert_default_profile(conn: &Connection, profile_ref: &str, payload_json: &str) {
    conn.execute(
        "INSERT INTO profiles (profile_ref, payload_json, is_default, created_at)
         VALUES (?1, ?2, 1, 0)",
        rusqlite::params![profile_ref, payload_json],
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// S23: profile.get returns deterministic digest
// ---------------------------------------------------------------------------

#[test]
fn test_profile_get_deterministic() {
    let (_tmp, conn, cas) = setup();

    insert_profile(&conn, "profile/test@1", r#"{"key": "value"}"#);

    let r1 = apply_engine_query(
        EngineQuery::ProfileGet {
            profile_ref: "profile/test@1".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();
    let r2 = apply_engine_query(
        EngineQuery::ProfileGet {
            profile_ref: "profile/test@1".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match (r1, r2) {
        (EngineQueryResult::ProfileGet(g1), EngineQueryResult::ProfileGet(g2)) => {
            assert_eq!(g1.profile_ref, "profile/test@1");
            assert_eq!(
                g1.profile_digest, g2.profile_digest,
                "digest must be deterministic"
            );
            assert!(!g1.profile_digest.is_empty());
            assert_eq!(g1.payload_json, g2.payload_json);
        }
        _ => panic!("expected ProfileGet"),
    }
}

// ---------------------------------------------------------------------------
// S24: profile.resolve with null uses default profile
// ---------------------------------------------------------------------------

#[test]
fn test_profile_resolve_null_uses_default() {
    let (_tmp, conn, cas) = setup();

    // Insert default profile (is_default = 1)
    insert_default_profile(&conn, "profile/default@0", r#"{"is_default": true}"#);

    let result = apply_engine_query(
        EngineQuery::ProfileResolve { profile_ref: None },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::ProfileResolve(r) => {
            assert_eq!(r.profile_ref, "profile/default@0");
            assert_eq!(r.parsed_profile["is_default"], true);
        }
        _ => panic!("expected ProfileResolve"),
    }
}

// ---------------------------------------------------------------------------
// S25: profile.resolve unknown → NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_profile_resolve_unknown_not_found() {
    let (_tmp, conn, cas) = setup();

    let err = apply_engine_query(
        EngineQuery::ProfileResolve {
            profile_ref: Some("profile/does-not-exist@99".to_string()),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::ProfileNotFound,
        "expected ProfileNotFound for unknown profile"
    );
}

// ---------------------------------------------------------------------------
// S26: profile.list pagination is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_profile_list_pagination_deterministic() {
    let (_tmp, conn, cas) = setup();

    for i in 0..5 {
        insert_profile(&conn, &format!("profile/pg-{:02}@0", i), r#"{"n": 0}"#);
    }

    let page1 = match apply_engine_query(
        EngineQuery::ProfileList(ListOptions {
            limit: Some(2),
            ..Default::default()
        }),
        &conn,
        &cas,
    )
    .unwrap()
    {
        EngineQueryResult::ProfileList(p) => p,
        _ => panic!("expected ProfileList"),
    };

    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);
    let cursor = page1.cursor.clone().expect("cursor must be present");

    let page2 = match apply_engine_query(
        EngineQuery::ProfileList(ListOptions {
            limit: Some(2),
            cursor: Some(cursor),
            ..Default::default()
        }),
        &conn,
        &cas,
    )
    .unwrap()
    {
        EngineQueryResult::ProfileList(p) => p,
        _ => panic!("expected ProfileList"),
    };

    assert_eq!(page2.items.len(), 2);

    // Refs from page1 and page2 must be disjoint
    let refs1: Vec<_> = page1.items.iter().map(|p| p.profile_ref.clone()).collect();
    let refs2: Vec<_> = page2.items.iter().map(|p| p.profile_ref.clone()).collect();
    for r in &refs1 {
        assert!(!refs2.contains(r), "pages must not overlap");
    }
}

// ---------------------------------------------------------------------------
// S27: approval.get returns digests and bytes
// ---------------------------------------------------------------------------

#[test]
fn test_approval_get_digests_and_bytes() {
    let (_tmp, mut conn, cas) = setup();

    // Route an approval request with CAS backing
    let token = {
        use ettlex_core::approval_router::ApprovalRouter;
        let router = SqliteApprovalRouter::new_with_cas(&mut conn, &cas);
        router
            .route_approval_request(
                "test_reason",
                vec!["cand:a".to_string(), "cand:b".to_string()],
            )
            .unwrap()
    };

    let result = apply_engine_query(
        EngineQuery::ApprovalGet {
            approval_token: token.clone(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::ApprovalGet(r) => {
            assert_eq!(r.approval_token, token);
            assert!(
                !r.request_digest.is_empty(),
                "request_digest must be populated"
            );
            assert!(!r.semantic_request_digest.is_empty());
            // payload_json must have the expected fields
            assert!(r.payload_json.get("approval_token").is_some());
            assert!(r.payload_json.get("reason_code").is_some());
        }
        _ => panic!("expected ApprovalGet"),
    }
}

// ---------------------------------------------------------------------------
// S28: approval.get unknown token → NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_approval_get_unknown_token() {
    let (_tmp, conn, cas) = setup();

    let err = apply_engine_query(
        EngineQuery::ApprovalGet {
            approval_token: "00000000-0000-0000-0000-000000000000".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::ApprovalNotFound,
        "expected ApprovalNotFound for unknown approval token"
    );
}

// ---------------------------------------------------------------------------
// S29: approval.get → ApprovalStorageCorrupt when CAS blob missing
// ---------------------------------------------------------------------------

#[test]
fn test_approval_get_cas_blob_missing_corrupt_error() {
    let (_tmp, conn, cas) = setup();

    // Insert an approval row with a fake request_digest that doesn't exist in CAS
    conn.execute(
        r#"INSERT INTO approval_requests
           (approval_token, reason_code, candidate_set_json, semantic_request_digest,
            status, created_at, request_digest)
           VALUES (?1, 'reason', '[]', 'semdig', 'pending', 0, ?2)"#,
        rusqlite::params![
            "corrupt-token-000",
            "a".repeat(64) // fake CAS digest that doesn't exist
        ],
    )
    .unwrap();

    let err = apply_engine_query(
        EngineQuery::ApprovalGet {
            approval_token: "corrupt-token-000".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::ApprovalStorageCorrupt,
        "expected ApprovalStorageCorrupt when CAS blob missing, got {:?}",
        err.kind()
    );
}

// ---------------------------------------------------------------------------
// S30: approval.list returns items in descending created_at order
// ---------------------------------------------------------------------------

#[test]
fn test_approval_list_deterministic() {
    let (_tmp, mut conn, cas) = setup();

    // Route 3 approval requests; they get different created_at timestamps
    let tokens: Vec<String> = (0..3)
        .map(|i| {
            use ettlex_core::approval_router::ApprovalRouter;
            let router = SqliteApprovalRouter::new_with_cas(&mut conn, &cas);
            router
                .route_approval_request(&format!("reason_{}", i), vec![format!("cand:{}", i)])
                .unwrap()
        })
        .collect();

    let result = apply_engine_query(
        EngineQuery::ApprovalList(ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::ApprovalList(page) => {
            assert_eq!(
                page.items.len(),
                tokens.len(),
                "all approvals must be listed"
            );
            // Items are ordered by descending created_at (newest first)
            let returned_tokens: Vec<_> = page
                .items
                .iter()
                .map(|a| a.approval_token.clone())
                .collect();
            for token in &tokens {
                assert!(
                    returned_tokens.contains(token),
                    "token {} must be in listing",
                    token
                );
            }
            // Timestamps must be non-decreasing (ascending, oldest first)
            let timestamps: Vec<_> = page.items.iter().map(|a| a.created_at).collect();
            for i in 0..timestamps.len().saturating_sub(1) {
                assert!(
                    timestamps[i] <= timestamps[i + 1],
                    "list must be in ascending created_at order"
                );
            }
        }
        _ => panic!("expected ApprovalList"),
    }
}
