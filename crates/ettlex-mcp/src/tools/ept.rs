//! Handlers for `ept.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

/// Handle `ept_compute`.
///
/// Params: `{ leaf_ep_id: String }`
pub fn handle_ept_compute(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let leaf_ep_id = match params.get("leaf_ep_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'leaf_ep_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::EptCompute { leaf_ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EptCompute(r) = result {
                McpResult::Ok(json!({
                    "leaf_ep_id": r.leaf_ep_id,
                    "ept_ep_ids": r.ept_ep_ids,
                    "ept_digest": r.ept_digest,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ept_compute_decision_context`.
///
/// Params: `{ leaf_ep_id: String }`
pub fn handle_ept_compute_decision_context(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let leaf_ep_id = match params.get("leaf_ep_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'leaf_ep_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::EptComputeDecisionContext { leaf_ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EptComputeDecisionContext(r) = result {
                let by_ep: serde_json::Map<String, Value> = r
                    .by_ep
                    .iter()
                    .map(|(ep_id, decisions)| {
                        let ds: Vec<Value> = decisions
                            .iter()
                            .map(|d| {
                                json!({
                                    "decision_id": d.decision_id,
                                    "title": d.title,
                                    "status": d.status,
                                })
                            })
                            .collect();
                        (ep_id.clone(), Value::Array(ds))
                    })
                    .collect();
                let all_for_leaf: Vec<Value> = r
                    .all_for_leaf
                    .iter()
                    .map(|d| {
                        json!({
                            "decision_id": d.decision_id,
                            "title": d.title,
                            "status": d.status,
                        })
                    })
                    .collect();
                McpResult::Ok(json!({
                    "by_ep": by_ep,
                    "all_for_leaf": all_for_leaf,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
