//! MCP tool-layer test for `mcp_ep_update` Ettle.
//!
//! Tests involving `handle_apply` live here (requires ettlex-mcp).
//!
//! Scenario → test mapping:
//!   S-MU-4  test_mcp_ep_update_tool_has_no_validation_logic

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_mcp::error::McpResult;
use ettlex_mcp::tools::apply::handle_apply;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::json;
use tempfile::TempDir;

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
// S-MU-4: The MCP tool handler (handle_apply) has no field-presence validation
//
// Structural proof: `tools/apply.rs::handle_apply` deserialises the JSON command
// and delegates immediately to `apply_mcp_command` (and through it, to ep_ops).
// It performs no EP field presence checks of its own.
// All domain validation lives in the action layer (ep_ops::update_ep).
//
// Evidence: sending an empty-field EpUpdate through `handle_apply` produces an
// error with the EmptyUpdate code — which originates from ep_ops, not from any
// MCP-tool-level guard.
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_ep_update_tool_has_no_validation_logic() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:mu4:0";
    seed_ep(&conn, ep_id);

    // Send via handle_apply — the MCP transport layer
    let params = json!({
        "command": {
            "tag": "EpUpdate",
            "ep_id": ep_id
            // why / what / how / title all absent (serde defaults to None)
        }
    });

    let result = handle_apply(
        &params,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    // handle_apply must return an error because the action layer rejects empty updates.
    // The error code must include "EmptyUpdate" — proving the origin is ep_ops, not the tool.
    match result {
        McpResult::Err(mcp_err) => {
            assert!(
                mcp_err.error_code.contains("EmptyUpdate"),
                "Error code must indicate EmptyUpdate (action-layer origin); got: {}",
                mcp_err.error_code
            );
        }
        McpResult::Ok(_) => {
            panic!("Empty EpUpdate via handle_apply must return an error, not Ok")
        }
    }
}
