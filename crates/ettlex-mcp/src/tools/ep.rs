//! Handler for `ep.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

/// Handle `ep.get`.
///
/// Params: `{ ep_id: String }`
pub fn handle_ep_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ep_id = match params.get("ep_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'ep_id' param")),
    };

    match apply_engine_query(
        EngineQuery::EpGet { ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EpGet(ep) = result {
                McpResult::Ok(json!({
                    "id": ep.id,
                    "ettle_id": ep.ettle_id,
                    "ordinal": ep.ordinal,
                    "normative": ep.normative,
                    "why": ep.why,
                    "what": ep.what,
                    "how": ep.how,
                    "child_ettle_id": ep.child_ettle_id,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
