//! MCP-layer scenario tests for `mcp_ep_update` Ettle.
//!
//! Written from spec only.  Tests S-MU-1..3 use `apply_mcp_command` directly.
//! S-MU-4 lives in `ettlex-mcp/tests/` (requires handle_apply from that crate).
//!
//! Scenario → test mapping:
//!   S-MU-1  test_mcp_ep_update_returns_ep_id          [RED until ep_id added to CommandResult]
//!   S-MU-2  test_mcp_ep_update_empty_returns_structured_error
//!   S-MU-3  test_mcp_ep_update_not_found_returns_structured_error

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::command::{apply_command, Command, CommandResult};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path))
}

fn seed_ep(conn: &Connection, ep_id: &str) {
    let ettle_id = format!("ettle:{}", ep_id);
    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('{ettle_id}', 'Test', NULL, 0, 0, 0, '{{}}');
         INSERT OR IGNORE INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                                    content_inline, deleted, created_at, updated_at)
         VALUES ('{ep_id}', '{ettle_id}', 0, 1, NULL,
                 '{{\"why\":\"why\",\"what\":\"what\",\"how\":\"how\"}}',
                 0, 0, 0);"
    ))
    .unwrap();
}

// ---------------------------------------------------------------------------
// S-MU-1: ep.update succeeds and the result carries the ep_id [RED gate]
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_ep_update_returns_ep_id() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:mu1:0";
    seed_ep(&conn, ep_id);

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let (result, _sv) = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    match result {
        CommandResult::EpUpdate {
            ep_id: returned_ep_id,
        } => {
            assert_eq!(
                returned_ep_id, ep_id,
                "Returned ep_id must match the updated EP"
            );
        }
        other => panic!(
            "Expected CommandResult::EpUpdate {{ ep_id }}, got {:?}",
            other
        ),
    }
}

// ---------------------------------------------------------------------------
// S-MU-2: ep.update with no fields returns EmptyUpdate structured error
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_ep_update_empty_returns_structured_error() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:mu2:0";
    seed_ep(&conn, ep_id);

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: None,
        what: None,
        how: None,
        title: None,
    };
    let result = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "Empty update must fail");
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ExErrorKind::EmptyUpdate,
        "Expected EmptyUpdate, got {:?}",
        err.kind()
    );
}

// ---------------------------------------------------------------------------
// S-MU-3: ep.update for unknown EP returns NotFound structured error
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_ep_update_not_found_returns_structured_error() {
    let (_dir, mut conn, cas) = setup();

    let cmd = Command::EpUpdate {
        ep_id: "ep:does-not-exist".to_string(),
        why: Some("anything".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let result = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    assert!(result.is_err(), "Update of missing EP must fail");
    let err = result.unwrap_err();
    assert_eq!(
        err.kind(),
        ExErrorKind::NotFound,
        "Expected NotFound, got {:?}",
        err.kind()
    );
}
