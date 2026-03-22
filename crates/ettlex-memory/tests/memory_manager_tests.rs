//! MemoryManager tests — Slice 02 (SC-S02-74 through SC-S02-76).

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_memory::memory_manager::MemoryManager;
use ettlex_memory::{Command, CommandResult};
use ettlex_store::cas::FsStore;
use ettlex_store::migrations::apply_migrations;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn setup_db_with_cas() -> (Connection, FsStore, TempDir) {
    let dir = TempDir::new().expect("temp dir");
    let cas = FsStore::new(dir.path().join("cas"));
    let mut conn = Connection::open_in_memory().expect("in-memory db");
    apply_migrations(&mut conn).expect("migrations should apply");
    (conn, cas, dir)
}

fn create_ettle(conn: &mut Connection, cas: &FsStore, title: &str) -> String {
    let mm = MemoryManager::new();
    let (res, _sv) = mm
        .apply_command(
            Command::EttleCreate {
                title: title.to_string(),
                ettle_id: None,
                why: Some("why field".to_string()),
                what: Some("what field".to_string()),
                how: Some("how field".to_string()),
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
        CommandResult::EttleCreate { ettle_id } => ettle_id,
        _ => panic!("unexpected result"),
    }
}

// ---------------------------------------------------------------------------
// SC-S02-74: MemoryManager.apply_command delegates to engine apply_command
// ---------------------------------------------------------------------------

#[test]
fn test_memory_manager_apply_command_delegates() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let mm = MemoryManager::new();

    // State version before
    let sv_before: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("query should work");

    // Apply a command through MemoryManager
    let (result, new_sv) = mm
        .apply_command(
            Command::EttleCreate {
                title: "Delegated Ettle".to_string(),
                ettle_id: None,
                why: None,
                what: None,
                how: None,
                reasoning_link_id: None,
                reasoning_link_type: None,
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("apply_command should succeed");

    // Verify the result is the expected variant
    match result {
        CommandResult::EttleCreate { ettle_id } => {
            assert!(ettle_id.contains(":"), "ettle_id should have ID format");
        }
        _ => panic!("expected EttleCreate result"),
    }

    // Verify state version was incremented (delegation proof)
    assert_eq!(new_sv, sv_before + 1, "state_version must increment");

    // Verify command_log row was inserted (further delegation proof)
    let sv_after: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .expect("query should work");
    assert_eq!(
        sv_after,
        sv_before + 1,
        "command_log must have one more row"
    );

    // Verify OCC is forwarded correctly: supply wrong expected_state_version
    let err = mm
        .apply_command(
            Command::EttleCreate {
                title: "OCC Test".to_string(),
                ettle_id: None,
                why: None,
                what: None,
                how: None,
                reasoning_link_id: None,
                reasoning_link_type: None,
            },
            Some(999), // wrong version
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect_err("wrong OCC version should fail");
    assert_eq!(err.kind(), ExErrorKind::HeadMismatch);
}

// ---------------------------------------------------------------------------
// SC-S02-75: MemoryManager.assemble_ettle_context returns correct fields
// ---------------------------------------------------------------------------

#[test]
fn test_memory_manager_assemble_context_fields() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let mm = MemoryManager::new();

    // Create ettle with all fields set
    let (res, _) = mm
        .apply_command(
            Command::EttleCreate {
                title: "Context Fields Ettle".to_string(),
                ettle_id: None,
                why: Some("The why reason".to_string()),
                what: Some("The what content".to_string()),
                how: Some("The how approach".to_string()),
                reasoning_link_id: None,
                reasoning_link_type: None,
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("create should succeed");

    let ettle_id = match res {
        CommandResult::EttleCreate { ettle_id } => ettle_id,
        _ => panic!("expected EttleCreate"),
    };

    // Assemble context
    let ctx = mm
        .assemble_ettle_context(&ettle_id, &conn)
        .expect("assemble should succeed");

    assert_eq!(ctx.ettle_id, ettle_id);
    assert_eq!(ctx.why, Some("The why reason".to_string()));
    assert_eq!(ctx.what, Some("The what content".to_string()));
    assert_eq!(ctx.how, Some("The how approach".to_string()));
    // No relations or group memberships yet
    assert!(ctx.relations.is_empty(), "no relations should be present");
    assert!(
        ctx.groups.is_empty(),
        "no group memberships should be present"
    );

    // Assemble context for a non-existent ettle should fail
    let err = mm
        .assemble_ettle_context("ettle:nonexistent", &conn)
        .expect_err("should fail for unknown ettle");
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-S02-76: MemoryManager.assemble_ettle_context includes relations and groups
// ---------------------------------------------------------------------------

#[test]
fn test_memory_manager_assemble_context_groups() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let mm = MemoryManager::new();

    // Create two ettles
    let ettle_a = create_ettle(&mut conn, &cas, "Ettle A");
    let ettle_b = create_ettle(&mut conn, &cas, "Ettle B");

    // Create a relation: A refinement B
    let (rel_res, _) = mm
        .apply_command(
            Command::RelationCreate {
                source_ettle_id: ettle_a.clone(),
                target_ettle_id: ettle_b.clone(),
                relation_type: "refinement".to_string(),
                properties_json: None,
                relation_id: None,
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("relation create should succeed");

    let _relation_id = match rel_res {
        CommandResult::RelationCreate { relation_id } => relation_id,
        _ => panic!("expected RelationCreate"),
    };

    // Create a group and add ettle_a to it
    let (grp_res, _) = mm
        .apply_command(
            Command::GroupCreate {
                name: "Test Group".to_string(),
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("group create should succeed");

    let group_id = match grp_res {
        CommandResult::GroupCreate { group_id } => group_id,
        _ => panic!("expected GroupCreate"),
    };

    mm.apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_a.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("group member add should succeed");

    // Assemble context for ettle_a
    let ctx = mm
        .assemble_ettle_context(&ettle_a, &conn)
        .expect("assemble should succeed");

    // ettle_a should have one outgoing relation
    assert_eq!(ctx.relations.len(), 1, "ettle_a should have one relation");
    let rel = &ctx.relations[0];
    assert_eq!(rel.source_ettle_id, ettle_a);
    assert_eq!(rel.target_ettle_id, ettle_b);
    assert_eq!(rel.relation_type, "refinement");

    // ettle_a should be in one group
    assert_eq!(ctx.groups.len(), 1, "ettle_a should be in one group");
    let grp = &ctx.groups[0];
    assert_eq!(grp.id, group_id);
    assert_eq!(grp.name, "Test Group");

    // ettle_b should have no outgoing relations (it is a target, not source)
    let ctx_b = mm
        .assemble_ettle_context(&ettle_b, &conn)
        .expect("assemble ettle_b should succeed");
    assert!(
        ctx_b.relations.is_empty(),
        "ettle_b should have no outgoing relations"
    );
    assert!(ctx_b.groups.is_empty(), "ettle_b is not in any group");
}
