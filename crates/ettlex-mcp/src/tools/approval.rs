//! Handler for `approval.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

/// Handle `approval.get`.
///
/// Params: `{ approval_token: String }`
pub fn handle_approval_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let approval_token = match params.get("approval_token").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'approval_token' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ApprovalGet { approval_token },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ApprovalGet(r) = result {
                McpResult::Ok(json!({
                    "approval_token": r.approval_token,
                    "request_digest": r.request_digest,
                    "semantic_request_digest": r.semantic_request_digest,
                    "payload": r.payload_json,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
