//! Handlers for `group.*` and `group_member.*` tool group — Slice 02b.
//!
//! All handlers are read-only: they call engine read handlers or store
//! queries directly, bypassing `apply_command` / `ettlex_apply`. No
//! provenance events are appended and `state_version` is never incremented.

use ettlex_core::policy_provider::PolicyProvider;
use ettlex_memory::commands::group::{handle_group_get, handle_group_list};
use ettlex_store::cas::FsStore;
use ettlex_store::model::{GroupMemberRecord, GroupRecord};
use ettlex_store::repo::SqliteRepo;
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

fn paginate_groups(
    items: Vec<GroupRecord>,
    limit: usize,
    cursor: Option<&str>,
) -> (Vec<GroupRecord>, Option<String>) {
    let sort_key = |r: &GroupRecord| format!("{}\x00{}", r.created_at, r.id);

    let after_key = cursor.and_then(decode_cursor);

    let filtered: Vec<GroupRecord> = match &after_key {
        Some(key) => items.into_iter().filter(|r| sort_key(r) > *key).collect(),
        None => items,
    };

    let has_more = filtered.len() > limit;
    let page: Vec<GroupRecord> = filtered.into_iter().take(limit).collect();
    let next_cursor = if has_more {
        page.last().map(|r| encode_cursor(&sort_key(r)))
    } else {
        None
    };
    (page, next_cursor)
}

fn paginate_members(
    items: Vec<GroupMemberRecord>,
    limit: usize,
    cursor: Option<&str>,
) -> (Vec<GroupMemberRecord>, Option<String>) {
    let sort_key = |r: &GroupMemberRecord| format!("{}\x00{}", r.created_at, r.id);

    let after_key = cursor.and_then(decode_cursor);

    let filtered: Vec<GroupMemberRecord> = match &after_key {
        Some(key) => items.into_iter().filter(|r| sort_key(r) > *key).collect(),
        None => items,
    };

    let has_more = filtered.len() > limit;
    let page: Vec<GroupMemberRecord> = filtered.into_iter().take(limit).collect();
    let next_cursor = if has_more {
        page.last().map(|r| encode_cursor(&sort_key(r)))
    } else {
        None
    };
    (page, next_cursor)
}

// ---------------------------------------------------------------------------
// handle_group_get
// ---------------------------------------------------------------------------

/// Handle `group_get`.
///
/// Params: `{ group_id: String }`
///
/// Returns all 4 fields: group_id, name, created_at, tombstoned_at.
/// Returns the record even if tombstoned.
pub fn handle_group_get_tool(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let group_id = match params.get("group_id").and_then(Value::as_str) {
        Some(s) => s.to_string(),
        None => {
            return McpResult::Err(McpError::new(MCP_INVALID_INPUT, "missing 'group_id' param"))
        }
    };

    match handle_group_get(conn, group_id) {
        Ok(ettlex_memory::commands::command::CommandResult::GroupGet { record: r }) => {
            McpResult::Ok(json!({
                "group_id": r.id,
                "name": r.name,
                "created_at": r.created_at,
                "tombstoned_at": r.tombstoned_at,
            }))
        }
        Ok(_) => McpResult::Err(McpError::new("Internal", "unexpected result variant")),
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}

// ---------------------------------------------------------------------------
// handle_group_list
// ---------------------------------------------------------------------------

/// Handle `group_list`.
///
/// Params: `{ include_tombstoned?: bool, limit?: u64, cursor?: String }`
///
/// Returns `{ items: [{ group_id, name, created_at, tombstoned_at }], cursor? }`.
pub fn handle_group_list_tool(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let include_tombstoned = params
        .get("include_tombstoned")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let limit: usize = params.get("limit").and_then(Value::as_u64).unwrap_or(100) as usize;

    let cursor = params
        .get("cursor")
        .and_then(Value::as_str)
        .map(str::to_string);

    match handle_group_list(conn, include_tombstoned) {
        Ok(ettlex_memory::commands::command::CommandResult::GroupList { items }) => {
            let (page, next_cursor) = paginate_groups(items, limit, cursor.as_deref());

            let json_items: Vec<Value> = page
                .iter()
                .map(|r| {
                    json!({
                        "group_id": r.id,
                        "name": r.name,
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

// ---------------------------------------------------------------------------
// handle_group_member_list
// ---------------------------------------------------------------------------

/// Handle `group_member_list`.
///
/// Params: `{ group_id?: String, ettle_id?: String,
///            include_tombstoned?: bool, limit?: u64, cursor?: String }`
///
/// At least one of `group_id` or `ettle_id` must be supplied.
/// Returns `{ items: [{ id, group_id, ettle_id, created_at, tombstoned_at }], cursor? }`.
///
/// Calls `SqliteRepo::list_group_members_by_filter` directly because the
/// engine handler only accepts a mandatory `group_id`; the `ettle_id`-only
/// filter is a spec gap filled at the MCP layer (Slice 02b store exception).
pub fn handle_group_member_list_tool(
    params: &Value,
    conn: &Connection,
    _cas: &FsStore,
    _policy_provider: &dyn PolicyProvider,
) -> McpResult {
    let group_id = params
        .get("group_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let ettle_id = params
        .get("ettle_id")
        .and_then(Value::as_str)
        .map(str::to_string);

    if group_id.is_none() && ettle_id.is_none() {
        return McpResult::Err(McpError::new(
            MCP_INVALID_INPUT,
            "at least one of 'group_id' or 'ettle_id' must be supplied",
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

    match SqliteRepo::list_group_members_by_filter(
        conn,
        group_id.as_deref(),
        ettle_id.as_deref(),
        include_tombstoned,
    ) {
        Ok(items) => {
            let (page, next_cursor) = paginate_members(items, limit, cursor.as_deref());

            let json_items: Vec<Value> = page
                .iter()
                .map(|r| {
                    json!({
                        "id": r.id,
                        "group_id": r.group_id,
                        "ettle_id": r.ettle_id,
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
        Err(e) => McpResult::Err(McpError::from_ex_error(e)),
    }
}
