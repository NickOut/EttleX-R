//! Engine-layer scenario tests for `store_ep_update` Ettle.
//!
//! Tests that require SQLite / snapshot machinery.  Written from spec only.
//!
//! Scenario → test mapping:
//!   S-SU-6    test_ep_update_increments_state_version
//!   S-SU-8    test_ep_update_reflected_in_snapshot_diff
//!   S-SU-mig1 (in ettlex-store/tests/migrations_test.rs — test_migration_011_eps_title_column)
//!   S-SU-null  test_ep_update_null_fields_rejected
//!   S-SU-large test_ep_update_large_content_succeeds
//!   S-SU-inv   test_ep_update_invariants_preserved
//!   S-SU-idem  test_ep_update_not_idempotent
//!   S-SU-det   test_ep_update_deterministic
//!   S-SU-conc1 test_ep_update_concurrent_occ_one_wins
//!   S-SU-conc2 test_ep_update_sequential_no_occ_both_succeed
//!   S-SU-obs   test_ep_update_state_version_observable
//!   S-SU-mig2  test_ep_update_on_pre_title_ep_succeeds
//!   S-SU-proh  test_ep_update_must_not_create_ep
//!   S-SU-byte  test_ep_update_byte_identical_retrieval

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::command::{apply_command, Command};
use ettlex_engine::commands::engine_query::{
    apply_engine_query, EngineQuery, EngineQueryResult, SnapshotRef,
};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (TempDir, Connection, FsStore) {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("test.db");
    let cas_path = dir.path().join("cas");
    let mut conn = Connection::open(&db).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (dir, conn, FsStore::new(cas_path))
}

fn seed_leaf(conn: &Connection, ep_id: &str, ettle_id: &str, why: &str) {
    conn.execute_batch(&format!(
        "INSERT OR IGNORE INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('{ettle_id}', 'Test Ettle', NULL, 0, 0, 0, '{{}}');
         INSERT OR IGNORE INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                                    content_inline, deleted, created_at, updated_at)
         VALUES ('{ep_id}', '{ettle_id}', 0, 1, NULL,
                 '{{\"why\":\"{why}\",\"what\":\"w\",\"how\":\"h\"}}',
                 0, 0, 0);"
    ))
    .unwrap();
}

// ---------------------------------------------------------------------------
// S-SU-6: EpUpdate increments state_version by exactly 1
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_increments_state_version() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:su6:0";
    let ettle_id = "ettle:su6";
    seed_leaf(&conn, ep_id, ettle_id, "original why");

    let before_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated why".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let (_result, new_sv) = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    assert_eq!(
        new_sv,
        before_sv + 1,
        "state_version should increment by exactly 1"
    );
}

// ---------------------------------------------------------------------------
// S-SU-8: EpUpdate is reflected in snapshot diff (ep_digest changes)
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_reflected_in_snapshot_diff() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:su8:0";
    let ettle_id = "ettle:su8";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    // Commit snapshot S1
    let s1 = snapshot_commit(
        ettle_id,
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Apply EpUpdate
    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("amended".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Commit snapshot S2
    let s2 = snapshot_commit(
        ettle_id,
        "policy/default@0",
        "profile/default@0",
        SnapshotOptions {
            expected_head: None,
            dry_run: false,
            allow_dedup: false,
        },
        &mut conn,
        &cas,
    )
    .unwrap();

    // Diff S1 vs S2 — changed_eps must include the EP
    let query = EngineQuery::SnapshotDiff {
        a_ref: SnapshotRef::SnapshotId(s1.snapshot_id.clone()),
        b_ref: SnapshotRef::SnapshotId(s2.snapshot_id.clone()),
    };
    let result = apply_engine_query(query, &conn, &cas, None).unwrap();

    let diff = match result {
        EngineQueryResult::SnapshotDiff(d) => *d,
        other => panic!("Expected SnapshotDiff, got {:?}", other),
    };

    assert!(
        diff.structured_diff
            .ep_content_changes
            .changed_eps
            .contains(&ep_id.to_string()),
        "changed_eps should include {} after EpUpdate; got {:?}",
        ep_id,
        diff.structured_diff.ep_content_changes.changed_eps
    );

    // ep_digest must differ between S1 and S2
    let find_ep_digest = |snap_id: &str| -> String {
        let digest: String = conn
            .query_row(
                "SELECT manifest_digest FROM snapshots WHERE snapshot_id = ?1",
                [snap_id],
                |r| r.get(0),
            )
            .unwrap();
        let manifest_json = cas.read(&digest).unwrap();
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_json).unwrap();
        manifest["ept"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["ep_id"].as_str() == Some(ep_id))
            .and_then(|e| e["ep_digest"].as_str())
            .unwrap()
            .to_string()
    };

    let s1_digest = find_ep_digest(&s1.snapshot_id);
    let s2_digest = find_ep_digest(&s2.snapshot_id);
    assert_ne!(
        s1_digest, s2_digest,
        "ep_digest must differ between S1 and S2 after EpUpdate"
    );

    // Also verify SqliteRepo reflects the updated EP
    let ep = SqliteRepo::get_ep(&conn, ep_id).unwrap().unwrap();
    assert_eq!(ep.why, "amended");
}

// ---------------------------------------------------------------------------
// S-SU-null: EpUpdate with all fields null rejected as EmptyUpdate
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_null_fields_rejected() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:sunull:0";
    let ettle_id = "ettle:sunull";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: None,
        what: None,
        how: None,
        title: None,
    };
    let result = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_err(), "EpUpdate with all null fields must fail");
    assert_eq!(result.unwrap_err().kind(), ExErrorKind::EmptyUpdate);
}

// ---------------------------------------------------------------------------
// S-SU-large: EpUpdate with large content (50KB) succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_large_content_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:sularge:0";
    let ettle_id = "ettle:sularge";
    seed_leaf(&conn, ep_id, ettle_id, "short");

    let large_why = "x".repeat(50 * 1024); // 50 KB
    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some(large_why.clone()),
        what: None,
        how: None,
        title: None,
    };
    let result = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "EpUpdate with large content must succeed: {:?}",
        result.err()
    );
    let ep = SqliteRepo::get_ep(&conn, ep_id).unwrap().unwrap();
    assert_eq!(ep.why.len(), 50 * 1024);
}

// ---------------------------------------------------------------------------
// S-SU-inv: EpUpdate does not change ordinal/ettle_id/child_ettle_id
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_invariants_preserved() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suinv:0";
    let ettle_id = "ettle:suinv";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("new why".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let ep = SqliteRepo::get_ep(&conn, ep_id).unwrap().unwrap();
    assert_eq!(ep.ordinal, 0, "ordinal must not change");
    assert_eq!(ep.ettle_id, ettle_id, "ettle_id must not change");
    assert_eq!(ep.child_ettle_id, None, "child_ettle_id must not change");
}

// ---------------------------------------------------------------------------
// S-SU-idem: EpUpdate NOT idempotent — two updates each increment state_version (V+2)
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_not_idempotent() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suidem:0";
    let ettle_id = "ettle:suidem";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let before_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();

    let cmd1 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("update 1".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd1,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Same content as cmd1 — still a distinct command, must still increment sv
    let cmd2 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("update 1".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd2,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let after_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        after_sv,
        before_sv + 2,
        "Two EpUpdate calls must increment state_version by 2"
    );
}

// ---------------------------------------------------------------------------
// S-SU-det: EpUpdate result deterministic — same input → same stored state
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_deterministic() {
    let (_dir, mut conn, cas) = setup();
    seed_leaf(&conn, "ep:sudet1:0", "ettle:sudet1", "original");
    seed_leaf(&conn, "ep:sudet2:0", "ettle:sudet2", "original");

    let cmd1 = Command::EpUpdate {
        ep_id: "ep:sudet1:0".to_string(),
        why: Some("deterministic".to_string()),
        what: Some("same content".to_string()),
        how: None,
        title: None,
    };
    apply_command(
        cmd1,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let cmd2 = Command::EpUpdate {
        ep_id: "ep:sudet2:0".to_string(),
        why: Some("deterministic".to_string()),
        what: Some("same content".to_string()),
        how: None,
        title: None,
    };
    apply_command(
        cmd2,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let ep1 = SqliteRepo::get_ep(&conn, "ep:sudet1:0").unwrap().unwrap();
    let ep2 = SqliteRepo::get_ep(&conn, "ep:sudet2:0").unwrap().unwrap();
    assert_eq!(ep1.why, ep2.why, "why must be identical");
    assert_eq!(ep1.what, ep2.what, "what must be identical");
    assert_eq!(ep1.how, ep2.how, "how must be identical");
}

// ---------------------------------------------------------------------------
// S-SU-conc1: Concurrent EpUpdate with expected_state_version → one success, HeadMismatch
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_concurrent_occ_one_wins() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suconc1:0";
    let ettle_id = "ettle:suconc1";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    // Both callers observe state_version = 0
    let sv_before: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(sv_before, 0);

    // First update wins
    let cmd1 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("first".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let result1 = apply_command(
        cmd1,
        Some(0),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result1.is_ok(), "First concurrent update must succeed");

    // Second update with same expected_sv=0 loses
    let cmd2 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("second".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let result2 = apply_command(
        cmd2,
        Some(0),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result2.is_err(), "Second concurrent update must fail");
    assert_eq!(result2.unwrap_err().kind(), ExErrorKind::HeadMismatch);
}

// ---------------------------------------------------------------------------
// S-SU-conc2: Sequential EpUpdate without expected_state_version → both succeed (V+2)
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_sequential_no_occ_both_succeed() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suconc2:0";
    let ettle_id = "ettle:suconc2";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let before_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();

    let cmd1 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("first".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd1,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let cmd2 = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("second".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd2,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let after_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        after_sv,
        before_sv + 2,
        "Both sequential updates must succeed (V+2)"
    );
}

// ---------------------------------------------------------------------------
// S-SU-obs: EpUpdate success reflected in state.get_version()
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_state_version_observable() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suobs:0";
    let ettle_id = "ettle:suobs";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let v0: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let (_result, returned_sv) = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let v1: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(v1, v0 + 1, "state_version must increment after EpUpdate");
    assert_eq!(
        returned_sv, v1,
        "returned new_sv must match observable state_version"
    );
}

// ---------------------------------------------------------------------------
// S-SU-mig2: EpUpdate on EP created without title column succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_on_pre_title_ep_succeeds() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:sumig2:0";
    let ettle_id = "ettle:sumig2";
    // seed_leaf does NOT include title column in INSERT (simulates pre-migration EP)
    seed_leaf(&conn, ep_id, ettle_id, "original");

    // Verify title is NULL (pre-migration state)
    let title: Option<String> = conn
        .query_row("SELECT title FROM eps WHERE id = ?1", [ep_id], |r| r.get(0))
        .unwrap();
    assert!(
        title.is_none(),
        "Pre-title-column EP should have null title"
    );

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated why".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let result = apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "EpUpdate on pre-title EP must succeed: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// S-SU-proh: EpUpdate MUST NOT create new EP (list_eps count unchanged)
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_must_not_create_ep() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:suproh:0";
    let ettle_id = "ettle:suproh";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let count_before: u64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |r| r.get(0))
        .unwrap();

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    let count_after: u64 = conn
        .query_row("SELECT COUNT(*) FROM eps", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count_before, count_after,
        "EpUpdate must not create a new EP row"
    );
}

// ---------------------------------------------------------------------------
// S-SU-byte: ep.get after EpUpdate returns byte-identical results
// ---------------------------------------------------------------------------

#[test]
fn test_ep_update_byte_identical_retrieval() {
    let (_dir, mut conn, cas) = setup();
    let ep_id = "ep:subyte:0";
    let ettle_id = "ettle:subyte";
    seed_leaf(&conn, ep_id, ettle_id, "original");

    let cmd = Command::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("byte check".to_string()),
        what: Some("exact content".to_string()),
        how: Some("precise how".to_string()),
        title: Some("T1".to_string()),
    };
    apply_command(
        cmd,
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .unwrap();

    // Read twice — must be byte-identical
    let ep1 = SqliteRepo::get_ep(&conn, ep_id).unwrap().unwrap();
    let ep2 = SqliteRepo::get_ep(&conn, ep_id).unwrap().unwrap();
    assert_eq!(ep1.why, ep2.why);
    assert_eq!(ep1.what, ep2.what);
    assert_eq!(ep1.how, ep2.how);
    assert_eq!(ep1.title, ep2.title);
    // Verify exact values
    assert_eq!(ep1.why, "byte check");
    assert_eq!(ep1.what, "exact content");
    assert_eq!(ep1.how, "precise how");
    assert_eq!(ep1.title, Some("T1".to_string()));
}
