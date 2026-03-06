//! Handler for `ettlex.apply` — dispatches write commands via `apply_mcp_command`.

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand, McpCommandResult};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_COMMAND, MCP_INVALID_INPUT};

/// Handle `ettlex.apply`.
///
/// Params: `{ command: {...}, expected_state_version?: u64 }`
pub fn handle_apply(
    params: &Value,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> McpResult {
    // Parse expected_state_version (optional)
    let expected_sv: Option<u64> = match params.get("expected_state_version") {
        Some(Value::Number(n)) => match n.as_u64() {
            Some(v) => Some(v),
            None => {
                return McpResult::Err(McpError::new(
                    MCP_INVALID_INPUT,
                    "expected_state_version must be a non-negative integer",
                ))
            }
        },
        Some(Value::Null) | None => None,
        _ => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "expected_state_version must be a non-negative integer",
            ))
        }
    };

    // Deserialize command
    let cmd_value = match params.get("command") {
        Some(v) => v,
        None => return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'command' field")),
    };

    let cmd: McpCommand = match serde_json::from_value(cmd_value.clone()) {
        Ok(c) => c,
        Err(e) => {
            let msg = e.to_string();
            // Unknown tag → InvalidCommand; missing fields → InvalidInput
            let code = if msg.contains("unknown variant") || msg.contains("unknown tag") {
                MCP_INVALID_COMMAND
            } else {
                MCP_INVALID_INPUT
            };
            return McpResult::Err(McpError::new(code, msg));
        }
    };

    // Dispatch
    match apply_mcp_command(
        cmd,
        expected_sv,
        conn,
        cas,
        policy_provider,
        approval_router,
    ) {
        Ok((result, new_sv)) => {
            let result_json = mcp_command_result_to_json(&result);
            McpResult::Ok(json!({
                "new_state_version": new_sv,
                "result": result_json,
            }))
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

fn mcp_command_result_to_json(r: &McpCommandResult) -> Value {
    match r {
        McpCommandResult::SnapshotCommit {
            snapshot_id,
            manifest_digest,
        } => json!({
            "tag": "SnapshotCommit",
            "snapshot_id": snapshot_id,
            "manifest_digest": manifest_digest,
        }),
        McpCommandResult::RoutedForApproval { approval_token } => json!({
            "tag": "RoutedForApproval",
            "approval_token": approval_token,
        }),
        McpCommandResult::EttleCreate { ettle_id } => json!({
            "tag": "EttleCreate",
            "ettle_id": ettle_id,
        }),
        McpCommandResult::EpCreate { ep_id } => json!({
            "tag": "EpCreate",
            "ep_id": ep_id,
        }),
        McpCommandResult::ConstraintCreate { constraint_id } => json!({
            "tag": "ConstraintCreate",
            "constraint_id": constraint_id,
        }),
        McpCommandResult::ConstraintAttachToEp => json!({ "tag": "ConstraintAttachToEp" }),
        McpCommandResult::ProfileCreate => json!({ "tag": "ProfileCreate" }),
        McpCommandResult::ProfileSetDefault => json!({ "tag": "ProfileSetDefault" }),
    }
}
