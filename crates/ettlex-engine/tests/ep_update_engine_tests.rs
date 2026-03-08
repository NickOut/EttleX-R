//! Engine-layer scenario tests for `store_ep_update` Ettle.
//!
//! Tests that require SQLite / snapshot machinery.  Written from spec only.
//!
//! Scenario → test mapping:
//!   S-SU-6  test_ep_update_increments_state_version
//!   S-SU-8  test_ep_update_reflected_in_snapshot_diff

#![allow(clippy::unwrap_used)]

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::engine_query::{
    apply_engine_query, EngineQuery, EngineQueryResult, SnapshotRef,
};
use ettlex_engine::commands::mcp_command::{apply_mcp_command, McpCommand};
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
        .query_row("SELECT COUNT(*) FROM mcp_command_log", [], |r| r.get(0))
        .unwrap();

    let cmd = McpCommand::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("updated why".to_string()),
        what: None,
        how: None,
        title: None,
    };
    let (_result, new_sv) = apply_mcp_command(
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
    let cmd = McpCommand::EpUpdate {
        ep_id: ep_id.to_string(),
        why: Some("amended".to_string()),
        what: None,
        how: None,
        title: None,
    };
    apply_mcp_command(
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
