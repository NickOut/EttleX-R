//! ettlex-mcp stdio transport — MCP JSON-RPC 2.0 server for Claude Desktop.
//!
//! Reads newline-delimited JSON from stdin, writes responses to stdout.
//! Diagnostics go to stderr.
//!
//! ## Usage
//!
//! ```text
//! ettlex-mcp --db /path/to/repo.db [--cas /path/to/cas]
//! ETTLEX_DB=/path/to/repo.db ettlex-mcp
//! ```
//!
//! ## Claude Desktop configuration (`claude_desktop_config.json`)
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "ettlex": {
//!       "command": "/path/to/ettlex-mcp",
//!       "args": ["--db", "/path/to/your/repo.db"]
//!     }
//!   }
//! }
//! ```

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_mcp::auth::AuthConfig;
use ettlex_mcp::context::RequestContext;
use ettlex_mcp::server::{McpResult, McpServer, McpToolCall};
use ettlex_store::cas::FsStore;
use ettlex_store::migrations::apply_migrations;
use rusqlite::Connection;
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let db_path = resolve_db_path(&args);
    let cas_path = resolve_cas_path(&args, &db_path);

    // Open DB and apply migrations
    let mut conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => {
            let _ = writeln!(
                io::stderr(),
                "ettlex-mcp: failed to open DB {:?}: {}",
                db_path,
                e
            );
            std::process::exit(1);
        }
    };
    if let Err(e) = apply_migrations(&mut conn) {
        let _ = writeln!(io::stderr(), "ettlex-mcp: migration failed: {}", e);
        std::process::exit(1);
    }

    let cas = FsStore::new(cas_path);
    let server = McpServer::new(AuthConfig::disabled(), 1024 * 1024);

    // MCP stdio loop
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                let _ = writeln!(io::stderr(), "ettlex-mcp: stdin read error: {}", e);
                break;
            }
        };

        if line.trim().is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(e) => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                let _ = writeln!(out, "{}", resp);
                let _ = out.flush();
                continue;
            }
        };

        // Notifications have no `id` field — do not respond
        if !msg
            .as_object()
            .map(|o| o.contains_key("id"))
            .unwrap_or(false)
        {
            continue;
        }

        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        let method = msg
            .get("method")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let params = msg.get("params").cloned().unwrap_or(Value::Null);

        let resp = dispatch_jsonrpc(&method, &params, id, &server, &mut conn, &cas);
        let _ = writeln!(out, "{}", resp);
        let _ = out.flush();
    }
}

// ---------------------------------------------------------------------------
// JSON-RPC dispatch
// ---------------------------------------------------------------------------

fn dispatch_jsonrpc(
    method: &str,
    params: &Value,
    id: Value,
    server: &McpServer,
    conn: &mut Connection,
    cas: &FsStore,
) -> Value {
    match method {
        "initialize" => jsonrpc_result(id, handle_initialize()),
        "ping" => jsonrpc_result(id, json!({})),
        "tools/list" => jsonrpc_result(id, handle_tools_list()),
        "tools/call" => {
            let result = handle_tools_call(params, server, conn, cas);
            jsonrpc_result(id, result)
        }
        // Respond to unknown methods with method-not-found
        _ => jsonrpc_error(id, -32601, format!("Method not found: {}", method)),
    }
}

fn jsonrpc_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn jsonrpc_error(id: Value, code: i32, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

// ---------------------------------------------------------------------------
// initialize
// ---------------------------------------------------------------------------

fn handle_initialize() -> Value {
    json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": {}
        },
        "serverInfo": {
            "name": "ettlex-mcp",
            "version": "0.1.0"
        }
    })
}

// ---------------------------------------------------------------------------
// tools/list
// ---------------------------------------------------------------------------

fn handle_tools_list() -> Value {
    let tools = vec![
        tool_def(
            "ettlex_apply",
            "Apply a write command (EttleCreate, EttleUpdate, EttleTombstone, SnapshotCommit, RelationCreate, RelationUpdate, RelationTombstone, GroupCreate, GroupTombstone, GroupMemberAdd, GroupMemberRemove, ProfileCreate, ProfileSetDefault, PolicyCreate).",
            json!({
                "type": "object",
                "required": ["command"],
                "properties": {
                    "command": {
                        "type": "object",
                        "description": "Tagged command object. Required field: tag. Tags: EttleCreate {title, why?, what?, how?, reasoning_link_id?, reasoning_link_type?}, EttleUpdate {ettle_id, title?, why?, what?, how?, reasoning_link_id?, reasoning_link_type?}, EttleTombstone {ettle_id}, SnapshotCommit {leaf_ep_id, policy_ref?}, RelationCreate {relation_type, source_ettle_id, target_ettle_id, properties_json?}, RelationUpdate {relation_id, properties_json}, RelationTombstone {relation_id}, GroupCreate {name}, GroupTombstone {group_id}, GroupMemberAdd {group_id, ettle_id}, GroupMemberRemove {group_id, ettle_id}, ProfileCreate {profile_ref, payload_json}, ProfileSetDefault {profile_ref}, PolicyCreate {policy_ref, text}."
                    },
                    "expected_state_version": {
                        "type": "integer",
                        "description": "Optional OCC guard. Returns HeadMismatch if current version differs."
                    }
                }
            }),
        ),
        tool_def(
            "ettle_get",
            "Get a single ettle by ID.",
            json!({
                "type": "object",
                "required": ["ettle_id"],
                "properties": {
                    "ettle_id": { "type": "string", "description": "Ettle ID (ettle:...)" }
                }
            }),
        ),
        tool_def(
            "ettle_list",
            "List ettles with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max results 1–500 (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" },
                    "include_tombstoned": { "type": "boolean", "description": "If true, include tombstoned ettles (default false)" }
                }
            }),
        ),
        tool_def(
            "snapshot_get",
            "Get a snapshot ledger row.",
            json!({
                "type": "object",
                "required": ["snapshot_id"],
                "properties": {
                    "snapshot_id": { "type": "string", "description": "Snapshot ID (snapshot:...)" }
                }
            }),
        ),
        tool_def(
            "snapshot_list",
            "List snapshots with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" },
                    "cursor": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "snapshot_get_head",
            "Get the manifest digest of the most recently committed snapshot for an ettle.",
            json!({
                "type": "object",
                "required": ["ettle_id"],
                "properties": {
                    "ettle_id": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "snapshot_get_manifest",
            "Get raw manifest bytes for a snapshot.",
            json!({
                "type": "object",
                "required": ["snapshot_id"],
                "properties": {
                    "snapshot_id": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "snapshot_diff",
            "Compute a structured diff between two snapshots.",
            json!({
                "type": "object",
                "required": ["a_snapshot_id", "b_snapshot_id"],
                "properties": {
                    "a_snapshot_id": { "type": "string" },
                    "b_snapshot_id": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "policy_get",
            "Read a policy document by reference.",
            json!({
                "type": "object",
                "required": ["policy_ref"],
                "properties": {
                    "policy_ref": { "type": "string", "description": "e.g. policy/name@version" }
                }
            }),
        ),
        tool_def(
            "policy_list",
            "List available policies with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" },
                    "cursor": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "policy_project_for_handoff",
            "Produce a deterministic projection of a policy's HANDOFF obligations for code-generator prompts.",
            json!({
                "type": "object",
                "required": ["policy_ref"],
                "properties": {
                    "policy_ref": { "type": "string" },
                    "profile_ref": { "type": ["string", "null"] }
                }
            }),
        ),
        tool_def(
            "profile_get",
            "Get a profile by reference.",
            json!({
                "type": "object",
                "required": ["profile_ref"],
                "properties": {
                    "profile_ref": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "profile_list",
            "List profiles with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" },
                    "cursor": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "profile_get_default",
            "Get the default profile.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool_def(
            "approval_get",
            "Get an approval request by token.",
            json!({
                "type": "object",
                "required": ["approval_token"],
                "properties": {
                    "approval_token": { "type": "string" }
                }
            }),
        ),
        tool_def(
            "constraint_predicates_preview",
            "Preview constraint predicate resolution without side-effects.",
            json!({
                "type": "object",
                "required": ["candidates"],
                "properties": {
                    "profile_ref": { "type": ["string", "null"] },
                    "context": { "type": "object" },
                    "candidates": {
                        "type": "array",
                        "items": { "type": "string" }
                    }
                }
            }),
        ),
        // ── Ettle decisions ──────────────────────────────────────────────
        tool_def(
            "ettle_list_decisions",
            "List decisions linked to an ettle, optionally including ancestor decisions.",
            json!({
                "type": "object",
                "required": ["ettle_id"],
                "properties": {
                    "ettle_id": { "type": "string", "description": "Ettle ID (ettle:...)" },
                    "include_ancestors": { "type": "boolean", "description": "Include decisions from ancestor ettles (default false)" }
                }
            }),
        ),
        // ── Decision ─────────────────────────────────────────────────────
        tool_def(
            "decision_get",
            "Get a single decision by ID.",
            json!({
                "type": "object",
                "required": ["decision_id"],
                "properties": {
                    "decision_id": { "type": "string", "description": "Decision ID (d:...)" }
                }
            }),
        ),
        tool_def(
            "decision_list",
            "List decisions with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max results (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" }
                }
            }),
        ),
        tool_def(
            "decision_list_by_target",
            "List decisions linked to a specific target (EP or ettle).",
            json!({
                "type": "object",
                "required": ["target_kind", "target_id"],
                "properties": {
                    "target_kind": { "type": "string", "description": "Target kind: 'ep' or 'ettle'" },
                    "target_id": { "type": "string", "description": "Target ID" },
                    "include_tombstoned": { "type": "boolean", "description": "Include tombstoned decisions (default false)" }
                }
            }),
        ),
        // ── State ─────────────────────────────────────────────────────────
        tool_def(
            "state_get_version",
            "Get the current state version (count of committed write commands) and semantic head digest.",
            json!({ "type": "object", "properties": {} }),
        ),
        // ── Manifest ──────────────────────────────────────────────────────
        tool_def(
            "manifest_get_by_digest",
            "Retrieve a snapshot manifest directly by its CAS digest.",
            json!({
                "type": "object",
                "required": ["manifest_digest"],
                "properties": {
                    "manifest_digest": { "type": "string", "description": "SHA-256 hex digest of the manifest" }
                }
            }),
        ),
        // ── Policy export ─────────────────────────────────────────────────
        tool_def(
            "policy_export",
            "Export a policy in a specific format (e.g. codegen_handoff).",
            json!({
                "type": "object",
                "required": ["policy_ref", "export_kind"],
                "properties": {
                    "policy_ref": { "type": "string", "description": "Policy reference (e.g. policy/name@version)" },
                    "export_kind": { "type": "string", "description": "Export format: 'codegen_handoff'" }
                }
            }),
        ),
        // ── Profile resolve ───────────────────────────────────────────────
        tool_def(
            "profile_resolve",
            "Resolve a profile by reference, or resolve the default profile if no reference is given.",
            json!({
                "type": "object",
                "properties": {
                    "profile_ref": { "type": ["string", "null"], "description": "Profile reference, or null for default" }
                }
            }),
        ),
        // ── Approval list ─────────────────────────────────────────────────
        tool_def(
            "approval_list",
            "List approval requests with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max results (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" }
                }
            }),
        ),
        // ── Relation (read) ───────────────────────────────────────────────
        tool_def(
            "relation_get",
            "Get a single relation by ID. Returns the record even if tombstoned.",
            json!({
                "type": "object",
                "required": ["relation_id"],
                "properties": {
                    "relation_id": { "type": "string", "description": "Relation ID (rel:...)" }
                }
            }),
        ),
        tool_def(
            "relation_list",
            "List relations filtered by source and/or target ettle. At least one of source_ettle_id or target_ettle_id must be supplied.",
            json!({
                "type": "object",
                "properties": {
                    "source_ettle_id": { "type": "string", "description": "Filter by source ettle ID" },
                    "target_ettle_id": { "type": "string", "description": "Filter by target ettle ID" },
                    "include_tombstoned": { "type": "boolean", "description": "Include tombstoned relations (default false)" },
                    "limit": { "type": "integer", "description": "Max results (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" }
                }
            }),
        ),
        // ── Group (read) ──────────────────────────────────────────────────
        tool_def(
            "group_get",
            "Get a single group by ID. Returns the record even if tombstoned.",
            json!({
                "type": "object",
                "required": ["group_id"],
                "properties": {
                    "group_id": { "type": "string", "description": "Group ID (grp:...)" }
                }
            }),
        ),
        tool_def(
            "group_list",
            "List groups with pagination.",
            json!({
                "type": "object",
                "properties": {
                    "include_tombstoned": { "type": "boolean", "description": "Include tombstoned groups (default false)" },
                    "limit": { "type": "integer", "description": "Max results (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" }
                }
            }),
        ),
        tool_def(
            "group_member_list",
            "List group memberships filtered by group and/or ettle. At least one of group_id or ettle_id must be supplied.",
            json!({
                "type": "object",
                "properties": {
                    "group_id": { "type": "string", "description": "Filter by group ID" },
                    "ettle_id": { "type": "string", "description": "Filter by ettle ID" },
                    "include_tombstoned": { "type": "boolean", "description": "Include tombstoned memberships (default false)" },
                    "limit": { "type": "integer", "description": "Max results (default 100)" },
                    "cursor": { "type": "string", "description": "Opaque pagination cursor" }
                }
            }),
        ),
    ];
    json!({ "tools": tools })
}

fn tool_def(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

// ---------------------------------------------------------------------------
// tools/call
// ---------------------------------------------------------------------------

fn handle_tools_call(
    params: &Value,
    server: &McpServer,
    conn: &mut Connection,
    cas: &FsStore,
) -> Value {
    let tool_name = match params.get("name").and_then(|n| n.as_str()) {
        Some(n) => n.to_string(),
        None => {
            return tool_error("InvalidInput: missing tool name");
        }
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    let payload_size = arguments.to_string().len();
    let call = McpToolCall {
        tool_name,
        params: arguments,
        context: RequestContext::default(),
        auth_token: None,
        payload_size,
    };

    let response = server.dispatch(call, conn, cas, &NoopPolicyProvider, &NoopApprovalRouter);

    match response.result {
        McpResult::Ok(value) => {
            let text = serde_json::to_string(&value).unwrap_or_default();
            json!({
                "content": [{ "type": "text", "text": text }],
                "isError": false
            })
        }
        McpResult::Err(err) => tool_error(format!("{}: {}", err.error_code, err.message)),
    }
}

fn tool_error(message: impl Into<String>) -> Value {
    json!({
        "content": [{ "type": "text", "text": message.into() }],
        "isError": true
    })
}

// ---------------------------------------------------------------------------
// Path resolution helpers
// ---------------------------------------------------------------------------

fn resolve_db_path(args: &[String]) -> PathBuf {
    // --db <path> takes priority
    if let Some(pos) = args.iter().position(|a| a == "--db") {
        if let Some(p) = args.get(pos + 1) {
            return PathBuf::from(p);
        }
    }
    // ETTLEX_DB env var
    if let Ok(p) = std::env::var("ETTLEX_DB") {
        return PathBuf::from(p);
    }
    // Fallback: ettlex.db in current directory
    PathBuf::from("ettlex.db")
}

fn resolve_cas_path(args: &[String], db_path: &std::path::Path) -> PathBuf {
    // --cas <path> takes priority
    if let Some(pos) = args.iter().position(|a| a == "--cas") {
        if let Some(p) = args.get(pos + 1) {
            return PathBuf::from(p);
        }
    }
    // Default: <db_dir>/cas
    let dir = db_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    dir.join("cas")
}
