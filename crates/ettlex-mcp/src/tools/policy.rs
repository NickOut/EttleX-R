//! Handlers for `policy.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};
use crate::tools::ettle::parse_list_opts;

/// Handle `policy.get`.
///
/// Params: `{ policy_ref: String }`
pub fn handle_policy_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let policy_ref = match params.get("policy_ref").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'policy_ref' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::PolicyRead { policy_ref },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::PolicyRead(r) = result {
                McpResult::Ok(json!({
                    "policy_ref": r.policy_ref,
                    "text": r.text,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `policy.list`.
///
/// Params: `{ limit?: u64, cursor?: String }`
pub fn handle_policy_list(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    if let Err(e) = parse_list_opts(params) {
        return e;
    }
    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(100);

    match apply_engine_query(EngineQuery::PolicyList, conn, cas, Some(policy_provider)) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::PolicyList(entries) = result {
                let items: Vec<Value> = entries
                    .into_iter()
                    .take(limit)
                    .map(|e| json!({ "policy_ref": e.policy_ref }))
                    .collect();
                McpResult::Ok(json!({ "items": items }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `policy_export`.
///
/// Params: `{ policy_ref: String, export_kind: String }`
pub fn handle_policy_export(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let policy_ref = match params.get("policy_ref").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'policy_ref' param",
            ))
        }
    };
    let export_kind = match params.get("export_kind").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'export_kind' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::PolicyExport {
            policy_ref,
            export_kind,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::PolicyExport(r) = result {
                McpResult::Ok(json!({
                    "policy_ref": r.policy_ref,
                    "export_kind": r.export_kind,
                    "text": r.text,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `policy.project_for_handoff`.
///
/// Params: `{ policy_ref: String, profile_ref?: String }`
pub fn handle_policy_project_for_handoff(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let policy_ref = match params.get("policy_ref").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'policy_ref' param",
            ))
        }
    };
    let profile_ref = params
        .get("profile_ref")
        .and_then(Value::as_str)
        .map(String::from);

    match apply_engine_query(
        EngineQuery::PolicyProjectForHandoff {
            policy_ref,
            profile_ref,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_memory::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::PolicyProjectForHandoff(r) = result {
                let projection_text = String::from_utf8_lossy(&r.projection_bytes).to_string();
                McpResult::Ok(json!({
                    "policy_ref": r.policy_ref,
                    "profile_ref": r.profile_ref,
                    "projection": projection_text,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
