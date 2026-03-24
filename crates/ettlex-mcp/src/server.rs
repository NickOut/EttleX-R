//! MCP server — dispatch table and request lifecycle.

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use serde_json::Value;

use crate::auth::AuthConfig;
use crate::context::RequestContext;
use crate::error::{McpError, MCP_AUTH_REQUIRED, MCP_REQUEST_TOO_LARGE, MCP_TOOL_NOT_FOUND};
pub use crate::error::{McpResponse, McpResult};
use crate::tools::{
    apply, approval, constraint, decision, ettle, group, policy, predicate, profile, relation,
    snapshot, state,
};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// An inbound MCP tool call, as constructed by the caller (test harness or main).
pub struct McpToolCall {
    /// Name of the tool to invoke, e.g. `"ettle_list"`.
    pub tool_name: String,
    /// JSON parameters for the tool.
    pub params: Value,
    /// Per-request context (correlation_id, etc.).
    pub context: RequestContext,
    /// Bearer token presented by the caller.
    pub auth_token: Option<String>,
    /// Pre-calculated payload size in bytes (used for size guard).
    pub payload_size: usize,
}

// ---------------------------------------------------------------------------
// McpServer
// ---------------------------------------------------------------------------

/// Stateless MCP dispatch server.
///
/// Holds configuration only; all I/O resources are passed per-call.
#[derive(Debug, Clone)]
pub struct McpServer {
    auth: AuthConfig,
    max_request_bytes: usize,
}

impl McpServer {
    /// Create a new server with the given auth configuration and payload size limit.
    pub fn new(auth: AuthConfig, max_request_bytes: usize) -> Self {
        Self {
            auth,
            max_request_bytes,
        }
    }

    /// Dispatch a tool call, returning a response with the correlation_id echoed.
    pub fn dispatch(
        &self,
        call: McpToolCall,
        conn: &mut Connection,
        cas: &FsStore,
        policy_provider: &dyn PolicyProvider,
        approval_router: &dyn ApprovalRouter,
    ) -> McpResponse {
        let correlation_id = call.context.correlation_id.clone();

        let result = self.dispatch_inner(call, conn, cas, policy_provider, approval_router);
        McpResponse {
            correlation_id,
            result,
        }
    }

    fn dispatch_inner(
        &self,
        call: McpToolCall,
        conn: &mut Connection,
        cas: &FsStore,
        policy_provider: &dyn PolicyProvider,
        approval_router: &dyn ApprovalRouter,
    ) -> McpResult {
        // 1. Size guard (checked before auth to prevent DoS parse)
        if call.payload_size > self.max_request_bytes {
            return McpResult::Err(McpError::new(
                MCP_REQUEST_TOO_LARGE,
                format!(
                    "payload size {} exceeds limit {}",
                    call.payload_size, self.max_request_bytes
                ),
            ));
        }

        // 2. Auth guard
        if let Err(e) = self.auth.validate(&call.auth_token) {
            return McpResult::Err(McpError::new(MCP_AUTH_REQUIRED, e.message));
        }

        // 3. Route by tool name
        let p = &call.params;
        match call.tool_name.as_str() {
            // ── Write ──────────────────────────────────────────────────────
            "ettlex_apply" => apply::handle_apply(p, conn, cas, policy_provider, approval_router),

            // ── Relation (read) ────────────────────────────────────────────
            "relation_get" => relation::handle_relation_get_tool(p, conn, cas, policy_provider),
            "relation_list" => relation::handle_relation_list_tool(p, conn, cas, policy_provider),

            // ── Group (read) ────────────────────────────────────────────────
            "group_get" => group::handle_group_get_tool(p, conn, cas, policy_provider),
            "group_list" => group::handle_group_list_tool(p, conn, cas, policy_provider),
            "group_member_list" => {
                group::handle_group_member_list_tool(p, conn, cas, policy_provider)
            }

            // ── Ettle ──────────────────────────────────────────────────────
            "ettle_get" => ettle::handle_ettle_get(p, conn, cas, policy_provider),
            "ettle_list" => ettle::handle_ettle_list(p, conn, cas, policy_provider),

            // ── Ettle (decisions) ──────────────────────────────────────────
            "ettle_list_decisions" => {
                ettle::handle_ettle_list_decisions(p, conn, cas, policy_provider)
            }

            // ── Constraint ─────────────────────────────────────────────────
            "constraint_get" => constraint::handle_constraint_get(p, conn, cas, policy_provider),
            "constraint_list_by_family" => {
                constraint::handle_constraint_list_by_family(p, conn, cas, policy_provider)
            }

            // ── Decision ───────────────────────────────────────────────────
            "decision_get" => decision::handle_decision_get(p, conn, cas, policy_provider),
            "decision_list" => decision::handle_decision_list(p, conn, cas, policy_provider),
            "decision_list_by_target" => {
                decision::handle_decision_list_by_target(p, conn, cas, policy_provider)
            }

            // ── State ──────────────────────────────────────────────────────
            "state_get_version" => state::handle_state_get_version(p, conn, cas, policy_provider),

            // ── Snapshot ───────────────────────────────────────────────────
            "snapshot_list" => snapshot::handle_snapshot_list(p, conn, cas, policy_provider),
            "snapshot_get" => snapshot::handle_snapshot_get(p, conn, cas, policy_provider),
            "snapshot_get_head" => {
                snapshot::handle_snapshot_get_head(p, conn, cas, policy_provider)
            }
            "snapshot_get_manifest" => {
                snapshot::handle_snapshot_get_manifest(p, conn, cas, policy_provider)
            }
            "snapshot_diff" => snapshot::handle_snapshot_diff(p, conn, cas, policy_provider),
            "manifest_get_by_digest" => {
                snapshot::handle_manifest_get_by_digest(p, conn, cas, policy_provider)
            }

            // ── Policy ─────────────────────────────────────────────────────
            "policy_get" => policy::handle_policy_get(p, conn, cas, policy_provider),
            "policy_list" => policy::handle_policy_list(p, conn, cas, policy_provider),
            "policy_export" => policy::handle_policy_export(p, conn, cas, policy_provider),
            "policy_project_for_handoff" => {
                policy::handle_policy_project_for_handoff(p, conn, cas, policy_provider)
            }

            // ── Profile ────────────────────────────────────────────────────
            "profile_get" => profile::handle_profile_get(p, conn, cas, policy_provider),
            "profile_list" => profile::handle_profile_list(p, conn, cas, policy_provider),
            "profile_get_default" => {
                profile::handle_profile_get_default(p, conn, cas, policy_provider)
            }
            "profile_resolve" => profile::handle_profile_resolve(p, conn, cas, policy_provider),

            // ── Approval ───────────────────────────────────────────────────
            "approval_get" => approval::handle_approval_get(p, conn, cas, policy_provider),
            "approval_list" => approval::handle_approval_list(p, conn, cas, policy_provider),

            // ── Predicate ──────────────────────────────────────────────────
            "constraint_predicates_preview" => {
                predicate::handle_predicate_preview(p, conn, cas, policy_provider)
            }

            // ── Unknown ────────────────────────────────────────────────────
            _ => McpResult::Err(McpError::new(
                MCP_TOOL_NOT_FOUND,
                format!("unknown tool: '{}'", call.tool_name),
            )),
        }
    }
}
