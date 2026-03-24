//! MCP read tool tests for Slice 02b — relation_get and relation_list.
//!
//! Scenario → test mapping:
//!   SC-S02b-01  test_s02b_relation_get_returns_full_record
//!   SC-S02b-02  test_s02b_relation_get_returns_tombstoned_record
//!   SC-S02b-03  test_s02b_relation_get_not_found
//!   SC-S02b-04  test_s02b_relation_get_does_not_use_apply_path
//!   SC-S02b-05  test_s02b_relation_get_byte_identical_repeated
//!   SC-S02b-06  test_s02b_relation_get_error_logged_with_relation_id
//!   SC-S02b-07  test_s02b_relation_get_does_not_mutate_state
//!   SC-S02b-08  test_s02b_relation_get_fields_match_stored_record
//!   SC-S02b-09  test_s02b_relation_list_by_source_returns_matching
//!   SC-S02b-10  test_s02b_relation_list_by_target_returns_matching
//!   SC-S02b-11  test_s02b_relation_list_by_source_and_target_returns_intersection
//!   SC-S02b-12  test_s02b_relation_list_include_tombstoned
//!   SC-S02b-13  test_s02b_relation_list_no_filter_returns_invalid_input
//!   SC-S02b-14  test_s02b_relation_list_empty_when_no_match
//!   SC-S02b-15  test_s02b_relation_list_pagination_complete_non_overlapping
//!   SC-S02b-16  test_s02b_relation_list_does_not_use_apply_path
//!   SC-S02b-17  test_s02b_relation_list_ordering_deterministic
//!   SC-S02b-18  test_s02b_relation_list_does_not_mutate_state

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

fn create_relation(
    h: &mut TestHarness,
    source_ettle_id: &str,
    target_ettle_id: &str,
    relation_type: &str,
) -> String {
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "RelationCreate",
                "source_ettle_id": source_ettle_id,
                "target_ettle_id": target_ettle_id,
                "relation_type": relation_type
            }
        }),
    );
    match resp.result {
        McpResult::Ok(v) => v["result"]["relation_id"].as_str().unwrap().to_string(),
        McpResult::Err(e) => panic!("create_relation failed: {} — {}", e.error_code, e.message),
    }
}

fn tombstone_relation(h: &mut TestHarness, relation_id: &str) {
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "RelationTombstone", "relation_id": relation_id } }),
    );
    assert!(
        matches!(resp.result, McpResult::Ok(_)),
        "tombstone_relation failed"
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
// SC-S02b-01 — relation_get returns full record for active relation
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_returns_full_record() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");

    let resp = h.call("relation_get", json!({ "relation_id": rid }));
    let v = ok_value(resp);

    assert_eq!(v["relation_id"].as_str().unwrap(), rid);
    assert_eq!(v["relation_type"].as_str().unwrap(), "refinement");
    assert_eq!(v["source_ettle_id"].as_str().unwrap(), a);
    assert_eq!(v["target_ettle_id"].as_str().unwrap(), b);
    assert!(v.get("properties_json").is_some());
    assert!(v.get("created_at").is_some());
    assert!(v["tombstoned_at"].is_null());
}

// ---------------------------------------------------------------------------
// SC-S02b-02 — relation_get returns tombstoned record
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_returns_tombstoned_record() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");
    tombstone_relation(&mut h, &rid);

    let resp = h.call("relation_get", json!({ "relation_id": rid }));
    let v = ok_value(resp);

    assert_eq!(v["relation_id"].as_str().unwrap(), rid);
    assert!(
        !v["tombstoned_at"].is_null(),
        "tombstoned_at should be non-null"
    );
    // tombstoned_at is a non-empty ISO-8601 string
    let ts = v["tombstoned_at"].as_str().unwrap();
    assert!(!ts.is_empty());
}

// ---------------------------------------------------------------------------
// SC-S02b-03 — relation_get returns NotFound for unknown relation_id
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_not_found() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "relation_get",
        json!({ "relation_id": "rel:does-not-exist" }),
    );
    assert_error(&resp, "NotFound");
}

// ---------------------------------------------------------------------------
// SC-S02b-04 — relation_get does not use the ettlex_apply write path
//              (state_version must not change)
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_does_not_use_apply_path() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");

    let sv_before = h.state_version();
    let _resp = h.call("relation_get", json!({ "relation_id": rid }));
    let sv_after = h.state_version();

    assert_eq!(
        sv_before, sv_after,
        "relation_get must not increment state_version"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-05 — relation_get is byte-identical across repeated calls
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_byte_identical_repeated() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");

    let r1 = ok_value(h.call("relation_get", json!({ "relation_id": rid })));
    let r2 = ok_value(h.call("relation_get", json!({ "relation_id": rid })));

    assert_eq!(
        r1.to_string(),
        r2.to_string(),
        "repeated relation_get calls must produce byte-identical responses"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-06 — relation_get errors include the relation_id
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_error_logged_with_relation_id() {
    let mut h = TestHarness::new();
    let unknown_id = "rel:unknown-for-logging-test";
    let resp = h.call("relation_get", json!({ "relation_id": unknown_id }));
    match &resp.result {
        McpResult::Err(e) => {
            assert!(
                e.message.contains(unknown_id),
                "error message should contain the relation_id '{}', got: {}",
                unknown_id,
                e.message
            );
        }
        McpResult::Ok(v) => panic!("expected error but got Ok: {}", v),
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-07 — relation_get does not mutate store state
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_does_not_mutate_state() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");

    let sv = h.state_version();
    // Call for an existing relation
    let _ = h.call("relation_get", json!({ "relation_id": rid }));
    // Call for a nonexistent relation (error path)
    let _ = h.call("relation_get", json!({ "relation_id": "rel:nope" }));

    assert_eq!(
        h.state_version(),
        sv,
        "state_version must not change after relation_get calls"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-08 — relation_get response fields match stored record byte-for-byte
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_get_fields_match_stored_record() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "Source Ettle");
    let b = create_ettle(&mut h, "Target Ettle");
    let rid = create_relation(&mut h, &a, &b, "refinement");

    let v = ok_value(h.call("relation_get", json!({ "relation_id": rid })));

    // All fields must exactly match what was stored
    assert_eq!(v["relation_id"].as_str().unwrap(), rid);
    assert_eq!(v["source_ettle_id"].as_str().unwrap(), a);
    assert_eq!(v["target_ettle_id"].as_str().unwrap(), b);
    assert_eq!(v["relation_type"].as_str().unwrap(), "refinement");
    // properties_json defaults to "{}" when not supplied
    let pj = v["properties_json"].as_str().unwrap();
    assert!(!pj.is_empty());
    // created_at is a non-empty ISO-8601 string
    let ca = v["created_at"].as_str().unwrap();
    assert!(!ca.is_empty());
    // tombstoned_at is null for an active relation
    assert!(v["tombstoned_at"].is_null());
}

// ---------------------------------------------------------------------------
// SC-S02b-09 — relation_list by source_ettle_id returns matching, excludes tombstoned
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_by_source_returns_matching() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let c = create_ettle(&mut h, "C");
    let r1 = create_relation(&mut h, &a, &b, "refinement");
    let r2 = create_relation(&mut h, &a, &c, "refinement");
    let r3 = create_relation(&mut h, &b, &c, "refinement"); // different source

    // Tombstone r2 to verify it's excluded by default
    tombstone_relation(&mut h, &r2);

    let resp = h.call("relation_list", json!({ "source_ettle_id": a }));
    let v = ok_value(resp);
    let items = v["items"].as_array().unwrap();

    // Only r1 is active with source A
    let ids: Vec<&str> = items
        .iter()
        .map(|i| i["relation_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&r1.as_str()), "r1 should be present");
    assert!(
        !ids.contains(&r2.as_str()),
        "r2 (tombstoned) should be excluded"
    );
    assert!(
        !ids.contains(&r3.as_str()),
        "r3 (different source) should be excluded"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-10 — relation_list by target_ettle_id returns matching
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_by_target_returns_matching() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let c = create_ettle(&mut h, "C");
    let r1 = create_relation(&mut h, &a, &b, "refinement");
    let r2 = create_relation(&mut h, &c, &b, "refinement");
    let _r3 = create_relation(&mut h, &a, &c, "refinement"); // different target

    let resp = h.call("relation_list", json!({ "target_ettle_id": b }));
    let v = ok_value(resp);
    let items = v["items"].as_array().unwrap();

    let ids: Vec<&str> = items
        .iter()
        .map(|i| i["relation_id"].as_str().unwrap())
        .collect();
    assert!(ids.contains(&r1.as_str()));
    assert!(ids.contains(&r2.as_str()));
    assert_eq!(ids.len(), 2);
}

// ---------------------------------------------------------------------------
// SC-S02b-11 — relation_list by both source and target returns intersection
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_by_source_and_target_returns_intersection() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let c = create_ettle(&mut h, "C");
    let r1 = create_relation(&mut h, &a, &b, "refinement");
    let _r2 = create_relation(&mut h, &a, &c, "refinement"); // same source, different target
    let _r3 = create_relation(&mut h, &c, &b, "refinement"); // same target, different source

    let resp = h.call(
        "relation_list",
        json!({ "source_ettle_id": a, "target_ettle_id": b }),
    );
    let v = ok_value(resp);
    let items = v["items"].as_array().unwrap();

    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["relation_id"].as_str().unwrap(), r1);
}

// ---------------------------------------------------------------------------
// SC-S02b-12 — relation_list with include_tombstoned includes tombstoned
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_include_tombstoned() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let rid = create_relation(&mut h, &a, &b, "refinement");
    tombstone_relation(&mut h, &rid);

    let resp = h.call(
        "relation_list",
        json!({ "source_ettle_id": a, "include_tombstoned": true }),
    );
    let v = ok_value(resp);
    let items = v["items"].as_array().unwrap();

    let ids: Vec<&str> = items
        .iter()
        .map(|i| i["relation_id"].as_str().unwrap())
        .collect();
    assert!(
        ids.contains(&rid.as_str()),
        "tombstoned relation should be included"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-13 — relation_list with neither filter returns InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_no_filter_returns_invalid_input() {
    let mut h = TestHarness::new();
    let resp = h.call("relation_list", json!({}));
    assert_error(&resp, "InvalidInput");
}

// ---------------------------------------------------------------------------
// SC-S02b-14 — relation_list returns empty list when no relations match
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_empty_when_no_match() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    // No relations created for A

    let resp = h.call("relation_list", json!({ "source_ettle_id": a }));
    let v = ok_value(resp);
    let items = v["items"].as_array().unwrap();

    assert!(
        items.is_empty(),
        "expected empty list, got {} items",
        items.len()
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-15 — relation_list pagination is complete and non-overlapping
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_pagination_complete_non_overlapping() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let c = create_ettle(&mut h, "C");
    let d = create_ettle(&mut h, "D");
    // Create 3 relations from A
    create_relation(&mut h, &a, &b, "refinement");
    create_relation(&mut h, &a, &c, "refinement");
    create_relation(&mut h, &a, &d, "refinement");

    // Page 1: limit 2
    let r1 = ok_value(h.call("relation_list", json!({ "source_ettle_id": a, "limit": 2 })));
    let page1_items = r1["items"].as_array().unwrap();
    assert_eq!(page1_items.len(), 2, "page 1 should have 2 items");
    let cursor = r1["cursor"]
        .as_str()
        .expect("cursor should be present after page 1");

    // Page 2: use cursor
    let r2 = ok_value(h.call(
        "relation_list",
        json!({ "source_ettle_id": a, "limit": 2, "cursor": cursor }),
    ));
    let page2_items = r2["items"].as_array().unwrap();
    assert_eq!(page2_items.len(), 1, "page 2 should have 1 item");
    assert!(
        r2["cursor"].is_null() || r2.get("cursor").is_none(),
        "no cursor after last page"
    );

    // No overlap
    let ids1: Vec<&str> = page1_items
        .iter()
        .map(|i| i["relation_id"].as_str().unwrap())
        .collect();
    let ids2: Vec<&str> = page2_items
        .iter()
        .map(|i| i["relation_id"].as_str().unwrap())
        .collect();
    for id in &ids2 {
        assert!(!ids1.contains(id), "item {} appeared in both pages", id);
    }
}

// ---------------------------------------------------------------------------
// SC-S02b-16 — relation_list does not use ettlex_apply write path
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_does_not_use_apply_path() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    create_relation(&mut h, &a, &b, "refinement");

    let sv_before = h.state_version();
    let _ = h.call("relation_list", json!({ "source_ettle_id": a }));
    let sv_after = h.state_version();

    assert_eq!(
        sv_before, sv_after,
        "relation_list must not increment state_version"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-17 — relation_list ordering is deterministic (created_at ASC, relation_id ASC)
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_ordering_deterministic() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    let c = create_ettle(&mut h, "C");
    create_relation(&mut h, &a, &b, "refinement");
    create_relation(&mut h, &a, &c, "refinement");

    let r1 = ok_value(h.call("relation_list", json!({ "source_ettle_id": a })));
    let r2 = ok_value(h.call("relation_list", json!({ "source_ettle_id": a })));

    assert_eq!(
        r1.to_string(),
        r2.to_string(),
        "relation_list must produce identical ordering on repeated calls"
    );
}

// ---------------------------------------------------------------------------
// SC-S02b-18 — relation_list does not mutate store state
// ---------------------------------------------------------------------------

#[test]
fn test_s02b_relation_list_does_not_mutate_state() {
    let mut h = TestHarness::new();
    let a = create_ettle(&mut h, "A");
    let b = create_ettle(&mut h, "B");
    create_relation(&mut h, &a, &b, "refinement");

    let sv = h.state_version();
    let _ = h.call("relation_list", json!({ "source_ettle_id": a }));
    let _ = h.call("relation_list", json!({ "source_ettle_id": a }));

    assert_eq!(
        h.state_version(),
        sv,
        "state_version must not change after relation_list"
    );
}
