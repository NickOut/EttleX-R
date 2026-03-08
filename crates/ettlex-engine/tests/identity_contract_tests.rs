//! Identity contract tests — ettle:store ordinal 3.
//!
//! Verifies that:
//! - Caller-supplied `ettle_id` / `ep_id` in create commands is rejected.
//! - IDs are auto-generated and returned to the caller.
//!
//! Scenario → test mapping:
//!   S-ID-1   test_ettle_create_generates_id
//!   S-ID-2   test_ep_create_generates_id
//!   S-ID-3   test_ettle_create_rejects_supplied_ettle_id
//!   S-ID-4   test_ep_create_rejects_supplied_ep_id
//!   S-ID-5   test_ettle_create_empty_title_fails
//!   S-ID-6   test_ep_create_missing_ettle_fails
//!   S-ID-7   test_ettle_create_max_length_title_succeeds
//!   S-ID-8   test_ep_create_ordinal_conflict_fails
//!   S-ID-9   test_ettle_create_id_ulid_format
//!   S-ID-10  test_ep_create_id_ulid_format
//!   S-ID-11  test_ettle_create_successive_calls_distinct_ids
//!   S-ID-12  test_ettle_create_identical_title_distinct_ids
//!   S-ID-13  test_ettle_create_then_get_consistent
//!   S-ID-14  DEFERRED: concurrent EttleCreate producing distinct ULIDs — needs thread-parallel harness
//!   S-ID-15  DEFERRED: seed importer rejection of ettle_id — seed importer is separate subsystem

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand, McpCommandResult};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path))
}

// ---------------------------------------------------------------------------
// S-ID-1: EttleCreate with no ettle_id generates a ULID and returns it
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_generates_id() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "My Ettle".to_string(),
        ettle_id: None,
    };
    let (result, _sv) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!("Expected EttleCreate result");
    };
    assert!(!ettle_id.is_empty(), "ettle_id must be generated");
}

// ---------------------------------------------------------------------------
// S-ID-2: EpCreate with no ep_id generates a ULID and returns it
// ---------------------------------------------------------------------------

#[test]
fn test_ep_create_generates_id() {
    let (_dir, mut conn, cas) = setup();
    // First create an ettle to attach the EP to
    let create_ettle = McpCommand::EttleCreate {
        title: "Parent Ettle".to_string(),
        ettle_id: None,
    };
    let (ettle_result, _) = apply_mcp_command(
        create_ettle,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = ettle_result else {
        panic!("Expected EttleCreate");
    };

    let cmd = McpCommand::EpCreate {
        ettle_id: ettle_id.clone(),
        ordinal: 0,
        normative: true,
        why: "why".to_string(),
        what: "what".to_string(),
        how: "how".to_string(),
        ep_id: None,
    };
    let (result, _sv) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EpCreate { ep_id } = result else {
        panic!("Expected EpCreate result");
    };
    assert!(!ep_id.is_empty(), "ep_id must be generated");
}

// ---------------------------------------------------------------------------
// S-ID-3: EttleCreate rejects supplied ettle_id → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_rejects_supplied_ettle_id() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "My Ettle".to_string(),
        ettle_id: Some("ettle:caller-supplied:0".to_string()),
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
        result.is_err(),
        "EttleCreate with supplied ettle_id must fail"
    );
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-ID-4: EpCreate rejects supplied ep_id → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_ep_create_rejects_supplied_ep_id() {
    let (_dir, mut conn, cas) = setup();
    // Create ettle first
    let (ettle_result, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: "Ettle".to_string(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = ettle_result else {
        panic!()
    };

    let cmd = McpCommand::EpCreate {
        ettle_id,
        ordinal: 0,
        normative: true,
        why: "w".to_string(),
        what: "w".to_string(),
        how: "h".to_string(),
        ep_id: Some("ep:caller-supplied:0".to_string()),
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "EpCreate with supplied ep_id must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-ID-5: EttleCreate with empty title fails → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_empty_title_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: String::new(),
        ettle_id: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "EttleCreate with empty title must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-ID-6: EpCreate referencing missing ettle fails → NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_ep_create_missing_ettle_fails() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EpCreate {
        ettle_id: "ettle:does-not-exist".to_string(),
        ordinal: 0,
        normative: true,
        why: "w".to_string(),
        what: "w".to_string(),
        how: "h".to_string(),
        ep_id: None,
    };
    let result = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "EpCreate for missing ettle must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// S-ID-7: EttleCreate with max-length title (255 chars) succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_max_length_title_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let title = "T".repeat(255);
    let cmd = McpCommand::EttleCreate {
        title,
        ettle_id: None,
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
        "EttleCreate with 255-char title must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-ID-8: EpCreate with ordinal conflict fails → AlreadyExists (or similar)
// ---------------------------------------------------------------------------

#[test]
fn test_ep_create_ordinal_conflict_fails() {
    let (_dir, mut conn, cas) = setup();
    // Create ettle
    let (ettle_result, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: "Ettle".to_string(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = ettle_result else {
        panic!()
    };

    // Create EP with ordinal 0
    apply_mcp_command(
        McpCommand::EpCreate {
            ettle_id: ettle_id.clone(),
            ordinal: 0,
            normative: true,
            why: "w".to_string(),
            what: "w".to_string(),
            how: "h".to_string(),
            ep_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Create second EP with same ordinal → must fail
    let result = apply_mcp_command(
        McpCommand::EpCreate {
            ettle_id,
            ordinal: 0,
            normative: true,
            why: "w2".to_string(),
            what: "w2".to_string(),
            how: "h2".to_string(),
            ep_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "EpCreate with duplicate ordinal must fail");
}

// ---------------------------------------------------------------------------
// S-ID-9: Generated ettle_id matches expected format (contains "ettle:")
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_id_ulid_format() {
    let (_dir, mut conn, cas) = setup();
    let cmd = McpCommand::EttleCreate {
        title: "Format Check".to_string(),
        ettle_id: None,
    };
    let (result, _) = apply_mcp_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = result else {
        panic!()
    };
    assert!(
        ettle_id.starts_with("ettle:"),
        "ettle_id must start with 'ettle:': got '{}'",
        ettle_id
    );
}

// ---------------------------------------------------------------------------
// S-ID-10: Generated ep_id matches expected format (contains "ep:")
// ---------------------------------------------------------------------------

#[test]
fn test_ep_create_id_ulid_format() {
    let (_dir, mut conn, cas) = setup();
    let (ettle_result, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: "E".to_string(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = ettle_result else {
        panic!()
    };

    let (result, _) = apply_mcp_command(
        McpCommand::EpCreate {
            ettle_id,
            ordinal: 0,
            normative: true,
            why: "w".to_string(),
            what: "w".to_string(),
            how: "h".to_string(),
            ep_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EpCreate { ep_id } = result else {
        panic!()
    };
    assert!(
        ep_id.starts_with("ep:"),
        "ep_id must start with 'ep:': got '{}'",
        ep_id
    );
}

// ---------------------------------------------------------------------------
// S-ID-11: Two successive EttleCreate calls produce distinct ettle_ids
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_successive_calls_distinct_ids() {
    let (_dir, mut conn, cas) = setup();
    let mk_cmd = || McpCommand::EttleCreate {
        title: "Same Title".to_string(),
        ettle_id: None,
    };

    let (r1, _) = apply_mcp_command(
        mk_cmd(),
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let (r2, _) = apply_mcp_command(
        mk_cmd(),
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let McpCommandResult::EttleCreate { ettle_id: id1 } = r1 else {
        panic!()
    };
    let McpCommandResult::EttleCreate { ettle_id: id2 } = r2 else {
        panic!()
    };
    assert_ne!(id1, id2, "Successive EttleCreate must produce distinct IDs");
}

// ---------------------------------------------------------------------------
// S-ID-12: Repeated EttleCreate with identical title produces distinct Ettles
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_identical_title_distinct_ids() {
    let (_dir, mut conn, cas) = setup();
    let (r1, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: "Dup Title".to_string(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let (r2, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: "Dup Title".to_string(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let McpCommandResult::EttleCreate { ettle_id: id1 } = r1 else {
        panic!()
    };
    let McpCommandResult::EttleCreate { ettle_id: id2 } = r2 else {
        panic!()
    };
    assert_ne!(id1, id2, "Distinct Ettles must have distinct IDs");

    // Both must exist in DB
    let count: u64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 2, "Both ettles must be persisted");
}

// ---------------------------------------------------------------------------
// S-ID-13: EttleCreate followed by ettle.get returns consistent state
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_create_then_get_consistent() {
    let (_dir, mut conn, cas) = setup();
    let title = "Consistency Check".to_string();
    let (r, _) = apply_mcp_command(
        McpCommand::EttleCreate {
            title: title.clone(),
            ettle_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();
    let McpCommandResult::EttleCreate { ettle_id } = r else {
        panic!()
    };

    // Retrieve from DB
    let stored_title: String = conn
        .query_row(
            "SELECT title FROM ettles WHERE id = ?1",
            [&ettle_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        stored_title, title,
        "Stored title must match supplied title"
    );
}

// DEFERRED: S-ID-14 — Concurrent EttleCreate producing distinct ULIDs
// Requires thread-parallel test harness; deferred to load test phase.

// DEFERRED: S-ID-15 — Seed importer rejection of ettle_id
// Seed importer is a separate subsystem; scoped separately.
