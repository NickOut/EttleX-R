//! Handlers for `constraint.*` tool group.

use ettlex_core::model::Constraint;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

fn constraint_to_json(c: &Constraint) -> Value {
    json!({
        "constraint_id": c.constraint_id,
        "family": c.family,
        "kind": c.kind,
        "scope": c.scope,
        "payload_json": c.payload_json,
        "payload_digest": c.payload_digest,
        "deleted_at": c.deleted_at.map(|t| t.timestamp_millis()),
    })
}

/// Handle `constraint_get`.
///
/// Params: `{ constraint_id: String }`
pub fn handle_constraint_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let constraint_id = match params.get("constraint_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'constraint_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ConstraintGet { constraint_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ConstraintGet(c) = result {
                McpResult::Ok(constraint_to_json(&c))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `constraint_list_by_family`.
///
/// Params: `{ family: String, include_tombstoned?: bool }`
pub fn handle_constraint_list_by_family(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let family = match params.get("family").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'family' param")),
    };
    let include_tombstoned = params
        .get("include_tombstoned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match apply_engine_query(
        EngineQuery::ConstraintListByFamily {
            family,
            include_tombstoned,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ConstraintListByFamily(cs) = result {
                let items: Vec<Value> = cs.iter().map(constraint_to_json).collect();
                McpResult::Ok(json!({ "items": items }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
