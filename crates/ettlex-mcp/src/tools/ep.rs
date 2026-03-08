//! Handler for `ep.*` tool group.

use ettlex_core::model::{Constraint, Decision, Ep};
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

fn ep_to_json(ep: &Ep) -> Value {
    json!({
        "id": ep.id,
        "ettle_id": ep.ettle_id,
        "ordinal": ep.ordinal,
        "normative": ep.normative,
        "why": ep.why,
        "what": ep.what,
        "how": ep.how,
        "child_ettle_id": ep.child_ettle_id,
    })
}

fn constraint_to_json(c: &Constraint) -> Value {
    json!({
        "constraint_id": c.constraint_id,
        "family": c.family,
        "kind": c.kind,
        "scope": c.scope,
        "payload_digest": c.payload_digest,
        "deleted_at": c.deleted_at.map(|t| t.timestamp_millis()),
    })
}

fn decision_to_json(d: &Decision) -> Value {
    json!({
        "decision_id": d.decision_id,
        "title": d.title,
        "status": d.status,
        "decision_text": d.decision_text,
        "tombstoned_at": d.tombstoned_at.map(|t| t.timestamp_millis()),
    })
}

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
                McpResult::Ok(ep_to_json(&ep))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ep_list_children`.
///
/// Params: `{ ep_id: String }`
pub fn handle_ep_list_children(
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
        EngineQuery::EpListChildren { ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EpListChildren(eps) = result {
                McpResult::Ok(json!({ "items": eps.iter().map(ep_to_json).collect::<Vec<_>>() }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ep_list_parents`.
///
/// Params: `{ ep_id: String }`
pub fn handle_ep_list_parents(
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
        EngineQuery::EpListParents { ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EpListParents(eps) = result {
                McpResult::Ok(json!({ "items": eps.iter().map(ep_to_json).collect::<Vec<_>>() }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ep_list_constraints`.
///
/// Params: `{ ep_id: String }`
pub fn handle_ep_list_constraints(
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
        EngineQuery::EpListConstraints { ep_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EpListConstraints(cs) = result {
                McpResult::Ok(
                    json!({ "items": cs.iter().map(constraint_to_json).collect::<Vec<_>>() }),
                )
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `ep_list_decisions`.
///
/// Params: `{ ep_id: String, include_ancestors?: bool }`
pub fn handle_ep_list_decisions(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ep_id = match params.get("ep_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'ep_id' param")),
    };
    let include_ancestors = params
        .get("include_ancestors")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match apply_engine_query(
        EngineQuery::EpListDecisions {
            ep_id,
            include_ancestors,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::EpListDecisions(ds) = result {
                McpResult::Ok(
                    json!({ "items": ds.iter().map(decision_to_json).collect::<Vec<_>>() }),
                )
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
