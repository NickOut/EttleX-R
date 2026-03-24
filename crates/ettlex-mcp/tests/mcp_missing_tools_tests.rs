//! Tests for ep:mcp_thin_slice ordinal 2 — wiring 17 missing MCP query tools.
//!
//! Each test verifies transport routing: parse params → EngineQuery → serialise result.
//! The test does NOT exhaustively exercise underlying store logic (that is covered by
//! unit tests in ettlex-engine and ettlex-store).
//!
//! Scenario → test mapping:
//!   S-MT-HP-1   test_state_get_version_returns_version
//!   S-MT-HP-7   test_decision_get_happy_path
//!   S-MT-HP-8   test_decision_list_returns_empty_not_error
//!   S-MT-HP-15  test_profile_resolve_happy_path
//!   S-MT-HP-16  test_approval_list_returns_empty_not_error
//!   S-MT-HP-17  test_policy_export_happy_path
//!   S-MT-ERR-3  test_decision_get_missing_returns_not_found
//!   S-MT-ERR-4  test_manifest_get_by_digest_bad_digest_returns_missing_blob
//!   S-MT-ERR-6  test_profile_resolve_missing_ref_returns_not_found
//!   S-MT-ERR-7  test_policy_export_nonexistent_returns_policy_not_found
//!   S-MT-ERR-8  test_policy_export_unknown_kind_returns_policy_export_failed
//!   S-MT-INV-2  test_state_version_increments_after_apply
//!
//! Deleted (EP-era, retired by Slice 03 / migration 015):
//!   S-MT-HP-2   test_ep_list_children_happy_path
//!   S-MT-HP-3   test_ep_list_parents_happy_path
//!   S-MT-HP-4   test_ep_list_constraints_happy_path
//!   S-MT-HP-5   test_constraint_get_happy_path
//!   S-MT-HP-6   test_constraint_list_by_family_happy_path
//!   S-MT-HP-9   test_decision_list_by_target_happy_path
//!   S-MT-HP-10  test_ep_list_decisions_happy_path
//!   S-MT-HP-11  test_ettle_list_decisions_happy_path
//!   S-MT-HP-12  test_ept_compute_decision_context_happy_path
//!   S-MT-HP-13  test_manifest_get_by_digest_happy_path
//!   S-MT-HP-14  test_ept_compute_happy_path
//!   S-MT-ERR-1  test_ep_list_children_missing_param_returns_invalid_input
//!   S-MT-ERR-2  test_constraint_get_missing_returns_not_found
//!   S-MT-ERR-5  test_ept_compute_missing_ep_returns_not_found
//!   S-MT-INV-1  test_query_tools_do_not_mutate_state_version

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::{NoopPolicyProvider, PolicyProvider};
use ettlex_mcp::auth::AuthConfig;
use ettlex_mcp::context::RequestContext;
use ettlex_mcp::error::McpResult;
use ettlex_mcp::server::{McpResponse, McpServer, McpToolCall};
use ettlex_store::cas::FsStore;
use ettlex_store::file_policy_provider::FilePolicyProvider;
use rusqlite::Connection;
use serde_json::{json, Value};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct Harness {
    _tmp: TempDir,
    pub conn: Connection,
    pub cas: FsStore,
    pub server: McpServer,
}

impl Harness {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("test.db");
        let cas_path = tmp.path().join("cas");
        let mut conn = Connection::open(&db).unwrap();
        ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
        let cas = FsStore::new(&cas_path);
        let server = McpServer::new(AuthConfig::with_token("t:dev"), 10 * 1024 * 1024);
        Self {
            _tmp: tmp,
            conn,
            cas,
            server,
        }
    }

    fn call(&mut self, tool: &str, params: Value) -> McpResponse {
        self.call_with_provider(tool, params, &NoopPolicyProvider)
    }

    fn call_with_provider(
        &mut self,
        tool: &str,
        params: Value,
        provider: &dyn PolicyProvider,
    ) -> McpResponse {
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
            provider,
            &NoopApprovalRouter,
        )
    }

    fn state_version(&self) -> u64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
            .unwrap()
    }

    fn seed_decision(&mut self) {
        self.conn
            .execute_batch(
                "INSERT OR IGNORE INTO decisions
                 (decision_id, title, status, decision_text, rationale,
                  evidence_kind, evidence_hash, created_at, updated_at)
                 VALUES ('d:test:1', 'Use Rust', 'accepted', 'We will use Rust',
                         'Performance', 'none', '', 0, 0);",
            )
            .unwrap();
    }
}

fn is_ok(r: &McpResponse) -> bool {
    matches!(&r.result, McpResult::Ok(_))
}

fn result_value(r: &McpResponse) -> &Value {
    match &r.result {
        McpResult::Ok(v) => v,
        McpResult::Err(e) => panic!("Expected Ok, got Err: {:?}", e),
    }
}

fn error_code(r: &McpResponse) -> &str {
    match &r.result {
        McpResult::Err(e) => &e.error_code,
        McpResult::Ok(v) => panic!("Expected Err, got Ok: {}", v),
    }
}

// ---------------------------------------------------------------------------
// S-MT-HP-1: state_get_version
// ---------------------------------------------------------------------------

#[test]
fn test_state_get_version_returns_version() {
    let mut h = Harness::new();
    let resp = h.call("state_get_version", json!({}));
    assert!(is_ok(&resp), "state_get_version must succeed");
    let v = result_value(&resp);
    assert!(
        v.get("state_version").is_some(),
        "response must contain state_version"
    );
}

// ---------------------------------------------------------------------------
// S-MT-HP-7: decision_get — returns decision JSON
// ---------------------------------------------------------------------------

#[test]
fn test_decision_get_happy_path() {
    let mut h = Harness::new();
    h.seed_decision();
    let resp = h.call("decision_get", json!({ "decision_id": "d:test:1" }));
    assert!(is_ok(&resp), "decision_get must succeed");
    let v = result_value(&resp);
    assert_eq!(v["decision_id"], "d:test:1");
    assert_eq!(v["status"], "accepted");
}

// ---------------------------------------------------------------------------
// S-MT-HP-8: decision_list — fresh DB returns empty items, not error
// ---------------------------------------------------------------------------

#[test]
fn test_decision_list_returns_empty_not_error() {
    let mut h = Harness::new();
    let resp = h.call("decision_list", json!({}));
    assert!(is_ok(&resp), "decision_list must succeed on empty DB");
    let v = result_value(&resp);
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty(), "no decisions yet → empty list");
}

// ---------------------------------------------------------------------------
// S-MT-HP-15: profile_resolve — create profile via apply, then resolve it
// ---------------------------------------------------------------------------

#[test]
fn test_profile_resolve_happy_path() {
    let mut h = Harness::new();

    // Create a profile
    let create_resp = h.call(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "ProfileCreate",
                "profile_ref": "test_profile@0",
                "payload_json": { "predicate_evaluation_enabled": false },
            }
        }),
    );
    assert!(is_ok(&create_resp), "ProfileCreate must succeed");

    let resp = h.call(
        "profile_resolve",
        json!({ "profile_ref": "test_profile@0" }),
    );
    assert!(is_ok(&resp), "profile_resolve must succeed");
    let v = result_value(&resp);
    assert_eq!(v["profile_ref"], "test_profile@0");
    assert!(v.get("payload").is_some(), "response must contain payload");
}

// ---------------------------------------------------------------------------
// S-MT-HP-16: approval_list — fresh DB returns empty items, not error
// ---------------------------------------------------------------------------

#[test]
fn test_approval_list_returns_empty_not_error() {
    let mut h = Harness::new();
    let resp = h.call("approval_list", json!({}));
    assert!(is_ok(&resp), "approval_list must succeed on empty DB");
    let v = result_value(&resp);
    let items = v["items"].as_array().unwrap();
    assert!(items.is_empty(), "no approvals yet → empty list");
}

// ---------------------------------------------------------------------------
// S-MT-HP-17: policy_export — with FilePolicyProvider + real policy
// ---------------------------------------------------------------------------

#[test]
fn test_policy_export_happy_path() {
    let mut h = Harness::new();
    // Create policy file via MCP apply
    let policy_text = "# Test policy\n<!-- HANDOFF: START -->\ndo the thing\n<!-- HANDOFF: END -->";
    let policy_dir = h._tmp.path().join("policies");
    std::fs::create_dir_all(&policy_dir).unwrap();
    let provider = FilePolicyProvider::new(&policy_dir);

    let create_resp = h.call_with_provider(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "PolicyCreate",
                "policy_ref": "export_test@0",
                "text": policy_text,
            }
        }),
        &provider,
    );
    assert!(
        is_ok(&create_resp),
        "PolicyCreate must succeed: {:?}",
        create_resp.result
    );

    let resp = h.call_with_provider(
        "policy_export",
        json!({
            "policy_ref": "export_test@0",
            "export_kind": "codegen_handoff",
        }),
        &provider,
    );
    assert!(is_ok(&resp), "policy_export must succeed");
    let v = result_value(&resp);
    assert_eq!(v["policy_ref"], "export_test@0");
    assert_eq!(v["export_kind"], "codegen_handoff");
    assert!(v["text"].as_str().unwrap().contains("do the thing"));
}

// ---------------------------------------------------------------------------
// S-MT-ERR-3: decision_get — missing decision → NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_decision_get_missing_returns_not_found() {
    let mut h = Harness::new();
    let resp = h.call("decision_get", json!({ "decision_id": "d:missing" }));
    assert_eq!(error_code(&resp), "NotFound");
}

// ---------------------------------------------------------------------------
// S-MT-ERR-4: manifest_get_by_digest — nonexistent digest → MissingBlob
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_get_by_digest_bad_digest_returns_missing_blob() {
    let mut h = Harness::new();
    let resp = h.call(
        "manifest_get_by_digest",
        json!({ "manifest_digest": "deadbeef0000000000000000000000000000000000000000000000000000dead" }),
    );
    // MissingBlob or NotFound depending on whether the digest lookup or CAS fetch fails
    let code = error_code(&resp);
    assert!(
        code == "MissingBlob" || code == "NotFound",
        "Expected MissingBlob or NotFound, got: {}",
        code
    );
}

// ---------------------------------------------------------------------------
// S-MT-ERR-6: profile_resolve — missing ref → ProfileNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_profile_resolve_missing_ref_returns_not_found() {
    let mut h = Harness::new();
    let resp = h.call(
        "profile_resolve",
        json!({ "profile_ref": "missing_profile@0" }),
    );
    assert_eq!(error_code(&resp), "ProfileNotFound");
}

// ---------------------------------------------------------------------------
// S-MT-ERR-7: policy_export — nonexistent policy_ref → PolicyNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_policy_export_nonexistent_returns_policy_not_found() {
    let mut h = Harness::new();
    let policy_dir = h._tmp.path().join("policies");
    std::fs::create_dir_all(&policy_dir).unwrap();
    let provider = FilePolicyProvider::new(&policy_dir);

    let resp = h.call_with_provider(
        "policy_export",
        json!({
            "policy_ref": "nonexistent@0",
            "export_kind": "codegen_handoff",
        }),
        &provider,
    );
    assert_eq!(error_code(&resp), "PolicyNotFound");
}

// ---------------------------------------------------------------------------
// S-MT-ERR-8: policy_export — unknown export_kind → PolicyExportFailed
// ---------------------------------------------------------------------------

#[test]
fn test_policy_export_unknown_kind_returns_policy_export_failed() {
    let mut h = Harness::new();
    let policy_dir = h._tmp.path().join("policies");
    std::fs::create_dir_all(&policy_dir).unwrap();
    let provider = FilePolicyProvider::new(&policy_dir);

    // Create a policy file first
    let create_resp = h.call_with_provider(
        "ettlex_apply",
        json!({
            "command": {
                "tag": "PolicyCreate",
                "policy_ref": "kind_test@0",
                "text": "# Policy without HANDOFF markers",
            }
        }),
        &provider,
    );
    assert!(is_ok(&create_resp), "PolicyCreate must succeed");

    let resp = h.call_with_provider(
        "policy_export",
        json!({
            "policy_ref": "kind_test@0",
            "export_kind": "unknown_export_kind_xyz",
        }),
        &provider,
    );
    assert_eq!(error_code(&resp), "PolicyExportFailed");
}

// ---------------------------------------------------------------------------
// S-MT-INV-2: state_get_version returns V+1 after any Apply command
// ---------------------------------------------------------------------------

#[test]
fn test_state_version_increments_after_apply() {
    let mut h = Harness::new();

    let v0 = result_value(&h.call("state_get_version", json!({})))["state_version"]
        .as_u64()
        .unwrap();

    // Apply one write command
    h.call(
        "ettlex_apply",
        json!({ "command": { "tag": "EttleCreate", "title": "My Ettle" } }),
    );

    let v1 = result_value(&h.call("state_get_version", json!({})))["state_version"]
        .as_u64()
        .unwrap();

    // state_version from StateGetVersion now uses command_log count (Slice 02 rename).
    // After one successful write, command_log has 1 row → state_version = 1.
    let mcp_log_v1 = h.state_version();
    assert_eq!(mcp_log_v1, 1, "command_log must increment after apply");
    // MCP state_version also reflects command_log count.
    assert_eq!(v1, v0 + 1, "state_version increments by 1 after apply");
}
