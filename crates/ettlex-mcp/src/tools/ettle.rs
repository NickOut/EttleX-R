//! Handlers for `ettle.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_memory::commands::read_tools::{base64_decode, ListOptions};
use ettlex_store::cas::FsStore;
use ettlex_store::model::EttleListOpts;
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_CURSOR, MCP_INVALID_INPUT};

/// Handle `ettle.get`.
///
/// Params: `{ ettle_id: String }`
///
/// Delegates to `ettlex_memory::commands::ettle::handle_ettle_get`.
/// Returns all v2 Ettle fields: id, title, why, what, how, reasoning_link_id,
/// reasoning_link_type, created_at, updated_at, tombstoned_at.
pub fn handle_ettle_get(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ettle_id = match params.get("ettle_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'ettle_id' param"))
        }
    };

    match ettlex_memory::commands::ettle::handle_ettle_get(conn, &ettle_id) {
        Ok(r) => McpResult::Ok(json!({
            "id": r.id,
            // ettle_id is a backward-compatibility alias for id
            "ettle_id": r.id,
            "title": r.title,
            "why": r.why,
            "what": r.what,
            "how": r.how,
            "reasoning_link_id": r.reasoning_link_id,
            "reasoning_link_type": r.reasoning_link_type,
            "created_at": r.created_at,
            "updated_at": r.updated_at,
            "tombstoned_at": r.tombstoned_at,
        })),
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ettle.list`.
///
/// Params: `{ limit?: u64, cursor?: String, include_tombstoned?: bool }`
///
/// Delegates to `ettlex_memory::commands::ettle::handle_ettle_list`.
/// Returns `{ items: [{ id, title, tombstoned_at }], cursor? }`.
pub fn handle_ettle_list(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ettle_list_opts = match parse_ettle_list_opts(params) {
        Ok(o) => o,
        Err(e) => return e,
    };

    match ettlex_memory::commands::ettle::handle_ettle_list(conn, ettle_list_opts) {
        Ok(page) => {
            let items: Vec<Value> = page
                .items
                .iter()
                .map(|item| {
                    json!({
                        "id": item.id,
                        "title": item.title,
                        "tombstoned_at": item.tombstoned_at,
                    })
                })
                .collect();
            let mut resp = json!({ "items": items });
            if let Some(cursor) = page.next_cursor {
                resp["cursor"] = Value::String(cursor);
            }
            McpResult::Ok(resp)
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
            use ettlex_memory::commands::engine_query::EngineQueryResult;
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

/// Handle `ettle_list_decisions`.
///
/// Params: `{ ettle_id: String, include_eps?: bool, include_ancestors?: bool }`
pub fn handle_ettle_list_decisions(
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
    let include_eps = params
        .get("include_eps")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_ancestors = params
        .get("include_ancestors")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match apply_engine_query(
        EngineQuery::EttleListDecisions {
            ettle_id,
            include_eps,
            include_ancestors,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EttleListDecisions(ds) = result {
                let items: Vec<Value> = ds
                    .iter()
                    .map(|d| {
                        json!({
                            "decision_id": d.decision_id,
                            "title": d.title,
                            "status": d.status,
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

/// Parse `EttleListOpts` from MCP params for the `ettle.list` handler.
///
/// - `limit`: optional u64 (1..=500); defaults to 100 if absent.
/// - `cursor`: optional base64 string decoded to `EttleCursor`.
/// - `include_tombstoned`: optional bool; defaults to false.
fn parse_ettle_list_opts(params: &Value) -> Result<EttleListOpts, McpResult> {
    let limit: u32 = match params.get("limit") {
        Some(Value::Number(n)) => n.as_u64().unwrap_or(100) as u32,
        Some(Value::Null) | None => 100,
        _ => {
            return Err(McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "limit must be a number",
            )))
        }
    };

    let cursor = match params.get("cursor") {
        Some(Value::String(s)) => match SqliteRepo::decode_ettle_cursor(s) {
            Ok(c) => Some(c),
            Err(_) => {
                return Err(McpResult::Err(McpError::new(
                    MCP_INVALID_CURSOR,
                    format!("invalid cursor: '{}'", s),
                )))
            }
        },
        Some(Value::Null) | None => None,
        _ => {
            return Err(McpResult::Err(McpError::new(
                MCP_INVALID_CURSOR,
                "cursor must be a string",
            )))
        }
    };

    let include_tombstoned = params
        .get("include_tombstoned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    Ok(EttleListOpts {
        limit,
        cursor,
        include_tombstoned,
    })
}

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
