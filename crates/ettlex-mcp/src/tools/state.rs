//! Handler for `state.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult};

/// Handle `state_get_version`.
///
/// Params: `{}`
pub fn handle_state_get_version(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let _ = params;
    match apply_engine_query(
        EngineQuery::StateGetVersion,
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::StateVersion(r) = result {
                McpResult::Ok(json!({
                    "state_version": r.state_version,
                    "semantic_head_digest": r.semantic_head_digest,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
