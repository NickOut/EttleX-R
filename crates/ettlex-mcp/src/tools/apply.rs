//! Handler for `ettlex.apply` — dispatches write commands via `apply_command`.

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::{apply_command, Command, CommandResult};
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

    let cmd: Command = match serde_json::from_value(cmd_value.clone()) {
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
    match apply_command(
        cmd,
        expected_sv,
        conn,
        cas,
        policy_provider,
        approval_router,
    ) {
        Ok((result, new_sv)) => {
            let result_json = command_result_to_json(&result);
            McpResult::Ok(json!({
                "new_state_version": new_sv,
                "result": result_json,
            }))
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

fn command_result_to_json(r: &CommandResult) -> Value {
    match r {
        CommandResult::SnapshotCommit {
            snapshot_id,
            manifest_digest,
        } => json!({
            "tag": "SnapshotCommit",
            "snapshot_id": snapshot_id,
            "manifest_digest": manifest_digest,
        }),
        CommandResult::RoutedForApproval { approval_token } => json!({
            "tag": "RoutedForApproval",
            "approval_token": approval_token,
        }),
        CommandResult::EttleCreate { ettle_id } => json!({
            "tag": "EttleCreate",
            "ettle_id": ettle_id,
        }),
        CommandResult::ProfileCreate => json!({ "tag": "ProfileCreate" }),
        CommandResult::ProfileSetDefault => json!({ "tag": "ProfileSetDefault" }),
        CommandResult::PolicyCreate { policy_ref } => {
            json!({ "tag": "PolicyCreate", "policy_ref": policy_ref })
        }
        CommandResult::EttleUpdate => json!({ "tag": "EttleUpdate" }),
        CommandResult::EttleTombstone => json!({ "tag": "EttleTombstone" }),
        CommandResult::RelationCreate { relation_id } => {
            json!({ "tag": "RelationCreate", "relation_id": relation_id })
        }
        CommandResult::RelationUpdate => json!({ "tag": "RelationUpdate" }),
        CommandResult::RelationGet { record } => {
            json!({ "tag": "RelationGet", "record": record })
        }
        CommandResult::RelationList { items } => {
            json!({ "tag": "RelationList", "items": items })
        }
        CommandResult::RelationTombstone => json!({ "tag": "RelationTombstone" }),
        CommandResult::GroupCreate { group_id } => {
            json!({ "tag": "GroupCreate", "group_id": group_id })
        }
        CommandResult::GroupGet { record } => {
            json!({ "tag": "GroupGet", "record": record })
        }
        CommandResult::GroupList { items } => {
            json!({ "tag": "GroupList", "items": items })
        }
        CommandResult::GroupTombstone => json!({ "tag": "GroupTombstone" }),
        CommandResult::GroupMemberAdd => json!({ "tag": "GroupMemberAdd" }),
        CommandResult::GroupMemberRemove => json!({ "tag": "GroupMemberRemove" }),
        CommandResult::GroupMemberList { items } => {
            json!({ "tag": "GroupMemberList", "items": items })
        }
    }
}
