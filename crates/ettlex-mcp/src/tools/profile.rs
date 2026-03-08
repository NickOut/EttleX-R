//! Handlers for `profile.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery};
use ettlex_engine::commands::read_tools::ListOptions;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};
use crate::tools::ettle::parse_list_opts;

/// Handle `profile.get`.
///
/// Params: `{ profile_ref: String }`
pub fn handle_profile_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let profile_ref = match params.get("profile_ref").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'profile_ref' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ProfileGet { profile_ref },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ProfileGet(r) = result {
                McpResult::Ok(json!({
                    "profile_ref": r.profile_ref,
                    "profile_digest": r.profile_digest,
                    "payload": r.payload_json,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `profile.list`.
///
/// Params: `{ limit?: u64, cursor?: String }`
pub fn handle_profile_list(
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
        EngineQuery::ProfileList(opts),
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ProfileList(page) = result {
                let items: Vec<Value> = page
                    .items
                    .iter()
                    .map(|p| {
                        json!({
                            "profile_ref": p.profile_ref,
                            "profile_digest": p.profile_digest,
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

/// Handle `profile.get_default`.
///
/// Params: `{}`
pub fn handle_profile_get_default(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let _ = params;
    match apply_engine_query(
        EngineQuery::ProfileGetDefault,
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ProfileGet(r) = result {
                McpResult::Ok(json!({
                    "profile_ref": r.profile_ref,
                    "profile_digest": r.profile_digest,
                    "payload": r.payload_json,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `profile_resolve`.
///
/// Params: `{ profile_ref?: String }`
pub fn handle_profile_resolve(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let profile_ref = params
        .get("profile_ref")
        .and_then(Value::as_str)
        .map(String::from);

    match apply_engine_query(
        EngineQuery::ProfileResolve { profile_ref },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ProfileResolve(r) = result {
                McpResult::Ok(json!({
                    "profile_ref": r.profile_ref,
                    "profile_digest": r.profile_digest,
                    "payload": r.parsed_profile,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

fn _use_opts(_: ListOptions) {}
