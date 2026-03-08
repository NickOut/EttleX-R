//! Handlers for `snapshot.*` tool group.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, SnapshotRef};
use ettlex_engine::commands::read_tools::ListOptions;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};
use crate::tools::ettle::parse_list_opts;

/// Handle `snapshot.list`.
///
/// Params: `{ ettle_id?: String, limit?: u64, cursor?: String }`
pub fn handle_snapshot_list(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let ettle_id = params
        .get("ettle_id")
        .and_then(Value::as_str)
        .map(String::from);
    if let Err(e) = parse_list_opts(params) {
        return e;
    } // validates cursor

    let limit = params
        .get("limit")
        .and_then(Value::as_u64)
        .map(|n| n as usize);

    match apply_engine_query(
        EngineQuery::SnapshotList { ettle_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::SnapshotList(rows) = result {
                let cap = limit.unwrap_or(100);
                let has_more = rows.len() > cap;
                let items: Vec<Value> = rows
                    .into_iter()
                    .take(cap)
                    .map(|r| {
                        json!({
                            "snapshot_id": r.snapshot_id,
                            "root_ettle_id": r.root_ettle_id,
                            "manifest_digest": r.manifest_digest,
                            "created_at": r.created_at,
                            "status": r.status,
                        })
                    })
                    .collect();
                let mut resp = json!({ "items": items });
                if has_more {
                    // Opaque cursor: just the index for now; engine handles real cursor
                    resp["cursor"] = Value::String(
                        ettlex_engine::commands::read_tools::base64_encode(&cap.to_string()),
                    );
                }
                McpResult::Ok(resp)
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `snapshot.get`.
///
/// Params: `{ snapshot_id: String }`
pub fn handle_snapshot_get(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let snapshot_id = match params.get("snapshot_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'snapshot_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::SnapshotGet { snapshot_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::SnapshotGet(r) = result {
                McpResult::Ok(json!({
                    "snapshot_id": r.snapshot_id,
                    "root_ettle_id": r.root_ettle_id,
                    "manifest_digest": r.manifest_digest,
                    "created_at": r.created_at,
                    "status": r.status,
                    "policy_ref": r.policy_ref,
                    "profile_ref": r.profile_ref,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `snapshot.get_head`.
///
/// Params: `{ ettle_id: String }`
pub fn handle_snapshot_get_head(
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
        EngineQuery::SnapshotGetHead {
            realised_ettle_id: ettle_id,
        },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::SnapshotGetHead(digest) = result {
                match digest {
                    Some(d) => McpResult::Ok(json!({ "manifest_digest": d })),
                    None => McpResult::Ok(json!({ "manifest_digest": null })),
                }
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `snapshot.get_manifest`.
///
/// Params: `{ snapshot_id: String }`
pub fn handle_snapshot_get_manifest(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let snapshot_id = match params.get("snapshot_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'snapshot_id' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ManifestGetBySnapshot { snapshot_id },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ManifestGet(r) = result {
                let manifest_bytes = String::from_utf8_lossy(&r.manifest_bytes).to_string();
                McpResult::Ok(json!({
                    "snapshot_id": r.snapshot_id,
                    "manifest_digest": r.manifest_digest,
                    "manifest_bytes": manifest_bytes,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `snapshot.diff`.
///
/// Params: `{ a_snapshot_id: String, b_snapshot_id: String }`
pub fn handle_snapshot_diff(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let a_id = match params.get("a_snapshot_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'a_snapshot_id' param",
            ))
        }
    };
    let b_id = match params.get("b_snapshot_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'b_snapshot_id' param",
            ))
        }
    };

    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::SnapshotId(a_id),
        b_ref: SnapshotRef::SnapshotId(b_id),
    };

    match apply_engine_query(query, conn, cas, Some(policy_provider)) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::SnapshotDiff(r) = result {
                let structured =
                    serde_json::to_value(&r.structured_diff).unwrap_or(serde_json::Value::Null);
                McpResult::Ok(json!({
                    "identity": structured["identity"],
                    "human_summary": r.human_summary,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Handle `manifest_get_by_digest`.
///
/// Params: `{ manifest_digest: String }`
pub fn handle_manifest_get_by_digest(
    params: &Value,
    conn: &Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let manifest_digest = match params.get("manifest_digest").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'manifest_digest' param",
            ))
        }
    };

    match apply_engine_query(
        EngineQuery::ManifestGetByDigest { manifest_digest },
        conn,
        cas,
        Some(policy_provider),
    ) {
        Ok(result) => {
            use ettlex_engine::commands::engine_query::EngineQueryResult;
            if let EngineQueryResult::ManifestGet(r) = result {
                let manifest_json: Value =
                    serde_json::from_slice(&r.manifest_bytes).unwrap_or(Value::Null);
                McpResult::Ok(json!({
                    "snapshot_id": r.snapshot_id,
                    "manifest_digest": r.manifest_digest,
                    "semantic_manifest_digest": r.semantic_manifest_digest,
                    "manifest": manifest_json,
                }))
            } else {
                McpResult::Err(McpError::new("Internal", "unexpected result variant"))
            }
        }
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

/// Unused import suppressor for ListOptions
fn _use_list_opts(_: ListOptions) {}
