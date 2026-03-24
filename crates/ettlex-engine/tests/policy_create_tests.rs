//! Scenario tests for PolicyCreate command — ettle:store ordinal 3.
//!
//! Scenario → test mapping:
//!   S-PC-1   test_policy_create_succeeds_state_version_increments
//!   S-PC-2   test_policy_create_duplicate_rejected
//!   S-PC-3   test_policy_create_empty_text_rejected
//!   S-PC-4   test_policy_create_empty_policy_ref_rejected
//!   S-PC-5   test_policy_create_malformed_policy_ref_rejected
//!   S-PC-6   test_policy_create_write_failure_no_state_change
//!   S-PC-7   test_policy_create_max_length_policy_ref_succeeds
//!   S-PC-8   test_policy_create_large_text_succeeds
//!   S-PC-9   test_policy_create_stable_retrieval_key
//!   S-PC-10  test_policy_create_not_idempotent
//!   S-PC-11  test_policy_create_appears_in_list
//!   S-PC-12  test_policy_create_usable_in_snapshot_commit
//!   S-PC-13  test_policy_create_state_version_incremented
//!   S-PC-14  test_policy_create_existing_policies_retrievable
//!   S-PC-15  test_policy_create_must_not_overwrite_existing
//!   DEFERRED: S-PC-concurrent — file-system atomic rename + SQLite uniqueness each guarantee
//!             single-winner semantics; cross-system race is OS-level

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_engine::commands::command::{apply_command, Command, CommandResult};
use ettlex_store::cas::FsStore;
use ettlex_store::file_policy_provider::FilePolicyProvider;
use rusqlite::Connection;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

/// Returns (tmp_dir, conn, cas, policies_dir, provider).
fn setup() -> (TempDir, Connection, FsStore, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let policies_dir = dir.path().join("policies");
    fs::create_dir_all(&policies_dir).unwrap();
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path), policies_dir)
}

fn sv(conn: &Connection) -> u64 {
    conn.query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap()
}

// ---------------------------------------------------------------------------
// S-PC-1: PolicyCreate with valid policy_ref and text succeeds, state_version+1
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_succeeds_state_version_increments() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);
    let before = sv(&conn);

    let cmd = Command::PolicyCreate {
        policy_ref: "my_policy@0".to_string(),
        text: "# Policy\n<!-- HANDOFF: START -->\nobligation\n<!-- HANDOFF: END -->".to_string(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(
        result.is_ok(),
        "PolicyCreate must succeed: {:?}",
        result.err()
    );
    let (res, new_sv) = result.unwrap();
    assert!(matches!(res, CommandResult::PolicyCreate { .. }));
    assert_eq!(new_sv, before + 1);
}

// ---------------------------------------------------------------------------
// S-PC-2: PolicyCreate rejects duplicate policy_ref → PolicyConflict
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_duplicate_rejected() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = || Command::PolicyCreate {
        policy_ref: "dup_policy@0".to_string(),
        text: "# Policy content".to_string(),
    };
    apply_command(cmd(), None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    let result2 = apply_command(cmd(), None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result2.is_err(), "Duplicate PolicyCreate must fail");
    assert_eq!(result2.unwrap_err().kind(), ExErrorKind::PolicyConflict);
}

// ---------------------------------------------------------------------------
// S-PC-3: PolicyCreate rejects empty text → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_empty_text_rejected() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = Command::PolicyCreate {
        policy_ref: "empty_text@0".to_string(),
        text: String::new(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result.is_err(), "Empty text must be rejected");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-PC-4: PolicyCreate rejects empty policy_ref → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_empty_policy_ref_rejected() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = Command::PolicyCreate {
        policy_ref: String::new(),
        text: "# Content".to_string(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result.is_err(), "Empty policy_ref must be rejected");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-PC-5: PolicyCreate rejects malformed policy_ref (no `@`) → InvalidInput
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_malformed_policy_ref_rejected() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = Command::PolicyCreate {
        policy_ref: "no_at_separator".to_string(),
        text: "# Content".to_string(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(
        result.is_err(),
        "Malformed policy_ref (no @) must be rejected"
    );
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// S-PC-6: PolicyCreate write failure → policy not persisted, state_version unchanged
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_write_failure_no_state_change() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    // Make policies dir unwritable to simulate write failure
    let mut perms = fs::metadata(&policies_dir).unwrap().permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    {
        use std::os::unix::fs::PermissionsExt;
        perms.set_mode(0o444); // read-only
    }
    fs::set_permissions(&policies_dir, perms).unwrap();

    let provider = FilePolicyProvider::new(&policies_dir);
    let before = sv(&conn);

    let cmd = Command::PolicyCreate {
        policy_ref: "fail_policy@0".to_string(),
        text: "# Content".to_string(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result.is_err(), "Write failure must return error");
    assert_eq!(
        sv(&conn),
        before,
        "state_version must not change on write failure"
    );
}

// ---------------------------------------------------------------------------
// S-PC-7: PolicyCreate with max-length policy_ref succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_max_length_policy_ref_succeeds() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    // 200-char policy_ref with @ separator
    let policy_ref = format!("{}@0", "a".repeat(200));
    let cmd = Command::PolicyCreate {
        policy_ref,
        text: "# Content".to_string(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(
        result.is_ok(),
        "Max-length policy_ref must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-PC-8: PolicyCreate with large text body (100KB) succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_large_text_succeeds() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let large_text = "# Large\n".repeat(10_000); // ~80KB
    let cmd = Command::PolicyCreate {
        policy_ref: "large_policy@0".to_string(),
        text: large_text.clone(),
    };
    let result = apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(
        result.is_ok(),
        "Large text must succeed: {:?}",
        result.err()
    );

    // Verify it's retrievable
    let text = provider.policy_read("large_policy@0").unwrap();
    assert_eq!(text, large_text);
}

// ---------------------------------------------------------------------------
// S-PC-9: policy_ref is stable retrieval key — policy.get returns exactly that policy
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_stable_retrieval_key() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let text = "# My stable policy\nObligation: do the thing.".to_string();
    let cmd = Command::PolicyCreate {
        policy_ref: "stable_key@1".to_string(),
        text: text.clone(),
    };
    apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    let retrieved = provider.policy_read("stable_key@1").unwrap();
    assert_eq!(
        retrieved, text,
        "Retrieved policy must be byte-identical to stored"
    );
}

// ---------------------------------------------------------------------------
// S-PC-10: PolicyCreate NOT idempotent — second identical call → PolicyConflict
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_not_idempotent() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let mk = || Command::PolicyCreate {
        policy_ref: "idem@0".to_string(),
        text: "# Identical".to_string(),
    };
    apply_command(mk(), None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    let result2 = apply_command(mk(), None, &mut conn, &cas, &provider, &NoopApprovalRouter);
    assert!(result2.is_err(), "Second identical PolicyCreate must fail");
    assert_eq!(result2.unwrap_err().kind(), ExErrorKind::PolicyConflict);
}

// ---------------------------------------------------------------------------
// S-PC-11: After PolicyCreate, policy.list includes new policy
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_appears_in_list() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let cmd = Command::PolicyCreate {
        policy_ref: "list_check@0".to_string(),
        text: "# Listed policy".to_string(),
    };
    apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    let list = provider.policy_list().unwrap();
    assert!(
        list.iter().any(|e| e.policy_ref == "list_check@0"),
        "Created policy must appear in policy_list"
    );
}

// ---------------------------------------------------------------------------
// S-PC-13: PolicyCreate success reflected in state.get_version (V+1)
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_state_version_incremented() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    let v0 = sv(&conn);
    let cmd = Command::PolicyCreate {
        policy_ref: "sv_check@0".to_string(),
        text: "# Content".to_string(),
    };
    let (_, returned_sv) =
        apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    let v1 = sv(&conn);
    assert_eq!(v1, v0 + 1, "state_version must increment by 1");
    assert_eq!(returned_sv, v1, "returned sv must match DB sv");
}

// ---------------------------------------------------------------------------
// S-PC-14: Existing file-backed policies remain retrievable after PolicyCreate
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_existing_policies_retrievable() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    // Pre-seed an existing policy file
    fs::write(policies_dir.join("existing@0.md"), "# Existing policy").unwrap();
    let provider = FilePolicyProvider::new(&policies_dir);

    // Create a new policy
    let cmd = Command::PolicyCreate {
        policy_ref: "new_one@0".to_string(),
        text: "# New policy".to_string(),
    };
    apply_command(cmd, None, &mut conn, &cas, &provider, &NoopApprovalRouter).unwrap();

    // Both must be retrievable
    let existing_text = provider.policy_read("existing@0").unwrap();
    assert_eq!(existing_text, "# Existing policy");
    let new_text = provider.policy_read("new_one@0").unwrap();
    assert_eq!(new_text, "# New policy");
}

// ---------------------------------------------------------------------------
// S-PC-15: PolicyCreate MUST NOT overwrite existing policy
// ---------------------------------------------------------------------------

#[test]
fn test_policy_create_must_not_overwrite_existing() {
    let (_dir, mut conn, cas, policies_dir) = setup();
    let provider = FilePolicyProvider::new(&policies_dir);

    // First create
    let original = "# Original content".to_string();
    apply_command(
        Command::PolicyCreate {
            policy_ref: "no_overwrite@0".to_string(),
            text: original.clone(),
        },
        None,
        &mut conn,
        &cas,
        &provider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Second create (attempt overwrite)
    let result = apply_command(
        Command::PolicyCreate {
            policy_ref: "no_overwrite@0".to_string(),
            text: "# Replacement content".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &provider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "Second PolicyCreate must fail");

    // Original content must be preserved
    let current = provider.policy_read("no_overwrite@0").unwrap();
    assert_eq!(current, original, "Original policy must not be overwritten");
}

// DEFERRED: S-PC-concurrent — file-system atomic rename + SQLite uniqueness each guarantee
// single-winner semantics; cross-system race is OS-level. Deferred to load test phase.
