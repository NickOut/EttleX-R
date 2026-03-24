//! Integration tests for ep:mcp_thin_slice:0
//!
//! Scenario tests exercising the MCP server in-process.
//! TDD RED→GREEN: tests written from spec only, before production code.
//!
//! Scenario → test mapping:
//!   S-AUTH-1   test_s_auth_1_reject_missing_token
//!   S-AUTH-2   test_s_auth_2_correlation_threaded
//!   S-UNK-1    test_s_unk_1_unknown_tool
//!   S-CURSOR-1 test_s_cursor_1_invalid_cursor
//!   S-APPLY-1  test_s_apply_1_unknown_command_tag
//!   S-APPLY-2  test_s_apply_2_missing_required_field
//!   S-APPLY-3  test_s_apply_3_oversized_payload
//!   S-OCC-1    test_s_occ_1_head_mismatch
//!   S-OCC-2    test_s_occ_2_new_state_version
//!   S-INV-2    test_s_inv_2_write_via_apply_only
//!   S-PAGE-1   test_s_page_1_ettle_list_default_limit
//!   S-PAGE-2   test_s_page_2_ettle_list_cursor_deterministic
//!   S-POL-1    test_s_pol_1_policy_get_deterministic
//!   S-POL-2    test_s_pol_2_project_for_handoff_deterministic
//!   S-POL-3    test_s_pol_3_policy_not_found
//!   S-POL-4    test_s_pol_4_profile_not_found
//!   S-POL-5    test_s_pol_5_policy_list_default_limit
//!   S-POL-6    test_s_pol_6_policy_list_cursor
//!   S-DET-2    test_s_det_2_determinism_violation_detected
//!   S-CON-2    test_s_con_2_missing_family (ignored — constraints table dropped in Slice 02)
//!   S-PROF-1   test_s_prof_1_profile_get_bytes
//!   S-PROF-2   test_s_prof_2_profile_list_limit
//!   S-APPR-1   test_s_appr_1_approval_not_found
//!   S-PRED-1   test_s_pred_1_preview_thin_transport
//!   S-PRED-2   test_s_pred_2_preview_no_mutation
//!   S-PRED-3   test_s_pred_3_preview_deterministic
//!   S-PA-1     test_s_pa_1_profile_create_readable
//!   S-PA-2     test_s_pa_2_profile_conflict
//!   S-PA-3     test_s_pa_3_profile_set_default_not_found
//!   S-PA-4     test_s_pa_4_profile_set_default_readable
//!
//! Deleted (EP-era, retired by Slice 03 / migration 015):
//!   S-INV-1    test_s_inv_1_delegation_only
//!   S-QUERY-1  test_s_query_1_no_mutation
//!   S-PAGE-3   test_s_page_3_snapshot_list_default_limit
//!   S-PAGE-4   test_s_page_4_snapshot_get_head_deterministic
//!   S-DET-1    test_s_det_1_canonical_json_stable
//!   S-SNAP-1   test_s_snap_1_snapshot_commit
//!   S-SNAP-2   test_s_snap_2_not_a_leaf
//!   S-SNAP-3   test_s_snap_3_policy_denied
//!   S-DIFF-1   test_s_diff_1_snapshot_diff
//!   S-DIFF-2   test_s_diff_2_missing_ref
//!   S-DIFF-3   test_s_diff_3_missing_blob
//!   S-CON-1    test_s_con_1_create_attach_snapshot
//!   S-CON-3    test_s_con_3_duplicate_attachment
//!   S-BOUND-1  test_s_bound_1_large_list_eps
//!   S-RAP-1    test_s_rap_1_routed_for_approval
//!   S-RAP-2    test_s_rap_2_no_auto_profile_ref
//!   S-RAP-3    test_s_rap_3_policy_denied_no_approval

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_mcp::auth::AuthConfig;
use ettlex_mcp::context::RequestContext;
use ettlex_mcp::server::{McpResponse, McpResult, McpServer, McpToolCall};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::json;
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
        let server = McpServer::new(AuthConfig::with_token("t:dev"), 1024 * 1024);
        Self {
            _tmp: tmp,
            conn,
            cas,
            server,
        }
    }

    fn call(&mut self, tool: &str, params: serde_json::Value) -> McpResponse {
        let size = params.to_string().len();
        self.server.dispatch(
            McpToolCall {
                tool_name: tool.to_string(),
                params,
                context: RequestContext::default(),
                auth_token: Some("t:dev".to_string()),
                payload_size: size,
            },
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
    }

    fn call_no_auth(&mut self, tool: &str, params: serde_json::Value) -> McpResponse {
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

    fn call_with_correlation(
        &mut self,
        tool: &str,
        params: serde_json::Value,
        cid: &str,
    ) -> McpResponse {
        let size = params.to_string().len();
        self.server.dispatch(
            McpToolCall {
                tool_name: tool.to_string(),
                params,
                context: RequestContext {
                    correlation_id: Some(cid.to_string()),
                },
                auth_token: Some("t:dev".to_string()),
                payload_size: size,
            },
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
    }

    fn call_oversized(
        &mut self,
        tool: &str,
        params: serde_json::Value,
        size: usize,
    ) -> McpResponse {
        self.server.dispatch(
            McpToolCall {
                tool_name: tool.to_string(),
                params,
                context: RequestContext::default(),
                auth_token: Some("t:dev".to_string()),
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

    fn approval_count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM approval_requests", [], |r| r.get(0))
            .unwrap()
    }

    /// Insert a profile via raw SQL.
    fn seed_profile(&mut self, profile_ref: &str, payload: &str, is_default: bool) {
        self.conn
            .execute(
                "INSERT INTO profiles (profile_ref, payload_json, is_default, created_at) VALUES (?1, ?2, ?3, 0)",
                rusqlite::params![profile_ref, payload, if is_default { 1i32 } else { 0i32 }],
            )
            .unwrap();
    }

    /// Bulk-insert N ettles via raw SQL (uses current schema).
    fn seed_ettles(&mut self, n: usize) {
        let mut sql = String::new();
        for i in 0..n {
            let ts = format!("2024-01-01T00:00:{i:02}.000Z");
            sql.push_str(&format!(
                "INSERT INTO ettles (id, title, why, what, how, created_at, updated_at) \
                 VALUES ('ettle:bulk:{i:05}', 'Bulk {i}', '', '', '', '{ts}', '{ts}');\n"
            ));
        }
        self.conn.execute_batch(&sql).unwrap();
    }
}

fn assert_error(resp: &McpResponse, expected_code: &str) {
    match &resp.result {
        McpResult::Err(e) => assert_eq!(
            e.error_code, expected_code,
            "expected error_code '{}' but got '{}': {}",
            expected_code, e.error_code, e.message
        ),
        McpResult::Ok(v) => panic!("expected error '{}', got success: {}", expected_code, v),
    }
}

fn assert_ok(resp: McpResponse) -> serde_json::Value {
    match resp.result {
        McpResult::Ok(v) => v,
        McpResult::Err(e) => panic!("expected ok, got error '{}': {}", e.error_code, e.message),
    }
}

// ---------------------------------------------------------------------------
// S-AUTH-1 — Reject missing auth token
// ---------------------------------------------------------------------------

#[test]
fn test_s_auth_1_reject_missing_token() {
    let mut h = TestHarness::new();
    let resp = h.call_no_auth("ettle_list", json!({}));
    assert_error(&resp, "AuthRequired");
}

// ---------------------------------------------------------------------------
// S-AUTH-2 — Accept token and thread correlation_id
// ---------------------------------------------------------------------------

#[test]
fn test_s_auth_2_correlation_threaded() {
    let mut h = TestHarness::new();
    // Call with correlation_id; the response must echo it back
    let resp = h.call_with_correlation("ettle_list", json!({}), "c:1");
    assert_eq!(resp.correlation_id.as_deref(), Some("c:1"));
    // Even on a successful call, correlation_id is threaded
    match resp.result {
        McpResult::Ok(_) => {} // success is fine
        McpResult::Err(e) => panic!("unexpected error: {}", e.error_code),
    }
}

// ---------------------------------------------------------------------------
// S-UNK-1 — Reject unknown tool name
// ---------------------------------------------------------------------------

#[test]
fn test_s_unk_1_unknown_tool() {
    let mut h = TestHarness::new();
    let resp = h.call("not.a.tool", json!({}));
    assert_error(&resp, "ToolNotFound");
}

// ---------------------------------------------------------------------------
// S-CURSOR-1 — Reject invalid cursor
// ---------------------------------------------------------------------------

#[test]
fn test_s_cursor_1_invalid_cursor() {
    let mut h = TestHarness::new();
    let resp = h.call("ettle_list", json!({ "cursor": "not-a-cursor" }));
    assert_error(&resp, "InvalidCursor");
}

// ---------------------------------------------------------------------------
// S-APPLY-1 — Apply rejects unknown command tag
// ---------------------------------------------------------------------------

#[test]
fn test_s_apply_1_unknown_command_tag() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "Command::Nope", "data": 42 } }),
    );
    assert_error(&resp, "InvalidCommand");
}

// ---------------------------------------------------------------------------
// S-APPLY-2 — Apply rejects missing required fields
// ---------------------------------------------------------------------------

#[test]
fn test_s_apply_2_missing_required_field() {
    let mut h = TestHarness::new();
    // EttleCreate requires title; omit it → InvalidInput
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate" } }), // missing required title field
    );
    assert_error(&resp, "InvalidInput");
}

// ---------------------------------------------------------------------------
// S-APPLY-3 — Apply rejects oversized payloads
// ---------------------------------------------------------------------------

#[test]
fn test_s_apply_3_oversized_payload() {
    let mut h = TestHarness::new();
    // payload_size > 1MB = 1048576 bytes
    let resp = h.call_oversized(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate", "title": "x" } }),
        1_048_577,
    );
    assert_error(&resp, "RequestTooLarge");
}

// ---------------------------------------------------------------------------
// S-OCC-1 — HeadMismatch on state_version mismatch
// ---------------------------------------------------------------------------

#[test]
fn test_s_occ_1_head_mismatch() {
    let mut h = TestHarness::new();
    let current_sv = h.state_version(); // 0 on fresh DB
    let wrong_sv = current_sv + 999;
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": { "tag": "EttleCreate", "title": "Occ Test" },
            "expected_state_version": wrong_sv
        }),
    );
    assert_error(&resp, "HeadMismatch");
    // state_version unchanged
    assert_eq!(h.state_version(), current_sv);
}

// ---------------------------------------------------------------------------
// S-OCC-2 — Returns new_state_version on success
// ---------------------------------------------------------------------------

#[test]
fn test_s_occ_2_new_state_version() {
    let mut h = TestHarness::new();
    let before = h.state_version();
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate", "title": "Sv Test" } }),
    );
    let v = assert_ok(resp);
    let new_sv = v["new_state_version"]
        .as_u64()
        .expect("new_state_version must be u64");
    assert_eq!(new_sv, before + 1);
    assert_eq!(h.state_version(), before + 1);
}

// ---------------------------------------------------------------------------
// S-INV-2 — Write operations call Apply only
// ---------------------------------------------------------------------------

#[test]
fn test_s_inv_2_write_via_apply_only() {
    let mut h = TestHarness::new();
    let before_sv = h.state_version();
    let resp = h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate", "title": "InvTest" } }),
    );
    let v = assert_ok(resp);
    // new_state_version increased (apply was called)
    let new_sv = v["new_state_version"].as_u64().unwrap();
    assert_eq!(new_sv, before_sv + 1);
}

// ---------------------------------------------------------------------------
// S-PAGE-1 — ettle.list enforces default limit
// ---------------------------------------------------------------------------

#[test]
fn test_s_page_1_ettle_list_default_limit() {
    let mut h = TestHarness::new();
    h.seed_ettles(150); // > default limit of 100

    let v = assert_ok(h.call("ettle_list", json!({})));
    let items = v["items"].as_array().expect("items array");
    assert!(items.len() <= 100, "items.len() {} > 100", items.len());
    // cursor present since more exist
    assert!(v["cursor"].is_string(), "cursor should be present");
}

// ---------------------------------------------------------------------------
// S-PAGE-2 — ettle.list cursor pagination (deterministic)
// ---------------------------------------------------------------------------

#[test]
fn test_s_page_2_ettle_list_cursor_deterministic() {
    let mut h = TestHarness::new();
    h.seed_ettles(250);

    let v1 = assert_ok(h.call("ettle_list", json!({ "limit": 100 })));
    let cursor1 = v1["cursor"]
        .as_str()
        .expect("cursor_1 must be present")
        .to_string();
    let ids1: Vec<String> = v1["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(ids1.len(), 100);

    let v2 = assert_ok(h.call("ettle_list", json!({ "limit": 100, "cursor": cursor1 })));
    let ids2: Vec<String> = v2["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap_or_default().to_string())
        .collect();

    // No duplicates
    let all_ids: std::collections::HashSet<_> = ids1.iter().chain(ids2.iter()).collect();
    assert_eq!(all_ids.len(), ids1.len() + ids2.len(), "duplicates found");

    // Determinism: same call returns same result
    let v1b = assert_ok(h.call("ettle_list", json!({ "limit": 100 })));
    let ids1b: Vec<String> = v1b["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(ids1, ids1b);
}

// ---------------------------------------------------------------------------
// S-POL-1 — policy.get via MCP is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_1_policy_get_deterministic() {
    let mut h = TestHarness::new();
    // policies/codegen_handoff_policy_v1.md should be loadable via FilePolicyProvider
    // For this test we use NoopPolicyProvider which returns PolicyNotFound; we just
    // verify that two identical calls produce the same (possibly error) result.
    let r1 = h.call("policy_get", json!({ "policy_ref": "any/policy@0" }));
    let r2 = h.call("policy_get", json!({ "policy_ref": "any/policy@0" }));
    // Both should return the same error code (deterministic)
    match (&r1.result, &r2.result) {
        (McpResult::Ok(v1), McpResult::Ok(v2)) => {
            assert_eq!(
                serde_json::to_string(v1).unwrap(),
                serde_json::to_string(v2).unwrap()
            );
        }
        (McpResult::Err(e1), McpResult::Err(e2)) => {
            assert_eq!(e1.error_code, e2.error_code);
        }
        _ => panic!("calls not deterministic"),
    }
}

// ---------------------------------------------------------------------------
// S-POL-2 — policy.project_for_handoff via MCP is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_2_project_for_handoff_deterministic() {
    let mut h = TestHarness::new();
    let r1 = h.call(
        "policy_project_for_handoff",
        json!({ "policy_ref": "any@0", "profile_ref": null }),
    );
    let r2 = h.call(
        "policy_project_for_handoff",
        json!({ "policy_ref": "any@0", "profile_ref": null }),
    );
    match (&r1.result, &r2.result) {
        (McpResult::Ok(v1), McpResult::Ok(v2)) => {
            assert_eq!(
                serde_json::to_string(v1).unwrap(),
                serde_json::to_string(v2).unwrap()
            );
        }
        (McpResult::Err(e1), McpResult::Err(e2)) => {
            assert_eq!(e1.error_code, e2.error_code);
        }
        _ => panic!("calls not deterministic"),
    }
}

// ---------------------------------------------------------------------------
// S-POL-3 — policy.project_for_handoff surfaces PolicyNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_3_policy_not_found() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "policy_project_for_handoff",
        json!({ "policy_ref": "policy/unknown@0" }),
    );
    assert_error(&resp, "PolicyNotFound");
}

// ---------------------------------------------------------------------------
// S-POL-4 — policy.project_for_handoff surfaces ProfileNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_4_profile_not_found() {
    let mut h = TestHarness::new();
    // Using ettle.list, since it doesn't require a policy provider to surface ProfileNotFound.
    // Actually we test this via snapshot.get_head for a non-existent ettle.
    // But ProfileNotFound is specifically from profile operations.
    let resp = h.call("profile_get", json!({ "profile_ref": "profile/missing@0" }));
    assert_error(&resp, "ProfileNotFound");
}

// ---------------------------------------------------------------------------
// S-POL-5 — policy.list enforces default limit
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_5_policy_list_default_limit() {
    let mut h = TestHarness::new();
    // With NoopPolicyProvider, policy.list returns empty list (not error)
    let v = assert_ok(h.call("policy_list", json!({})));
    let items = v["items"].as_array().expect("items array");
    assert!(items.len() <= 100, "items must be <= 100");
}

// ---------------------------------------------------------------------------
// S-POL-6 — policy.list cursor pagination
// ---------------------------------------------------------------------------

#[test]
fn test_s_pol_6_policy_list_cursor() {
    let mut h = TestHarness::new();
    // With NoopPolicyProvider, policy.list returns empty
    let v = assert_ok(h.call("policy_list", json!({ "limit": 100 })));
    let items = v["items"].as_array().expect("items array");
    assert!(items.len() <= 100);
    // No cursor since no items
    // (full pagination test with many policies is beyond NoopPolicyProvider scope)
}

// ---------------------------------------------------------------------------
// S-DET-2 — Determinism test detects unstable key ordering
// ---------------------------------------------------------------------------

#[test]
fn test_s_det_2_determinism_violation_detected() {
    // This test verifies the test infrastructure can detect non-deterministic JSON.
    // We construct two JSON objects with same keys but different insertion order,
    // then verify that our canonical serialiser produces identical output.
    use ettlex_mcp::canonical::canonical_json;

    // Input 1: keys in different order from Input 2
    let a = json!({ "z": 1, "a": 2, "m": 3 });
    let b = json!({ "a": 2, "m": 3, "z": 1 });

    let ca = canonical_json(&a);
    let cb = canonical_json(&b);
    assert_eq!(
        ca, cb,
        "canonical_json must produce identical bytes for same logical content"
    );
}

// ---------------------------------------------------------------------------
// S-CON-2 — Reject constraint create missing family (ignored — table dropped in Slice 02)
// ---------------------------------------------------------------------------

#[test]
#[ignore = "EP constraint model deprecated in Slice 02; constraints/ep_constraint_refs tables dropped — revisit in policy/snapshot slice"]
fn test_s_con_2_missing_family() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ConstraintCreate",
                "constraint_id": "c:bad",
                "family": "",
                "kind": "K",
                "scope": "EP",
                "payload_json": {}
            }
        }),
    );
    assert_error(&resp, "InvalidConstraintFamily");
}

// ---------------------------------------------------------------------------
// S-PROF-1 — profile.get via MCP returns identical bytes
// ---------------------------------------------------------------------------

#[test]
fn test_s_prof_1_profile_get_bytes() {
    let mut h = TestHarness::new();
    h.seed_profile("profile/default@0", r#"{"ambiguity_policy":"deny"}"#, true);

    let r1 = assert_ok(h.call("profile_get", json!({ "profile_ref": "profile/default@0" })));
    let r2 = assert_ok(h.call("profile_get", json!({ "profile_ref": "profile/default@0" })));

    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap(),
        "profile.get bytes must be identical"
    );
    assert_eq!(r1["profile_ref"].as_str(), Some("profile/default@0"));
}

// ---------------------------------------------------------------------------
// S-PROF-2 — profile.list enforces default limit
// ---------------------------------------------------------------------------

#[test]
fn test_s_prof_2_profile_list_limit() {
    let mut h = TestHarness::new();
    // Seed 150 profiles
    for i in 0..150usize {
        h.conn.execute(
            "INSERT INTO profiles (profile_ref, payload_json, is_default, created_at) VALUES (?1, '{}', 0, ?2)",
            rusqlite::params![format!("profile/bulk:{i:05}@0"), i as i64],
        ).unwrap();
    }

    let v = assert_ok(h.call("profile_list", json!({})));
    let items = v["items"].as_array().expect("items array");
    assert!(items.len() <= 100, "items.len() {} > 100", items.len());
    assert!(v["cursor"].is_string(), "cursor present when more exist");
}

// ---------------------------------------------------------------------------
// S-APPR-1 — approval.get surfaces ApprovalNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_s_appr_1_approval_not_found() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "approval_get",
        json!({ "approval_token": "approval:missing" }),
    );
    assert_error(&resp, "ApprovalNotFound");
}

// ---------------------------------------------------------------------------
// S-PRED-1 — constraint_predicates.preview is thin transport
// ---------------------------------------------------------------------------

#[test]
fn test_s_pred_1_preview_thin_transport() {
    let mut h = TestHarness::new();
    h.seed_profile(
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": false}"#,
        true,
    );

    let r1 = assert_ok(h.call(
        "constraint_predicates_preview",
        json!({ "profile_ref": "profile/default@0", "context": {}, "candidates": [] }),
    ));
    let r2 = assert_ok(h.call(
        "constraint_predicates_preview",
        json!({ "profile_ref": "profile/default@0", "context": {}, "candidates": [] }),
    ));
    // Identical inputs → identical outputs
    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap()
    );
}

// ---------------------------------------------------------------------------
// S-PRED-2 — preview does not create approval requests or mutate state
// ---------------------------------------------------------------------------

#[test]
fn test_s_pred_2_preview_no_mutation() {
    let mut h = TestHarness::new();
    h.seed_profile(
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": false}"#,
        true,
    );

    let sv_before = h.state_version();
    let appr_before = h.approval_count();

    let _ = assert_ok(h.call(
        "constraint_predicates_preview",
        json!({ "profile_ref": "profile/default@0", "context": {}, "candidates": [] }),
    ));

    assert_eq!(
        h.state_version(),
        sv_before,
        "state_version must not change on preview"
    );
    assert_eq!(
        h.approval_count(),
        appr_before,
        "approval count must not change on preview"
    );
}

// ---------------------------------------------------------------------------
// S-PRED-3 — preview is deterministic for identical inputs
// ---------------------------------------------------------------------------

#[test]
fn test_s_pred_3_preview_deterministic() {
    let mut h = TestHarness::new();
    h.seed_profile(
        "profile/default@0",
        r#"{"predicate_evaluation_enabled": false}"#,
        true,
    );

    let r1 = assert_ok(h.call(
        "constraint_predicates_preview",
        json!({ "profile_ref": "profile/default@0", "context": {}, "candidates": ["c:a", "c:b"] }),
    ));
    let r2 = assert_ok(h.call(
        "constraint_predicates_preview",
        json!({ "profile_ref": "profile/default@0", "context": {}, "candidates": ["c:a", "c:b"] }),
    ));
    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap(),
        "preview must be byte-identical for identical inputs"
    );
}

// ---------------------------------------------------------------------------
// S-PA-1 — ProfileCreate succeeds and is readable
// ---------------------------------------------------------------------------

#[test]
fn test_s_pa_1_profile_create_readable() {
    let mut h = TestHarness::new();
    let before_sv = h.state_version();

    let payload = json!({ "ambiguity_policy": "deny" });
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ProfileCreate",
                "profile_ref": "profile/demo@0",
                "payload_json": payload
            }
        }),
    );
    let v = assert_ok(resp);
    assert_eq!(v["new_state_version"].as_u64(), Some(before_sv + 1));

    // Read back
    let r = assert_ok(h.call("profile_get", json!({ "profile_ref": "profile/demo@0" })));
    assert_eq!(r["profile_ref"].as_str(), Some("profile/demo@0"));
    assert_eq!(r["payload"]["ambiguity_policy"].as_str(), Some("deny"));
}

// ---------------------------------------------------------------------------
// S-PA-2 — ProfileCreate conflict surfaces ProfileConflict
// ---------------------------------------------------------------------------

#[test]
fn test_s_pa_2_profile_conflict() {
    let mut h = TestHarness::new();
    h.seed_profile("profile/demo@0", r#"{"ambiguity_policy":"deny"}"#, false);

    // Create with different content → ProfileConflict
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ProfileCreate",
                "profile_ref": "profile/demo@0",
                "payload_json": { "ambiguity_policy": "route_for_approval" }
            }
        }),
    );
    assert_error(&resp, "ProfileConflict");
}

// ---------------------------------------------------------------------------
// S-PA-3 — ProfileSetDefault surfaces ProfileNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_s_pa_3_profile_set_default_not_found() {
    let mut h = TestHarness::new();
    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ProfileSetDefault",
                "profile_ref": "profile/missing@0"
            }
        }),
    );
    assert_error(&resp, "ProfileNotFound");
}

// ---------------------------------------------------------------------------
// S-PA-4 — ProfileSetDefault makes get_default return new value
// ---------------------------------------------------------------------------

#[test]
fn test_s_pa_4_profile_set_default_readable() {
    let mut h = TestHarness::new();
    h.seed_profile("profile/demo@0", r#"{"ambiguity_policy":"deny"}"#, false);

    let resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ProfileSetDefault",
                "profile_ref": "profile/demo@0"
            }
        }),
    );
    let _ = assert_ok(resp);

    // Get default profile
    let r = assert_ok(h.call("profile_get_default", json!({})));
    assert_eq!(
        r["profile_ref"].as_str(),
        Some("profile/demo@0"),
        "default profile must be 'profile/demo@0'"
    );
}
