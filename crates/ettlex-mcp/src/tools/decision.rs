//! Handlers for `decision.*` tool group.

use ettlex_core::model::Decision;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};
use crate::tools::ettle::parse_list_opts;

fn decision_to_json(d: &Decision) -> Value {
    json!({
        "decision_id": d.decision_id,
        "title": d.title,
        "status": d.status,
        "decision_text": d.decision_text,
        "tombstoned_at": d.tombstoned_at.map(|t| t.timestamp_millis()),
    })
}

/// Handle `decision_get`.
///
/// Params: `{ decision_id: String }`
pub fn handle_decision_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let decision_id = match params.get("decision_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'decision_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::DecisionGet { decision_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::DecisionGet(d) = result {
                McpResult::Ok(decision_to_json(&d))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `decision_list`.
///
/// Params: `{ limit?: u64, cursor?: String }`
pub fn handle_decision_list(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let opts = match parse_list_opts(params) {
        Ok(o) => o,
        Err(e) => return e,
    };

    match apply_engine_query(
        EngineQuery::DecisionList(opts),
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::DecisionList(page) = result {
                let items: Vec<Value> = page.items.iter().map(decision_to_json).collect();
                let mut resp = json!({ "items": items });
                if let Some(cursor) = page.cursor {
                    resp["cursor"] = Value::String(cursor);
                }
                McpResult::Ok(resp)
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `decision_list_by_target`.
///
/// Params: `{ target_kind: String, target_id: String, include_tombstoned?: bool }`
pub fn handle_decision_list_by_target(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let target_kind = match params.get("target_kind").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'target_kind' param",
            ))
        }
    };
    let target_id = match params.get("target_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'target_id' param",
            ))
        }
    };
    let include_tombstoned = params
        .get("include_tombstoned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match apply_engine_query(
        EngineQuery::DecisionListByTarget {
            target_kind,
            target_id,
            include_tombstoned,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::DecisionListByTarget(ds) = result {
                let items: Vec<Value> = ds.iter().map(decision_to_json).collect();
                McpResult::Ok(json!({ "items": items }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
