//! MCP tool-layer test for `mcp_ep_update` Ettle.
//!
//! EP construct retired in Slice 03. The EpUpdate command variant no longer exists.
//! These tests are retired; Slice 04 will remove the seed module entirely.
//!
//! Scenario → test mapping:
//!   S-MU-4  test_mcp_ep_update_tool_retired (verifies EpUpdate returns InvalidCommand)

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

// ---------------------------------------------------------------------------
// S-MU-4: EpUpdate is retired in Slice 03 — handle_apply must return InvalidCommand
// ---------------------------------------------------------------------------

#[test]
fn test_mcp_ep_update_tool_retired() {
    let (_dir, mut conn, cas) = setup();

    // Send EpUpdate via handle_apply — the command no longer exists
    let params = json!({
        "command": {
            "tag": "EpUpdate",
            "ep_id": "ep:some:0"
        }
    });

    let result = handle_apply(
        &params,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    // handle_apply must return an error because EpUpdate is an unknown command variant
    match result {
        McpResult::Err(mcp_err) => {
            assert!(
                mcp_err.error_code.contains("InvalidCommand"),
                "Error code must indicate InvalidCommand for retired EpUpdate; got: {}",
                mcp_err.error_code
            );
        }
        McpResult::Ok(_) => {
            panic!("EpUpdate (retired) via handle_apply must return an error, not Ok")
        }
    }
}
