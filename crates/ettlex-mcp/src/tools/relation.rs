//! Handlers for `relation.*` tool group — Slice 02b.
//!
//! All handlers are read-only: they call engine read handlers directly,
//! bypassing `apply_command` / `ettlex_apply`. No provenance events are
//! appended and `state_version` is never incremented.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::relation::{handle_relation_get, handle_relation_list};
use ettlex_store::cas::FsStore;
use ettlex_store::model::RelationRecord;
use rusqlite::Connection;
use serde_json::{json, Value};

use crate::error::{McpError, McpResult, MCP_INVALID_INPUT};

// ---------------------------------------------------------------------------
// Pagination helpers
// ---------------------------------------------------------------------------

fn encode_cursor(key: &str) -> String {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD.encode(key.as_bytes())
}

fn decode_cursor(cursor: &str) -> Option<String> {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .ok()
        .and_then(|b| String::from_utf8(b).ok())
}

/// Apply cursor + limit pagination over a `Vec<RelationRecord>` already in
/// `(created_at ASC, id ASC)` order.  Returns `(page_items, next_cursor)`.
fn paginate_relations(
    items: Vec<RelationRecord>,
    limit: usize,
    cursor: Option<&str>,
) -> (Vec<RelationRecord>, Option<String>) {
    let sort_key = |r: &RelationRecord| format!("{}\x00{}", r.created_at, r.id);

    let after_key = cursor.and_then(decode_cursor);

    let filtered: Vec<RelationRecord> = match &after_key {
        Some(key) => items.into_iter().filter(|r| sort_key(r) > *key).collect(),
        None => items,
    };

    let has_more = filtered.len() > limit;
    let page: Vec<RelationRecord> = filtered.into_iter().take(limit).collect();
    let next_cursor = if has_more {
        page.last().map(|r| encode_cursor(&sort_key(r)))
    } else {
        None
    };
    (page, next_cursor)
}

// ---------------------------------------------------------------------------
// handle_relation_get
// ---------------------------------------------------------------------------

/// Handle `relation_get`.
///
/// Params: `{ relation_id: String }`
///
/// Returns all 7 fields: relation_id, source_ettle_id, target_ettle_id,
/// relation_type, properties_json, created_at, tombstoned_at.
/// Returns the record even if tombstoned.
pub fn handle_relation_get_tool(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let relation_id = match params.get("relation_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(
                MCP_INVALID_INPUT,
                "missing 'relation_id' param",
            ))
        }
    };

    match handle_relation_get(conn, relation_id) {
        Ok(ettlex_memory::commands::command::CommandResult::RelationGet { record: r }) => {
            McpResult::Ok(json!({
                "relation_id": r.id,
                "source_ettle_id": r.source_ettle_id,
                "target_ettle_id": r.target_ettle_id,
                "relation_type": r.relation_type,
                "properties_json": r.properties_json,
                "created_at": r.created_at,
                "tombstoned_at": r.tombstoned_at,
            }))
        }
        Ok(_) => McpResult::Err(McpError::new("Internal", "unexpected result variant")),
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

// ---------------------------------------------------------------------------
// handle_relation_list
// ---------------------------------------------------------------------------

/// Handle `relation_list`.
///
/// Params: `{ source_ettle_id?: String, target_ettle_id?: String,
///            include_tombstoned?: bool, limit?: u64, cursor?: String }`
///
/// At least one of `source_ettle_id` or `target_ettle_id` must be supplied.
/// Returns `{ items: [...], cursor? }`.
pub fn handle_relation_list_tool(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let source_ettle_id = params
        .get("source_ettle_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let target_ettle_id = params
        .get("target_ettle_id")
        .and_then(Value::as_str)
        .map(str::to_string);

    if source_ettle_id.is_none() && target_ettle_id.is_none() {
        return McpResult::Err(McpError::new(
            MCP_INVALID_INPUT,
            "at least one of 'source_ettle_id' or 'target_ettle_id' must be supplied",
        ));
    }

    let include_tombstoned = params
        .get("include_tombstoned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let limit: usize = params.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;

    let cursor = params
        .get("cursor")
        .and_then(Value::as_str)
        .map(str::to_string);

    match handle_relation_list(
        conn,
        source_ettle_id,
        target_ettle_id,
        None,
        include_tombstoned,
    ) {
        Ok(ettlex_memory::commands::command::CommandResult::RelationList { items }) => {
            let (page, next_cursor) = paginate_relations(items, limit, cursor.as_deref());

            let json_items: Vec<Value> = page
                .iter()
                .map(|r| {
                    json!({
                        "relation_id": r.id,
                        "source_ettle_id": r.source_ettle_id,
                        "target_ettle_id": r.target_ettle_id,
                        "relation_type": r.relation_type,
                        "properties_json": r.properties_json,
                        "created_at": r.created_at,
                        "tombstoned_at": r.tombstoned_at,
                    })
                })
                .collect();

            let mut resp = json!({ "items": json_items });
            if let Some(c) = next_cursor {
                resp["cursor"] = Value::String(c);
            }
            McpResult::Ok(resp)
        }
        Ok(_) => McpResult::Err(McpError::new("Internal", "unexpected result variant")),
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
