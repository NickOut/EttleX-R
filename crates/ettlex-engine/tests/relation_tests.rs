//! Relation CRUD tests — Slice 02 (SC-S02-07 through SC-S02-42).
//!
//! Tests cover: timestamp format, relation type registry validation,
//! RelationCreate happy/error paths, cycle detection, RelationUpdate,
//! RelationGet, RelationList, RelationTombstone, and EttleTombstone extension.

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::command::{apply_command, Command};
use ettlex_store::cas::FsStore;
use ettlex_store::migrations::apply_migrations;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

//fn setup_db() -> (Connection, TempDir) {
//    let dir = TempDir::new().expect("temp dir");
//    let mut conn = Connection::open_in_memory().expect("in-memory db");
//    apply_migrations(&mut conn).expect("migrations should apply");
//    (conn, dir)
//}

fn setup_db_with_cas() -> (Connection, FsStore, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let cas = FsStore::new(dir.path().join("cas"));
    let mut conn = Connection::open_in_memory().expect("in-memory db");
    apply_migrations(&mut conn).expect("migrations should apply");
    (conn, cas, dir)
}

fn create_ettle(conn: &mut Connection, cas: &FsStore) -> String {
    let (res, _sv) = apply_command(
        Command::EttleCreate {
            title: format!("Test Ettle {}", uuid::Uuid::new_v4()),
            ettle_id: None,
            why: None,
            what: None,
            how: None,
            reasoning_link_id: None,
            reasoning_link_type: None,
        },
        None,
        conn,
        cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("ettle create should succeed");
    match res {
        ettlex_engine::commands::command::CommandResult::EttleCreate { ettle_id } => ettle_id,
        _ => panic!("unexpected result"),
    }
}

fn tombstone_ettle(conn: &mut Connection, cas: &FsStore, ettle_id: &str) {
    apply_command(
        Command::EttleTombstone {
            ettle_id: ettle_id.to_string(),
        },
        None,
        conn,
        cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("ettle tombstone should succeed");
}

fn create_relation(
    conn: &mut Connection,
    cas: &FsStore,
    src: &str,
    tgt: &str,
    rel_type: &str,
) -> String {
    let (res, _sv) = apply_command(
        Command::RelationCreate {
            source_ettle_id: src.to_string(),
            target_ettle_id: tgt.to_string(),
            relation_type: rel_type.to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        conn,
        cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("relation create should succeed");
    match res {
        ettlex_engine::commands::command::CommandResult::RelationCreate { relation_id } => {
            relation_id
        }
        _ => panic!("unexpected result"),
    }
}

// ---------------------------------------------------------------------------
// Group B: Timestamp ISO-8601 (SC-S02-07, SC-S02-08)
// ---------------------------------------------------------------------------

// SC-S02-07: occurred_at is ISO-8601 string after provenance mutation
#[test]
fn test_occurred_at_is_iso8601_after_mutation() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    // Create an ettle to trigger provenance event
    create_ettle(&mut conn, &cas);

    // Check provenance_events.occurred_at is ISO-8601
    let occurred_at: String = conn
        .query_row(
            "SELECT occurred_at FROM provenance_events ORDER BY rowid DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .expect("provenance event should exist");

    // ISO-8601 format: yyyy-mm-ddThh:mm:ss...Z or +00:00
    assert!(
        occurred_at.len() > 15,
        "occurred_at should be ISO-8601, got: {}",
        occurred_at
    );
    assert!(
        occurred_at.contains('T'),
        "occurred_at should contain 'T' separator, got: {}",
        occurred_at
    );
    // Should not be a short integer-like string
    assert!(
        !occurred_at.chars().all(|c| c.is_ascii_digit()),
        "occurred_at should not be a pure integer, got: {}",
        occurred_at
    );
}

// SC-S02-08: command_log applied_at is ISO-8601 after write
#[test]
fn test_command_log_applied_at_is_iso8601() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    // Create a relation to trigger command_log insert
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    // Check command_log.applied_at
    let applied_at: String = conn
        .query_row(
            "SELECT applied_at FROM command_log ORDER BY rowid DESC LIMIT 1",
            [],
            |r| r.get(0),
        )
        .expect("command_log row should exist");

    assert!(
        applied_at.len() > 15,
        "applied_at should be ISO-8601, got: {}",
        applied_at
    );
    assert!(
        applied_at.contains('T'),
        "applied_at should contain 'T', got: {}",
        applied_at
    );
}

// ---------------------------------------------------------------------------
// Group C: Relation Type Registry validation (SC-S02-09, SC-S02-10)
// ---------------------------------------------------------------------------

// SC-S02-09: RelationCreate with unknown type rejected
#[test]
fn test_relation_create_unknown_type_rejected() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "nonexistent_type".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with unknown type");

    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// SC-S02-10: RelationCreate with tombstoned registry entry rejected
#[test]
fn test_relation_create_tombstoned_type_rejected() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    // Tombstone the 'option' type in the registry
    conn.execute(
        "UPDATE relation_type_registry SET tombstoned_at = '2026-01-01T00:00:00Z' WHERE relation_type = 'option'",
        [],
    )
    .expect("update should work");

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "option".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with tombstoned type");

    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// Group D: RelationCreate happy paths (SC-S02-11 through SC-S02-16)
// ---------------------------------------------------------------------------

// SC-S02-11: RelationCreate with valid endpoints and type succeeds
#[test]
fn test_relation_create_valid_succeeds() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let result = apply_command(
        Command::RelationCreate {
            source_ettle_id: src.clone(),
            target_ettle_id: tgt.clone(),
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "RelationCreate should succeed: {:?}",
        result.err()
    );
}

// SC-S02-12: RelationCreate returns id with rel: prefix
#[test]
fn test_relation_create_returns_rel_prefix_id() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");
    assert!(
        rel_id.starts_with("rel:"),
        "relation_id should start with 'rel:', got: {}",
        rel_id
    );
}

// SC-S02-13: RelationCreate increments state_version by 1
#[test]
fn test_relation_create_increments_state_version() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let sv_before: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("count");

    create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let sv_after: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("count");

    assert_eq!(
        sv_after,
        sv_before + 1,
        "state_version should increment by 1"
    );
}

// SC-S02-14: RelationCreate with constraint type and properties_json succeeds
#[test]
fn test_relation_create_constraint_type_with_properties() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let result = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "constraint".to_string(),
            properties_json: Some(serde_json::json!({
                "family": "ABB",
                "kind": "MUST",
                "scope": "global"
            })),
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "RelationCreate with constraint type should succeed: {:?}",
        result.err()
    );
}

// SC-S02-15: Two RelationCreate with same endpoints produce distinct ids
#[test]
fn test_relation_create_two_same_endpoints_distinct_ids() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let id1 = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");
    let id2 = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    assert_ne!(
        id1, id2,
        "two relations on same endpoints should have distinct IDs"
    );
}

// SC-S02-16: RelationCreate provenance event carries relation context fields
#[test]
fn test_relation_create_provenance_event_carries_context() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    // Check provenance event was appended with kind = relation_created
    let (kind, correlation_id): (String, String) = conn
        .query_row(
            "SELECT kind, correlation_id FROM provenance_events WHERE kind = 'relation_created' ORDER BY rowid DESC LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .expect("relation_created provenance event should exist");

    assert_eq!(kind, "relation_created");
    assert_eq!(
        correlation_id, rel_id,
        "correlation_id should be the relation_id"
    );
}

// ---------------------------------------------------------------------------
// Group E: RelationCreate error paths (SC-S02-17 through SC-S02-22)
// ---------------------------------------------------------------------------

// SC-S02-17: RelationCreate rejects non-existent source
#[test]
fn test_relation_create_missing_source_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let tgt = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: "ettle:does-not-exist".to_string(),
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with missing source");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-18: RelationCreate rejects tombstoned source
#[test]
fn test_relation_create_tombstoned_source_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    tombstone_ettle(&mut conn, &cas, &src);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with tombstoned source");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-19: RelationCreate rejects non-existent target
#[test]
fn test_relation_create_missing_target_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: "ettle:does-not-exist".to_string(),
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with missing target");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-20: RelationCreate rejects tombstoned target
#[test]
fn test_relation_create_tombstoned_target_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    tombstone_ettle(&mut conn, &cas, &tgt);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with tombstoned target");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-21: RelationCreate rejects self-referential relation
#[test]
fn test_relation_create_self_referential_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src.clone(),
            target_ettle_id: src.clone(),
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with self-referential");

    assert_eq!(err.kind(), ExErrorKind::SelfReferentialLink);
}

// SC-S02-22: RelationCreate rejects caller-supplied relation_id
#[test]
fn test_relation_create_caller_supplied_id_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: Some("rel:my-supplied-id".to_string()),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with caller-supplied id");

    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// Group F: Cycle detection (SC-S02-23 through SC-S02-26)
// ---------------------------------------------------------------------------

// SC-S02-23: RelationCreate detects direct cycle for constraint type
#[test]
fn test_relation_create_direct_cycle_detected() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let a = create_ettle(&mut conn, &cas);
    let b = create_ettle(&mut conn, &cas);

    // A → B (constraint)
    create_relation(&mut conn, &cas, &a, &b, "constraint");

    // B → A (constraint) — should fail: cycle
    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: b.clone(),
            target_ettle_id: a.clone(),
            relation_type: "constraint".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with cycle detection");

    assert_eq!(err.kind(), ExErrorKind::CycleDetected);
}

// SC-S02-24: RelationCreate detects transitive cycle for constraint type
#[test]
fn test_relation_create_transitive_cycle_detected() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let a = create_ettle(&mut conn, &cas);
    let b = create_ettle(&mut conn, &cas);
    let c = create_ettle(&mut conn, &cas);

    // A → B → C (constraint)
    create_relation(&mut conn, &cas, &a, &b, "constraint");
    create_relation(&mut conn, &cas, &b, &c, "constraint");

    // C → A (constraint) — should fail: transitive cycle
    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: c,
            target_ettle_id: a,
            relation_type: "constraint".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with transitive cycle detection");

    assert_eq!(err.kind(), ExErrorKind::CycleDetected);
}

// SC-S02-25: RelationCreate does not check cycles for semantic_peer type
#[test]
fn test_relation_create_no_cycle_check_for_semantic_peer() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let a = create_ettle(&mut conn, &cas);
    let b = create_ettle(&mut conn, &cas);

    // A → B (semantic_peer)
    create_relation(&mut conn, &cas, &a, &b, "semantic_peer");

    // B → A (semantic_peer) — should succeed (no cycle check for semantic_peer)
    let result = apply_command(
        Command::RelationCreate {
            source_ettle_id: b,
            target_ettle_id: a,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "semantic_peer should not have cycle detection: {:?}",
        result.err()
    );
}

// SC-S02-26: CycleDetected leaves no partial state
#[test]
fn test_relation_create_cycle_detected_no_partial_state() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let a = create_ettle(&mut conn, &cas);
    let b = create_ettle(&mut conn, &cas);

    create_relation(&mut conn, &cas, &a, &b, "constraint");

    // Count relations before attempt
    let count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE tombstoned_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("count");

    // Attempt cycle-creating relation (should fail)
    let _ = apply_command(
        Command::RelationCreate {
            source_ettle_id: b,
            target_ettle_id: a,
            relation_type: "constraint".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    let count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM relations WHERE tombstoned_at IS NULL",
            [],
            |r| r.get(0),
        )
        .expect("count");

    assert_eq!(
        count_before, count_after,
        "failed CycleDetected should leave no partial state"
    );
}

// ---------------------------------------------------------------------------
// Group G: RelationUpdate (SC-S02-27 through SC-S02-30)
// ---------------------------------------------------------------------------

// SC-S02-27: RelationUpdate changes properties_json
#[test]
fn test_relation_update_properties_succeeds() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let result = apply_command(
        Command::RelationUpdate {
            relation_id: rel_id.clone(),
            properties_json: Some(serde_json::json!({"updated": true})),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "RelationUpdate should succeed: {:?}",
        result.err()
    );

    // Verify the update was persisted
    let props: String = conn
        .query_row(
            "SELECT properties_json FROM relations WHERE id = ?1",
            [&rel_id],
            |r| r.get(0),
        )
        .expect("relation should exist");
    let val: serde_json::Value = serde_json::from_str(&props).expect("valid JSON");
    assert_eq!(val.get("updated").and_then(|v| v.as_bool()), Some(true));
}

// SC-S02-28: RelationUpdate rejects non-existent relation
#[test]
fn test_relation_update_not_found_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::RelationUpdate {
            relation_id: "rel:does-not-exist".to_string(),
            properties_json: Some(serde_json::json!({})),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-29: RelationUpdate rejects tombstoned relation
#[test]
fn test_relation_update_tombstoned_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    // Tombstone it
    apply_command(
        Command::RelationTombstone {
            relation_id: rel_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("tombstone should succeed");

    let err = apply_command(
        Command::RelationUpdate {
            relation_id: rel_id,
            properties_json: Some(serde_json::json!({"x": 1})),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-30: RelationUpdate with no fields rejected
#[test]
fn test_relation_update_empty_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let err = apply_command(
        Command::RelationUpdate {
            relation_id: rel_id,
            properties_json: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with EmptyUpdate");

    assert_eq!(err.kind(), ExErrorKind::EmptyUpdate);
}

// ---------------------------------------------------------------------------
// Group H: RelationGet and RelationList (SC-S02-31 through SC-S02-38)
// ---------------------------------------------------------------------------

// SC-S02-31: RelationGet returns full record
#[test]
fn test_relation_get_returns_full_record() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let (result, _sv) = apply_command(
        Command::RelationGet {
            relation_id: rel_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("RelationGet should succeed");

    if let ettlex_engine::commands::command::CommandResult::RelationGet { record } = result {
        assert_eq!(record.id, rel_id);
        assert_eq!(record.source_ettle_id, src);
        assert_eq!(record.target_ettle_id, tgt);
        assert_eq!(record.relation_type, "semantic_peer");
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-32: RelationGet returns NotFound for unknown id
#[test]
fn test_relation_get_not_found() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::RelationGet {
            relation_id: "rel:does-not-exist".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-33: RelationList by source_ettle_id returns matching relations
#[test]
fn test_relation_list_by_source() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt1 = create_ettle(&mut conn, &cas);
    let tgt2 = create_ettle(&mut conn, &cas);
    let other = create_ettle(&mut conn, &cas);
    let another = create_ettle(&mut conn, &cas);

    create_relation(&mut conn, &cas, &src, &tgt1, "semantic_peer");
    create_relation(&mut conn, &cas, &src, &tgt2, "semantic_peer");
    // Different source — should not appear
    create_relation(&mut conn, &cas, &other, &another, "semantic_peer");

    let (result, _sv) = apply_command(
        Command::RelationList {
            source_ettle_id: Some(src),
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("RelationList should succeed");

    if let ettlex_engine::commands::command::CommandResult::RelationList { items } = result {
        assert_eq!(items.len(), 2, "should return 2 relations for that source");
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-34: RelationList by relation_type filters correctly
#[test]
fn test_relation_list_by_type() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let a = create_ettle(&mut conn, &cas);
    let b = create_ettle(&mut conn, &cas);
    let c = create_ettle(&mut conn, &cas);

    create_relation(&mut conn, &cas, &a, &b, "semantic_peer");
    create_relation(&mut conn, &cas, &a, &c, "refinement");

    let (result, _sv) = apply_command(
        Command::RelationList {
            source_ettle_id: Some(a),
            target_ettle_id: None,
            relation_type: Some("semantic_peer".to_string()),
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("RelationList should succeed");

    if let ettlex_engine::commands::command::CommandResult::RelationList { items } = result {
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].relation_type, "semantic_peer");
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-35: RelationList with no filter rejected
#[test]
fn test_relation_list_no_filter_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::RelationList {
            source_ettle_id: None,
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail with no filter");

    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// SC-S02-36: RelationList ordering is deterministic
#[test]
fn test_relation_list_ordering_is_deterministic() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt1 = create_ettle(&mut conn, &cas);
    let tgt2 = create_ettle(&mut conn, &cas);
    let tgt3 = create_ettle(&mut conn, &cas);

    create_relation(&mut conn, &cas, &src, &tgt1, "semantic_peer");
    create_relation(&mut conn, &cas, &src, &tgt2, "semantic_peer");
    create_relation(&mut conn, &cas, &src, &tgt3, "semantic_peer");

    let (result1, _) = apply_command(
        Command::RelationList {
            source_ettle_id: Some(src.clone()),
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (result2, _) = apply_command(
        Command::RelationList {
            source_ettle_id: Some(src),
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    if let (
        ettlex_engine::commands::command::CommandResult::RelationList { items: items1 },
        ettlex_engine::commands::command::CommandResult::RelationList { items: items2 },
    ) = (result1, result2)
    {
        let ids1: Vec<&str> = items1.iter().map(|r| r.id.as_str()).collect();
        let ids2: Vec<&str> = items2.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(ids1, ids2, "ordering should be deterministic");
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-37: RelationList excludes tombstoned by default
#[test]
fn test_relation_list_excludes_tombstoned_by_default() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt1 = create_ettle(&mut conn, &cas);
    let tgt2 = create_ettle(&mut conn, &cas);

    let rel1 = create_relation(&mut conn, &cas, &src, &tgt1, "semantic_peer");
    create_relation(&mut conn, &cas, &src, &tgt2, "semantic_peer");

    // Tombstone rel1
    apply_command(
        Command::RelationTombstone {
            relation_id: rel1.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("tombstone should succeed");

    let (result, _) = apply_command(
        Command::RelationList {
            source_ettle_id: Some(src),
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    if let ettlex_engine::commands::command::CommandResult::RelationList { items } = result {
        assert_eq!(items.len(), 1, "tombstoned relation should be excluded");
        assert_ne!(items[0].id, rel1);
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-38: RelationGet byte-identical across repeated calls
#[test]
fn test_relation_get_byte_identical() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let (r1, _) = apply_command(
        Command::RelationGet {
            relation_id: rel_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (r2, _) = apply_command(
        Command::RelationGet {
            relation_id: rel_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let j1 = serde_json::to_string(&r1).expect("serialize");
    let j2 = serde_json::to_string(&r2).expect("serialize");
    assert_eq!(j1, j2, "RelationGet should be byte-identical across calls");
}

// ---------------------------------------------------------------------------
// Group I: RelationTombstone (SC-S02-39 through SC-S02-42)
// ---------------------------------------------------------------------------

// SC-S02-39: RelationTombstone marks relation inactive; row retained
#[test]
fn test_relation_tombstone_marks_inactive() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    apply_command(
        Command::RelationTombstone {
            relation_id: rel_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("tombstone should succeed");

    // Row should still exist but with tombstoned_at set
    let tombstoned_at: Option<String> = conn
        .query_row(
            "SELECT tombstoned_at FROM relations WHERE id = ?1",
            [&rel_id],
            |r| r.get(0),
        )
        .expect("row should still exist");

    assert!(tombstoned_at.is_some(), "tombstoned_at should be set");
}

// SC-S02-40: RelationTombstone rejects non-existent relation
#[test]
fn test_relation_tombstone_not_found_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::RelationTombstone {
            relation_id: "rel:does-not-exist".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-41: RelationTombstone rejects already tombstoned relation
#[test]
fn test_relation_tombstone_already_tombstoned_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    apply_command(
        Command::RelationTombstone {
            relation_id: rel_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("first tombstone should succeed");

    let err = apply_command(
        Command::RelationTombstone {
            relation_id: rel_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("second tombstone should fail");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-42: EttleTombstone blocked by active outgoing constraint relation
#[test]
fn test_ettle_tombstone_blocked_by_active_constraint_relation() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    // Create constraint relation from src → tgt
    create_relation(&mut conn, &cas, &src, &tgt, "constraint");

    // Attempt to tombstone src — should fail
    let err = apply_command(
        Command::EttleTombstone {
            ettle_id: src.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: has active constraint relation");

    assert_eq!(err.kind(), ExErrorKind::HasActiveDependants);
}

// ---------------------------------------------------------------------------
// Group L: OCC for new commands (SC-S02-68, SC-S02-69)
// ---------------------------------------------------------------------------

// SC-S02-68: Correct expected_state_version succeeds for new write commands
#[test]
fn test_occ_correct_version_succeeds_for_relation_create() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let current_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("count");

    let result = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        Some(current_sv),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(result.is_ok(), "correct OCC version should succeed");
}

// SC-S02-69: Wrong expected_state_version fails for new write commands
#[test]
fn test_occ_wrong_version_fails_for_relation_create() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let current_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("count");

    let err = apply_command(
        Command::RelationCreate {
            source_ettle_id: src,
            target_ettle_id: tgt,
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        Some(current_sv + 999),
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("wrong OCC version should fail");

    assert_eq!(err.kind(), ExErrorKind::HeadMismatch);
}

// ---------------------------------------------------------------------------
// Group M: Provenance (SC-S02-70 through SC-S02-73)
// ---------------------------------------------------------------------------

// SC-S02-70: Each relation mutation appends exactly one provenance event
#[test]
fn test_relation_mutation_appends_provenance_event() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);

    let prov_count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind LIKE 'relation_%'",
            [],
            |r| r.get(0),
        )
        .expect("count");

    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let prov_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind LIKE 'relation_%'",
            [],
            |r| r.get(0),
        )
        .expect("count");

    assert_eq!(
        prov_count_after,
        prov_count_before + 1,
        "exactly one provenance event per relation mutation"
    );

    // Check it was relation_created
    let kind: String = conn
        .query_row(
            "SELECT kind FROM provenance_events WHERE correlation_id = ?1",
            [&rel_id],
            |r| r.get(0),
        )
        .expect("provenance event should exist");
    assert_eq!(kind, "relation_created");
}

// SC-S02-71: Each group mutation appends exactly one provenance event
#[test]
fn test_group_mutation_appends_provenance_event() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let prov_count_before: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind LIKE 'group_%'",
            [],
            |r| r.get(0),
        )
        .expect("count");

    let (result, _) = apply_command(
        Command::GroupCreate {
            name: "My Group".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("group create should succeed");

    let group_id = match result {
        ettlex_engine::commands::command::CommandResult::GroupCreate { group_id } => group_id,
        _ => panic!("unexpected result"),
    };

    let prov_count_after: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind LIKE 'group_%'",
            [],
            |r| r.get(0),
        )
        .expect("count");

    assert_eq!(
        prov_count_after,
        prov_count_before + 1,
        "exactly one provenance event"
    );

    let kind: String = conn
        .query_row(
            "SELECT kind FROM provenance_events WHERE correlation_id = ?1",
            [&group_id],
            |r| r.get(0),
        )
        .expect("provenance event should exist");
    assert_eq!(kind, "group_created");
}

// SC-S02-72: Failed command appends no provenance event (relations)
#[test]
fn test_failed_relation_command_no_provenance_event() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let prov_count_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM provenance_events", [], |r| r.get(0))
        .expect("count");

    // Attempt to create relation with non-existent source (should fail)
    let _ = apply_command(
        Command::RelationCreate {
            source_ettle_id: "ettle:nonexistent".to_string(),
            target_ettle_id: "ettle:also-nonexistent".to_string(),
            relation_type: "semantic_peer".to_string(),
            properties_json: None,
            relation_id: None,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );

    let prov_count_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM provenance_events", [], |r| r.get(0))
        .expect("count");

    assert_eq!(
        prov_count_before, prov_count_after,
        "failed command should not append provenance event"
    );
}

// SC-S02-73: Provenance occurred_at field is valid ISO-8601 after mutation
#[test]
fn test_provenance_occurred_at_is_iso8601() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let src = create_ettle(&mut conn, &cas);
    let tgt = create_ettle(&mut conn, &cas);
    let rel_id = create_relation(&mut conn, &cas, &src, &tgt, "semantic_peer");

    let occurred_at: String = conn
        .query_row(
            "SELECT occurred_at FROM provenance_events WHERE correlation_id = ?1",
            [&rel_id],
            |r| r.get(0),
        )
        .expect("provenance event should exist");

    // ISO-8601 check: length > 15, contains T
    assert!(
        occurred_at.len() > 15 && occurred_at.contains('T'),
        "occurred_at should be ISO-8601, got: {}",
        occurred_at
    );
}
