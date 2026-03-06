//! Handlers for `ettle.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_engine::commands::read_tools::{base64_decode, ListOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_CURSOR, MCP_INVALID_INPUT};

/// Handle `ettle.get`.
///
/// Params: `{ ettle_id: String }`
pub fn handle_ettle_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ettle_id = match params.get("ettle_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'ettle_id' param"))
        }
    };

    match apply_engine_query(
        EngineQuery::EttleGet { ettle_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EttleGet(r) = result {
                McpResult::Ok(json!({
                    "ettle_id": r.ettle.id,
                    "title": r.ettle.title,
                    "parent_id": r.ettle.parent_id,
                    "ep_ids": r.ep_ids,
                    "created_at": r.ettle.created_at,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ettle.list`.
///
/// Params: `{ limit?: u64, cursor?: String }`
pub fn handle_ettle_list(
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
        EngineQuery::EttleList(opts),
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EttleList(page) = result {
                let items: Vec<Value> = page
                    .items
                    .iter()
                    .map(|e| {
                        json!({
                            "id": e.id,
                            "title": e.title,
                            "parent_id": e.parent_id,
                        })
                    })
                    .collect();
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

/// Handle `ettle.list_eps`.
///
/// Params: `{ ettle_id: String }`
pub fn handle_ettle_list_eps(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ettle_id = match params.get("ettle_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'ettle_id' param"))
        }
    };

    match apply_engine_query(
        EngineQuery::EttleListEps { ettle_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EttleListEps(eps) = result {
                let items: Vec<Value> = eps
                    .iter()
                    .map(|ep| {
                        json!({
                            "id": ep.id,
                            "ettle_id": ep.ettle_id,
                            "ordinal": ep.ordinal,
                            "normative": ep.normative,
                        })
                    })
                    .collect();
                McpResult::Ok(json!({ "items": items }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

pub(crate) fn parse_list_opts(params: &Value) -> Result<ListOptions, McpResult> {
    let limit = match params.get("limit") {
        Some(Value::Number(n)) => Some(n.as_u64().unwrap_or(100) as usize),
        Some(Value::Null) | None => None,
        _ => {
            return Err(McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "limit must be a number",
            )))
        }
    };

    // Validate cursor if present
    let cursor = match params.get("cursor") {
        Some(Value::String(s)) => {
            // Validate base64
            if base64_decode(s).is_err() {
                return Err(McpResult::Err(McpError::new(
                    MCP_INVALID_CURSOR,
                    format!("invalid cursor: '{}'", s),
                )));
            }
            Some(s.clone())
        }
        Some(Value::Null) | None => None,
        _ => {
            return Err(McpResult::Err(McpError::new(
                MCP_INVALID_CURSOR,
                "cursor must be a string",
            )))
        }
    };

    Ok(ListOptions {
        limit,
        cursor,
        ..Default::default()
    })
}
