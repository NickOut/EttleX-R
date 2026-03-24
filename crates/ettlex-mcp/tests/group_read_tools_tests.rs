//! MCP read tool tests for Slice 02b — group_get, group_list, group_member_list.
//!
//! Scenario → test mapping:
//!   SC-S02b-19  test_s02b_group_get_returns_full_record
//!   SC-S02b-20  test_s02b_group_get_returns_tombstoned_group
//!   SC-S02b-21  test_s02b_group_get_not_found
//!   SC-S02b-22  test_s02b_group_get_does_not_use_apply_path
//!   SC-S02b-23  test_s02b_group_get_does_not_mutate_state
//!   SC-S02b-24  test_s02b_group_get_fields_match_stored_record
//!   SC-S02b-25  test_s02b_group_list_returns_active_groups
//!   SC-S02b-26  test_s02b_group_list_include_tombstoned
//!   SC-S02b-27  test_s02b_group_list_pagination_complete_non_overlapping
//!   SC-S02b-28  test_s02b_group_list_empty_when_no_groups
//!   SC-S02b-29  test_s02b_group_list_ordering_deterministic
//!   SC-S02b-30  test_s02b_group_list_does_not_mutate_state
//!   SC-S02b-31  test_s02b_group_member_list_by_group_id
//!   SC-S02b-32  test_s02b_group_member_list_by_ettle_id
//!   SC-S02b-33  test_s02b_group_member_list_by_group_and_ettle_intersection
//!   SC-S02b-34  test_s02b_group_member_list_include_tombstoned
//!   SC-S02b-35  test_s02b_group_member_list_no_filter_returns_invalid_input
//!   SC-S02b-36  test_s02b_group_member_list_empty_when_no_match
//!   SC-S02b-37  test_s02b_group_member_list_pagination_complete_non_overlapping
//!   SC-S02b-38  test_s02b_group_member_list_does_not_use_apply_path
//!   SC-S02b-39  test_s02b_group_member_list_ordering_deterministic
//!   SC-S02b-40  test_s02b_all_five_tools_registered_at_startup
//!   SC-S02b-41  test_s02b_group_member_list_does_not_mutate_state
//!   SC-S02b-42  test_s02b_no_new_tool_invokes_write_command_path

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_mcp::auth::AuthConfig;
use ettlex_mcp::context::RequestContext;
use ettlex_mcp::server::{McpResponse, McpResult, McpServer, McpToolCall};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct TestHarness {
    _tmp: TempDir,
    pub conn: Connection,
    pub cas: FsStore,
    pub server: McpServer,
}

impl TestHarness {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("test.db");
        let cas_path = tmp.path().join("cas");
        let mut conn = Connection::open(&db).unwrap();
        ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
        let cas = FsStore::new(cas_path);
        let server = McpServer::new(AuthConfig::disabled(), 1024 * 1024);
        Self {
            _tmp: tmp,
            conn,
            cas,
            server,
        }
    }

    fn call(&mut self, tool: &str, params: Value) -> McpResponse {
        let size = params.to_string().len();
        self.server.dispatch(
            McpToolCall {
                tool_name: tool.to_string(),
                params,
                context: RequestContext::default(),
                auth_token: None,
                payload_size: size,
            },
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
    }

    fn state_version(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
            .unwrap()
    }
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn create_ettle(h: &mut TestHarness, title: &str) -> String {
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate", "title": title } }),
    );
    match resp.result {
        McpResult::Ok(v) => v["result"]["ettle_id"].as_str().unwrap().to_string(),
        McpResult::Err(e) => panic!("create_ettle failed: {} — {}", e.error_code, e.message),
    }
}

fn create_group(h: &mut TestHarness, name: &str) -> String {
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "GroupCreate", "name": name } }),
    );
    match resp.result {
        McpResult::Ok(v) => v["result"]["group_id"].as_str().unwrap().to_string(),
        McpResult::Err(e) => panic!("create_group failed: {} — {}", e.error_code, e.message),
    }
}

fn tombstone_group(h: &mut TestHarness, group_id: &str) {
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "GroupTombstone", "group_id": group_id } }),
    );
    assert!(
        matches!(resp.result, McpResult::Ok(_)),
        "tombstone_group failed"
    );
}

fn add_group_member(h: &mut TestHarness, group_id: &str, ettle_id: &str) {
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "GroupMemberAdd",
                "group_id": group_id,
                "ettle_id": ettle_id
            }
        }),
    );
    assert!(
        matches!(resp.result, McpResult::Ok(_)),
        "add_group_member failed"
    );
}

fn remove_group_member(h: &mut TestHarness, group_id: &str, ettle_id: &str) {
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "GroupMemberRemove",
                "group_id": group_id,
                "ettle_id": ettle_id
            }
        }),
    );
    assert!(
        matches!(resp.result, McpResult::Ok(_)),
        "remove_group_member failed"
    );
}

fn assert_error(resp: &McpResponse, expected_code: &str) {
    match &resp.result {
        McpResult::Err(e) => assert_eq!(
            e.error_code, expected_code,
            "expected error_code '{}' but got '{}': {}",
            expected_code, e.error_code, e.message
        ),
        McpResult::Ok(v) => panic!("expected error '{}' but got Ok: {}", expected_code, v),
    }
}

fn ok_value(resp: McpResponse) -> Value {
    match resp.result {
        McpResult::Ok(v) => v,
        McpResult::Err(e) => panic!(
            "expected Ok but got error '{}': {}",
            e.error_code, e.message
        ),
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-19 — group_get returns full record for active group
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_returns_full_record() {
    let mut h = TestHarness::new();
    let gid = create_group(&mut h, "Constraints");

    let v = ok_value(h.call("group_get", json!({ "group_id": gid })));

    assert_eq!(v["group_id"].as_str().unwrap(), gid);
    assert_eq!(v["name"].as_str().unwrap(), "Constraints");
    assert!(v.get("created_at").is_some());
    assert!(v["tombstoned_at"].is_null());
}

// ---------------------------------------------------------------------------
// SC-S02b-20 — group_get returns tombstoned group
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_returns_tombstoned_group() {
    let mut h = TestHarness::new();
    let gid = create_group(&mut h, "ToBeGone");
    tombstone_group(&mut h, &gid);

    let v = ok_value(h.call("group_get", json!({ "group_id": gid })));

    assert_eq!(v["group_id"].as_str().unwrap(), gid);
    assert!(
        !v["tombstoned_at"].is_null(),
        "tombstoned_at should be non-null"
    );
    assert!(!v["tombstoned_at"].as_str().unwrap().is_empty());
}

// ---------------------------------------------------------------------------
// SC-S02b-21 — group_get returns NotFound for unknown group_id
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_not_found() {
    let mut h = TestHarness::new();
    let resp = h.call("group_get", json!({ "group_id": "grp:does-not-exist" }));
    assert_error(&resp, "NotFound");
}

// ---------------------------------------------------------------------------
// SC-S02b-22 — group_get does not use the ettlex_apply write path
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_does_not_use_apply_path() {
    let mut h = TestHarness::new();
    let gid = create_group(&mut h, "G");

    let sv_before = h.state_version();
    let _ = h.call("group_get", json!({ "group_id": gid }));
    let sv_after = h.state_version();

    assert_eq!(
        sv_before, sv_after,
        "group_get must not increment state_version"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-23 — group_get does not mutate store state
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_does_not_mutate_state() {
    let mut h = TestHarness::new();
    let gid = create_group(&mut h, "G");

    let sv = h.state_version();
    let _ = h.call("group_get", json!({ "group_id": gid }));
    let _ = h.call("group_get", json!({ "group_id": "grp:nope" }));

    assert_eq!(
        h.state_version(),
        sv,
        "state_version must not change after group_get calls"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-24 — group_get response fields match stored record byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_get_fields_match_stored_record() {
    let mut h = TestHarness::new();
    let gid = create_group(&mut h, "My Group");

    let v = ok_value(h.call("group_get", json!({ "group_id": gid })));

    assert_eq!(v["group_id"].as_str().unwrap(), gid);
    assert_eq!(v["name"].as_str().unwrap(), "My Group");
    assert!(!v["created_at"].as_str().unwrap().is_empty());
    assert!(v["tombstoned_at"].is_null());
}

// ---------------------------------------------------------------------------
// SC-S02b-25 — group_list returns all active groups (tombstoned excluded by default)
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_returns_active_groups() {
    let mut h = TestHarness::new();
    let g1 = create_group(&mut h, "G1");
    let g2 = create_group(&mut h, "G2");
    let g3 = create_group(&mut h, "G3");
    tombstone_group(&mut h, &g3);

    let v = ok_value(h.call("group_list", json!({})));
    let items = v["items"].as_array().unwrap();

    let ids: Vec<&str> = items
        .iter()
        .map(|i| i["group_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&g1.as_str()));
    assert!(ids.contains(&g2.as_str()));
    assert!(
        !ids.contains(&g3.as_str()),
        "tombstoned group should be excluded by default"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-26 — group_list with include_tombstoned returns all groups
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_include_tombstoned() {
    let mut h = TestHarness::new();
    let g1 = create_group(&mut h, "G1");
    let g2 = create_group(&mut h, "G2");
    tombstone_group(&mut h, &g2);

    let v = ok_value(h.call("group_list", json!({ "include_tombstoned": true })));
    let items = v["items"].as_array().unwrap();

    let ids: Vec<&str> = items
        .iter()
        .map(|i| i["group_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&g1.as_str()));
    assert!(
        ids.contains(&g2.as_str()),
        "tombstoned group should be included"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-27 — group_list pagination is complete and non-overlapping
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_pagination_complete_non_overlapping() {
    let mut h = TestHarness::new();
    create_group(&mut h, "Alpha");
    create_group(&mut h, "Beta");
    create_group(&mut h, "Gamma");

    // Page 1: limit 2
    let r1 = ok_value(h.call("group_list", json!({ "limit": 2 })));
    let page1_items = r1["items"].as_array().unwrap();
    assert_eq!(page1_items.len(), 2);
    let cursor = r1["cursor"]
        .as_str()
        .expect("cursor should be present after page 1");

    // Page 2
    let r2 = ok_value(h.call("group_list", json!({ "limit": 2, "cursor": cursor })));
    let page2_items = r2["items"].as_array().unwrap();
    assert_eq!(page2_items.len(), 1);
    assert!(r2["cursor"].is_null() || r2.get("cursor").is_none());

    // No overlap
    let ids1: Vec<&str> = page1_items
        .iter()
        .map(|i| i["group_id"].as_str().unwrap())
        .collect();
    let ids2: Vec<&str> = page2_items
        .iter()
        .map(|i| i["group_id"].as_str().unwrap())
        .collect();
    for id in &ids2 {
        assert!(!ids1.contains(id), "item {} appeared in both pages", id);
    }
    // Total = 3
    assert_eq!(ids1.len() + ids2.len(), 3);
}

// ---------------------------------------------------------------------------
// SC-S02b-28 — group_list returns empty list when no groups exist
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_empty_when_no_groups() {
    let mut h = TestHarness::new();
    let v = ok_value(h.call("group_list", json!({})));
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------------
// SC-S02b-29 — group_list ordering is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_ordering_deterministic() {
    let mut h = TestHarness::new();
    create_group(&mut h, "G1");
    create_group(&mut h, "G2");

    let r1 = ok_value(h.call("group_list", json!({})));
    let r2 = ok_value(h.call("group_list", json!({})));

    assert_eq!(
        r1.to_string(),
        r2.to_string(),
        "group_list must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-30 — group_list does not mutate store state
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_list_does_not_mutate_state() {
    let mut h = TestHarness::new();
    create_group(&mut h, "G1");

    let sv = h.state_version();
    let _ = h.call("group_list", json!({}));

    assert_eq!(h.state_version(), sv);
}

// ---------------------------------------------------------------------------
// SC-S02b-31 — group_member_list filtered by group_id returns all members
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_by_group_id() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e1 = create_ettle(&mut h, "E1");
    let e2 = create_ettle(&mut h, "E2");
    let e3 = create_ettle(&mut h, "E3");
    add_group_member(&mut h, &g, &e1);
    add_group_member(&mut h, &g, &e2);
    // e3 not added to g

    let v = ok_value(h.call("group_member_list", json!({ "group_id": g })));
    let items = v["items"].as_array().unwrap();

    let ettle_ids: Vec<&str> = items
        .iter()
        .map(|i| i["ettle_id"].as_str().unwrap())
        .collect();
    assert!(ettle_ids.contains(&e1.as_str()));
    assert!(ettle_ids.contains(&e2.as_str()));
    assert!(!ettle_ids.contains(&e3.as_str()));
    // Each item has required fields
    for item in items {
        assert!(item.get("group_id").is_some());
        assert!(item.get("ettle_id").is_some());
        assert!(item.get("created_at").is_some());
        assert!(item.get("tombstoned_at").is_some());
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-32 — group_member_list filtered by ettle_id returns all memberships
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_by_ettle_id() {
    let mut h = TestHarness::new();
    let g1 = create_group(&mut h, "G1");
    let g2 = create_group(&mut h, "G2");
    let g3 = create_group(&mut h, "G3");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g1, &e);
    add_group_member(&mut h, &g2, &e);
    // g3: e is NOT a member

    let v = ok_value(h.call("group_member_list", json!({ "ettle_id": e })));
    let items = v["items"].as_array().unwrap();

    let group_ids: Vec<&str> = items
        .iter()
        .map(|i| i["group_id"].as_str().unwrap())
        .collect();
    assert!(group_ids.contains(&g1.as_str()));
    assert!(group_ids.contains(&g2.as_str()));
    assert!(!group_ids.contains(&g3.as_str()));
}

// ---------------------------------------------------------------------------
// SC-S02b-33 — group_member_list by both returns intersection
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_by_group_and_ettle_intersection() {
    let mut h = TestHarness::new();
    let g1 = create_group(&mut h, "G1");
    let g2 = create_group(&mut h, "G2");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g1, &e);
    add_group_member(&mut h, &g2, &e);

    let v = ok_value(h.call(
        "group_member_list",
        json!({ "group_id": g1, "ettle_id": e }),
    ));
    let items = v["items"].as_array().unwrap();

    assert_eq!(items.len(), 1, "intersection should return exactly 1 item");
    assert_eq!(items[0]["group_id"].as_str().unwrap(), g1);
    assert_eq!(items[0]["ettle_id"].as_str().unwrap(), e);
}

// ---------------------------------------------------------------------------
// SC-S02b-34 — group_member_list with include_tombstoned includes removed memberships
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_include_tombstoned() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g, &e);
    remove_group_member(&mut h, &g, &e);

    // Default: excluded
    let v_default = ok_value(h.call("group_member_list", json!({ "group_id": g })));
    let default_items = v_default["items"].as_array().unwrap();
    assert!(
        default_items.is_empty(),
        "tombstoned member should be excluded by default"
    );

    // With include_tombstoned: included
    let v_all = ok_value(h.call(
        "group_member_list",
        json!({ "group_id": g, "include_tombstoned": true }),
    ));
    let all_items = v_all["items"].as_array().unwrap();
    assert_eq!(all_items.len(), 1, "tombstoned member should be included");
    assert!(!all_items[0]["tombstoned_at"].is_null());
}

// ---------------------------------------------------------------------------
// SC-S02b-35 — group_member_list with neither filter returns InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_no_filter_returns_invalid_input() {
    let mut h = TestHarness::new();
    let resp = h.call("group_member_list", json!({}));
    assert_error(&resp, "InvalidInput");
}

// ---------------------------------------------------------------------------
// SC-S02b-36 — group_member_list returns empty list when no memberships match
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_empty_when_no_match() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    // No members added

    let v = ok_value(h.call("group_member_list", json!({ "group_id": g })));
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------------
// SC-S02b-37 — group_member_list pagination is complete and non-overlapping
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_pagination_complete_non_overlapping() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e1 = create_ettle(&mut h, "E1");
    let e2 = create_ettle(&mut h, "E2");
    let e3 = create_ettle(&mut h, "E3");
    add_group_member(&mut h, &g, &e1);
    add_group_member(&mut h, &g, &e2);
    add_group_member(&mut h, &g, &e3);

    // Page 1: limit 2
    let r1 = ok_value(h.call("group_member_list", json!({ "group_id": g, "limit": 2 })));
    let page1 = r1["items"].as_array().unwrap();
    assert_eq!(page1.len(), 2);
    let cursor = r1["cursor"].as_str().expect("cursor should be present");

    // Page 2
    let r2 = ok_value(h.call(
        "group_member_list",
        json!({ "group_id": g, "limit": 2, "cursor": cursor }),
    ));
    let page2 = r2["items"].as_array().unwrap();
    assert_eq!(page2.len(), 1);
    assert!(r2["cursor"].is_null() || r2.get("cursor").is_none());

    // No overlap
    let ids1: Vec<&str> = page1
        .iter()
        .map(|i| i["ettle_id"].as_str().unwrap())
        .collect();
    let ids2: Vec<&str> = page2
        .iter()
        .map(|i| i["ettle_id"].as_str().unwrap())
        .collect();
    for id in &ids2 {
        assert!(!ids1.contains(id), "item {} in both pages", id);
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-38 — group_member_list does not use ettlex_apply write path
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_does_not_use_apply_path() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g, &e);

    let sv_before = h.state_version();
    let _ = h.call("group_member_list", json!({ "group_id": g }));
    let sv_after = h.state_version();

    assert_eq!(
        sv_before, sv_after,
        "group_member_list must not increment state_version"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-39 — group_member_list ordering is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_ordering_deterministic() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e1 = create_ettle(&mut h, "E1");
    let e2 = create_ettle(&mut h, "E2");
    add_group_member(&mut h, &g, &e1);
    add_group_member(&mut h, &g, &e2);

    let r1 = ok_value(h.call("group_member_list", json!({ "group_id": g })));
    let r2 = ok_value(h.call("group_member_list", json!({ "group_id": g })));

    assert_eq!(
        r1.to_string(),
        r2.to_string(),
        "group_member_list must be deterministic"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-40 — All five new MCP tools are registered at startup
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_all_five_tools_registered_at_startup() {
    // This is a compile-time / dispatch-table test.
    // If any of these tools is not registered, dispatch returns ToolNotFound.
    let mut h = TestHarness::new();

    // relation_get — registered
    let resp = h.call("relation_get", json!({ "relation_id": "rel:x" }));
    match &resp.result {
        McpResult::Err(e) if e.error_code == "ToolNotFound" => {
            panic!("relation_get not registered in dispatch table")
        }
        _ => {} // NotFound or other error is fine — it means the tool is registered
    }

    // relation_list — registered
    let resp = h.call("relation_list", json!({}));
    match &resp.result {
        McpResult::Err(e) if e.error_code == "ToolNotFound" => {
            panic!("relation_list not registered in dispatch table")
        }
        _ => {}
    }

    // group_get — registered
    let resp = h.call("group_get", json!({ "group_id": "grp:x" }));
    match &resp.result {
        McpResult::Err(e) if e.error_code == "ToolNotFound" => {
            panic!("group_get not registered in dispatch table")
        }
        _ => {}
    }

    // group_list — registered
    let resp = h.call("group_list", json!({}));
    match &resp.result {
        McpResult::Err(e) if e.error_code == "ToolNotFound" => {
            panic!("group_list not registered in dispatch table")
        }
        _ => {}
    }

    // group_member_list — registered
    let resp = h.call("group_member_list", json!({}));
    match &resp.result {
        McpResult::Err(e) if e.error_code == "ToolNotFound" => {
            panic!("group_member_list not registered in dispatch table")
        }
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-41 — group_member_list does not mutate store state
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_group_member_list_does_not_mutate_state() {
    let mut h = TestHarness::new();
    let g = create_group(&mut h, "G");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g, &e);

    let sv = h.state_version();
    let _ = h.call("group_member_list", json!({ "group_id": g }));
    let _ = h.call("group_member_list", json!({ "ettle_id": e }));

    assert_eq!(
        h.state_version(),
        sv,
        "state_version must not change after group_member_list"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-42 — None of the five new tools invoke the write command path
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_no_new_tool_invokes_write_command_path() {
    let mut h = TestHarness::new();
    // Create base data
    let g = create_group(&mut h, "G");
    let e = create_ettle(&mut h, "E");
    add_group_member(&mut h, &g, &e);

    let sv = h.state_version();

    // Call all five read tools
    let _ = h.call("relation_get", json!({ "relation_id": "rel:x" }));
    let _ = h.call("relation_list", json!({ "source_ettle_id": e }));
    let _ = h.call("group_get", json!({ "group_id": g }));
    let _ = h.call("group_list", json!({}));
    let _ = h.call("group_member_list", json!({ "group_id": g }));

    assert_eq!(
        h.state_version(),
        sv,
        "none of the five read tools should mutate state_version"
    );
}
