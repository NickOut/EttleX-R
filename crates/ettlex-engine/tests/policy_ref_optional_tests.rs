//! Scenario tests for ettle:snapshot_commit_actions_refactor ordinal 1.
//!
//! Verifies that `policy_ref` is optional on SnapshotCommit:
//! - Absent → permissive pass-through (empty string in manifest)
//! - Explicit → used directly
//!
//! Scenario → test mapping:
//!   S-PR-1   test_absent_policy_ref_no_default_permissive
//!   S-PR-2   test_explicit_policy_ref_succeeds
//!   S-PR-4   test_explicit_nonexistent_policy_ref_fails
//!   S-PR-6   test_none_policy_ref_same_as_absent
//!   S-PR-7   test_manifest_always_records_policy_ref
//!   S-PR-10  test_absent_policy_ref_transitions_to_committed
//!   S-PR-11  test_result_tag_snapshot_committed
//!   S-PR-12  test_existing_explicit_policy_ref_unchanged
//!   S-PR-14  test_manifest_bytes_stable_for_absent_policy_ref
//!   DEFERRED S-PR-3  get_default_policy_ref returning non-None — Phase 1 always returns None
//!   DEFERRED S-PR-5  absent policy_ref + default that denies — same rationale
//!   DEFERRED S-PR-8  explicit takes precedence over default — default always None in Phase 1
//!   DEFERRED S-PR-9  absent policy_ref defaulting is deterministic — same rationale
//!   DEFERRED S-PR-13 MCP transport receives None — verified structurally (Option<String> type)

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::engine_command::{apply_engine_command, EngineCommand};
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand};
use ettlex_engine::commands::snapshot::SnapshotOptions;
use ettlex_store::cas::FsStore;
use ettlex_store::file_policy_provider::FilePolicyProvider;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path))
}

fn seed_leaf(conn: &Connection, ep_id: &str, ettle_id: &str) {
    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('{ettle_id}', 'T', NULL, 0, 0, 0, '{{}}');
         INSERT OR IGNORE INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
         VALUES ('{ep_id}', '{ettle_id}', 0, 1, NULL, '{{\"why\":\"w\",\"what\":\"w\",\"how\":\"h\"}}', 0, 0, 0);"
    )).unwrap();
}

fn commit_cmd(leaf_ep_id: &str, policy_ref: Option<String>) -> EngineCommand {
    EngineCommand::SnapshotCommit {
        leaf_ep_id: leaf_ep_id.to_string(),
        policy_ref,
        profile_ref: None,
        options: SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
    }
}

// ---------------------------------------------------------------------------
// S-PR-1: SnapshotCommit succeeds with policy_ref absent (None), no default → permissive
// ---------------------------------------------------------------------------

#[test]
fn test_absent_policy_ref_no_default_permissive() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr1:0", "ettle:pr1");

    let cmd = commit_cmd("ep:pr1:0", None);
    let result = apply_engine_command(
        cmd,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "Absent policy_ref must succeed (permissive): {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-PR-2: SnapshotCommit succeeds with explicit policy_ref
// ---------------------------------------------------------------------------

#[test]
fn test_explicit_policy_ref_succeeds() {
    let (dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr2:0", "ettle:pr2");

    // Create a real policy file so FilePolicyProvider can check it
    let policies_dir = dir.path().join("policies");
    fs::create_dir_all(&policies_dir).unwrap();
    fs::write(policies_dir.join("my_policy@0.md"), "# Policy").unwrap();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = commit_cmd("ep:pr2:0", Some("my_policy@0".to_string()));
    let result = apply_engine_command(cmd, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(
        result.is_ok(),
        "Explicit policy_ref must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-PR-4: SnapshotCommit with absent policy_ref + nonexistent explicit ref → PolicyNotFound
// ---------------------------------------------------------------------------

#[test]
fn test_explicit_nonexistent_policy_ref_fails() {
    let (dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr4:0", "ettle:pr4");

    let policies_dir = dir.path().join("policies");
    fs::create_dir_all(&policies_dir).unwrap();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = commit_cmd("ep:pr4:0", Some("nonexistent@0".to_string()));
    let result = apply_engine_command(cmd, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result.is_err(), "Nonexistent policy_ref must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::PolicyNotFound);
}

// ---------------------------------------------------------------------------
// S-PR-6: None policy_ref behaves identically to absent (both are permissive)
// ---------------------------------------------------------------------------

#[test]
fn test_none_policy_ref_same_as_absent() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr6:0", "ettle:pr6");

    // None and absent (None) are the same thing in Rust Option
    let result = apply_engine_command(
        commit_cmd("ep:pr6:0", None),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "None policy_ref must succeed (permissive): {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-PR-7: Manifest always records policy_ref field (empty string if permissive)
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_always_records_policy_ref() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr7:0", "ettle:pr7");

    let result = apply_engine_command(
        commit_cmd("ep:pr7:0", None),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    use ettlex_engine::commands::engine_command::EngineCommandResult;
    let EngineCommandResult::SnapshotCommit(r) = result else {
        panic!("Expected SnapshotCommit");
    };

    // Load manifest from CAS and check policy_ref field
    let manifest_bytes = cas.read(&r.manifest_digest).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert!(
        manifest.get("policy_ref").is_some(),
        "Manifest must always contain policy_ref field"
    );
    // Permissive pass-through → empty string
    let policy_ref_val = manifest["policy_ref"].as_str().unwrap_or("__missing__");
    assert_eq!(
        policy_ref_val, "",
        "policy_ref must be empty string for permissive pass-through"
    );
}

// ---------------------------------------------------------------------------
// S-PR-10: SnapshotCommit with absent policy_ref transitions to committed state
// ---------------------------------------------------------------------------

#[test]
fn test_absent_policy_ref_transitions_to_committed() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr10:0", "ettle:pr10");

    let result = apply_engine_command(
        commit_cmd("ep:pr10:0", None),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    use ettlex_engine::commands::engine_command::EngineCommandResult;
    let EngineCommandResult::SnapshotCommit(r) = result else {
        panic!("Expected SnapshotCommit");
    };
    assert!(!r.snapshot_id.is_empty(), "snapshot_id must be populated");
    assert!(
        !r.manifest_digest.is_empty(),
        "manifest_digest must be populated"
    );

    // Verify snapshot in DB
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM snapshots WHERE snapshot_id = ?1",
            [&r.snapshot_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "Snapshot must be persisted in DB");
}

// ---------------------------------------------------------------------------
// S-PR-11: Result tag is SnapshotCommitted with permissive pass-through
// ---------------------------------------------------------------------------

#[test]
fn test_result_tag_snapshot_committed() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr11:0", "ettle:pr11");

    // Via McpCommand
    let cmd = McpCommand::SnapshotCommit {
        leaf_ep_id: "ep:pr11:0".to_string(),
        policy_ref: None,
        profile_ref: None,
        dry_run: false,
        expected_head: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "MCP SnapshotCommit with None policy_ref must succeed"
    );

    use ettlex_engine::commands::mcp_command::McpCommandResult;
    let (res, _) = result.unwrap();
    assert!(
        matches!(res, McpCommandResult::SnapshotCommit { .. }),
        "Result must be SnapshotCommit"
    );
}

// ---------------------------------------------------------------------------
// S-PR-12: Existing calls with explicit policy_ref continue to work unchanged
// ---------------------------------------------------------------------------

#[test]
fn test_existing_explicit_policy_ref_unchanged() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr12:0", "ettle:pr12");

    // Explicit policy_ref still works (NoopPolicyProvider allows everything)
    let result = apply_engine_command(
        commit_cmd("ep:pr12:0", Some("any/policy@0".to_string())),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "Explicit policy_ref still works: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-PR-14: Manifest bytes stable for identical state + absent policy_ref
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_bytes_stable_for_absent_policy_ref() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:pr14:0", "ettle:pr14");

    let r1 = apply_engine_command(
        commit_cmd("ep:pr14:0", None),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    use ettlex_engine::commands::engine_command::EngineCommandResult;
    let EngineCommandResult::SnapshotCommit(r1) = r1 else {
        panic!()
    };

    let r2 = apply_engine_command(
        commit_cmd("ep:pr14:0", None),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let EngineCommandResult::SnapshotCommit(r2) = r2 else {
        panic!()
    };

    // Semantic digest should be identical (same content, same policy_ref "")
    assert_eq!(
        r1.semantic_manifest_digest, r2.semantic_manifest_digest,
        "Semantic digest must be stable for identical state + absent policy_ref"
    );
}

// DEFERRED: S-PR-3 — get_default_policy_ref returning non-None
// Phase 1: FilePolicyProvider always returns None. Configurable default is a future EP.

// DEFERRED: S-PR-5 — absent policy_ref + default that denies
// Same rationale as S-PR-3.

// DEFERRED: S-PR-8 — explicit takes precedence over default
// Default is always None in Phase 1; precedence test deferred.

// DEFERRED: S-PR-9 — absent policy_ref defaulting is deterministic
// Default always None; determinism test deferred.

// DEFERRED: S-PR-13 — MCP does not inject policy_ref
// Verified structurally: McpCommand::SnapshotCommit.policy_ref is Option<String>.
// Formal MCP transport test is low priority.
