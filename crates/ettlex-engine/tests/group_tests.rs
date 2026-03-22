//! Group and GroupMember CRUD tests — Slice 02 (SC-S02-43 through SC-S02-67).

use ettlex_core::approval_router::NoopApprovalRouter;
use ettlex_core::errors::ExErrorKind;
use ettlex_core::policy_provider::NoopPolicyProvider;
use ettlex_engine::commands::command::{apply_command, Command, CommandResult};
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
        CommandResult::EttleCreate { ettle_id } => ettle_id,
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

fn create_group(conn: &mut Connection, cas: &FsStore, name: &str) -> String {
    let (res, _sv) = apply_command(
        Command::GroupCreate {
            name: name.to_string(),
        },
        None,
        conn,
        cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("group create should succeed");
    match res {
        CommandResult::GroupCreate { group_id } => group_id,
        _ => panic!("unexpected result"),
    }
}

// ---------------------------------------------------------------------------
// Group J: Groups (SC-S02-43 through SC-S02-54)
// ---------------------------------------------------------------------------

// SC-S02-43: GroupCreate succeeds with valid name
#[test]
fn test_group_create_succeeds() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let result = apply_command(
        Command::GroupCreate {
            name: "My Test Group".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "GroupCreate should succeed: {:?}",
        result.err()
    );
}

// SC-S02-44: GroupCreate returns id with grp: prefix
#[test]
fn test_group_create_returns_grp_prefix_id() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    assert!(
        group_id.starts_with("grp:"),
        "group_id should start with 'grp:', got: {}",
        group_id
    );
}

// SC-S02-45: GroupCreate rejects empty name
#[test]
fn test_group_create_empty_name_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let err = apply_command(
        Command::GroupCreate {
            name: "".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::InvalidTitle);
}

// SC-S02-46: GroupCreate rejects whitespace-only name
#[test]
fn test_group_create_whitespace_name_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let err = apply_command(
        Command::GroupCreate {
            name: "   ".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail");

    assert_eq!(err.kind(), ExErrorKind::InvalidTitle);
}

// SC-S02-47: GroupGet returns full record
#[test]
fn test_group_get_returns_full_record() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");

    let (result, _sv) = apply_command(
        Command::GroupGet {
            group_id: group_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("GroupGet should succeed");

    if let CommandResult::GroupGet { record } = result {
        assert_eq!(record.id, group_id);
        assert_eq!(record.name, "My Group");
        assert!(record.tombstoned_at.is_none());
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-48: GroupGet returns NotFound for unknown id
#[test]
fn test_group_get_not_found() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::GroupGet {
            group_id: "grp:does-not-exist".to_string(),
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

// SC-S02-49: GroupList returns active groups in deterministic order
#[test]
fn test_group_list_ordering_is_deterministic() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let _g1 = create_group(&mut conn, &cas, "Alpha");
    let _g2 = create_group(&mut conn, &cas, "Beta");
    let _g3 = create_group(&mut conn, &cas, "Gamma");

    let (r1, _) = apply_command(
        Command::GroupList {
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (r2, _) = apply_command(
        Command::GroupList {
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
        CommandResult::GroupList { items: items1 },
        CommandResult::GroupList { items: items2 },
    ) = (r1, r2)
    {
        let ids1: Vec<&str> = items1.iter().map(|g| g.id.as_str()).collect();
        let ids2: Vec<&str> = items2.iter().map(|g| g.id.as_str()).collect();
        assert_eq!(ids1, ids2, "GroupList ordering should be deterministic");
        assert_eq!(items1.len(), 3);
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-50: GroupList is byte-identical across repeated calls
#[test]
fn test_group_list_byte_identical() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    create_group(&mut conn, &cas, "Alpha");
    create_group(&mut conn, &cas, "Beta");

    let (r1, _) = apply_command(
        Command::GroupList {
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (r2, _) = apply_command(
        Command::GroupList {
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap(),
        "GroupList should be byte-identical"
    );
}

// SC-S02-51: GroupTombstone blocked by active members
#[test]
fn test_group_tombstone_blocked_by_active_members() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    // Add member
    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("add member should succeed");

    let err = apply_command(
        Command::GroupTombstone {
            group_id: group_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: has active members");

    assert_eq!(err.kind(), ExErrorKind::HasActiveDependants);
}

// SC-S02-52: GroupTombstone succeeds with no active members
#[test]
fn test_group_tombstone_succeeds_no_members() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "Empty Group");

    let result = apply_command(
        Command::GroupTombstone {
            group_id: group_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "GroupTombstone with no members should succeed: {:?}",
        result.err()
    );

    // Verify tombstoned_at is set
    let ts: Option<String> = conn
        .query_row(
            "SELECT tombstoned_at FROM groups WHERE id = ?1",
            [&group_id],
            |r| r.get(0),
        )
        .expect("group should exist");
    assert!(ts.is_some(), "tombstoned_at should be set");
}

// SC-S02-53: GroupTombstone rejects non-existent group
#[test]
fn test_group_tombstone_not_found_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();

    let err = apply_command(
        Command::GroupTombstone {
            group_id: "grp:does-not-exist".to_string(),
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

// SC-S02-54: GroupTombstone rejects already tombstoned group
#[test]
fn test_group_tombstone_already_tombstoned_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");

    apply_command(
        Command::GroupTombstone {
            group_id: group_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("first tombstone should succeed");

    let err = apply_command(
        Command::GroupTombstone { group_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("second tombstone should fail");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// Group K: Group Membership (SC-S02-55 through SC-S02-67)
// ---------------------------------------------------------------------------

// SC-S02-55: GroupMemberAdd links ettle to group
#[test]
fn test_group_member_add_succeeds() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    let result = apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "GroupMemberAdd should succeed: {:?}",
        result.err()
    );

    // Verify member exists
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM group_members WHERE group_id = ?1 AND ettle_id = ?2 AND tombstoned_at IS NULL",
            rusqlite::params![group_id, ettle_id],
            |r| r.get(0),
        )
        .expect("query should work");
    assert_eq!(count, 1, "member should exist");
}

// SC-S02-56: GroupMemberAdd rejects duplicate active membership
#[test]
fn test_group_member_add_duplicate_active_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("first add should succeed");

    let err = apply_command(
        Command::GroupMemberAdd { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("duplicate add should fail");

    assert_eq!(err.kind(), ExErrorKind::DuplicateMapping);
}

// SC-S02-57: GroupMemberAdd succeeds after prior tombstoned membership
#[test]
fn test_group_member_add_after_tombstoned_succeeds() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    // Add then remove
    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("first add should succeed");

    apply_command(
        Command::GroupMemberRemove {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("remove should succeed");

    // Add again — should succeed even though prior tombstoned membership exists
    let result = apply_command(
        Command::GroupMemberAdd { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    );
    assert!(
        result.is_ok(),
        "re-add after tombstone should succeed: {:?}",
        result.err()
    );
}

// SC-S02-58: GroupMemberAdd rejects tombstoned group
#[test]
fn test_group_member_add_tombstoned_group_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    apply_command(
        Command::GroupTombstone {
            group_id: group_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("tombstone should succeed");

    let err = apply_command(
        Command::GroupMemberAdd { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: tombstoned group");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-59: GroupMemberAdd rejects tombstoned ettle
#[test]
fn test_group_member_add_tombstoned_ettle_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);
    tombstone_ettle(&mut conn, &cas, &ettle_id);

    let err = apply_command(
        Command::GroupMemberAdd { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: tombstoned ettle");

    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// SC-S02-60: GroupMemberAdd rejects non-existent group
#[test]
fn test_group_member_add_missing_group_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let ettle_id = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::GroupMemberAdd {
            group_id: "grp:does-not-exist".to_string(),
            ettle_id,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: missing group");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-61: GroupMemberAdd rejects non-existent ettle
#[test]
fn test_group_member_add_missing_ettle_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");

    let err = apply_command(
        Command::GroupMemberAdd {
            group_id,
            ettle_id: "ettle:does-not-exist".to_string(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: missing ettle");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-62: GroupMemberRemove tombstones membership record
#[test]
fn test_group_member_remove_tombstones_membership() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("add should succeed");

    apply_command(
        Command::GroupMemberRemove {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("remove should succeed");

    // Row should still exist but tombstoned
    let tombstoned: Option<String> = conn
        .query_row(
            "SELECT tombstoned_at FROM group_members WHERE group_id = ?1 AND ettle_id = ?2",
            rusqlite::params![group_id, ettle_id],
            |r| r.get(0),
        )
        .expect("row should exist");
    assert!(tombstoned.is_some(), "tombstoned_at should be set");
}

// SC-S02-63: GroupMemberRemove rejects non-existent membership
#[test]
fn test_group_member_remove_not_found_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    let err = apply_command(
        Command::GroupMemberRemove { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("should fail: not a member");

    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-64: GroupMemberRemove rejects already tombstoned membership
#[test]
fn test_group_member_remove_already_tombstoned_fails() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let ettle_id = create_ettle(&mut conn, &cas);

    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("add should succeed");

    apply_command(
        Command::GroupMemberRemove {
            group_id: group_id.clone(),
            ettle_id: ettle_id.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("first remove should succeed");

    let err = apply_command(
        Command::GroupMemberRemove { group_id, ettle_id },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect_err("second remove should fail");

    // Second remove fails because no active membership found (NotFound)
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// SC-S02-65: GroupMemberList returns active members in deterministic order
#[test]
fn test_group_member_list_ordering_is_deterministic() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let e1 = create_ettle(&mut conn, &cas);
    let e2 = create_ettle(&mut conn, &cas);
    let e3 = create_ettle(&mut conn, &cas);

    for e in &[&e1, &e2, &e3] {
        apply_command(
            Command::GroupMemberAdd {
                group_id: group_id.clone(),
                ettle_id: e.to_string(),
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("add should succeed");
    }

    let (r1, _) = apply_command(
        Command::GroupMemberList {
            group_id: group_id.clone(),
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (r2, _) = apply_command(
        Command::GroupMemberList {
            group_id,
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
        CommandResult::GroupMemberList { items: items1 },
        CommandResult::GroupMemberList { items: items2 },
    ) = (r1, r2)
    {
        assert_eq!(items1.len(), 3);
        let ids1: Vec<&str> = items1.iter().map(|m| m.id.as_str()).collect();
        let ids2: Vec<&str> = items2.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids1, ids2, "ordering should be deterministic");
    } else {
        panic!("unexpected result");
    }
}

// SC-S02-66: GroupMemberList is byte-identical across repeated calls
#[test]
fn test_group_member_list_byte_identical() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let e1 = create_ettle(&mut conn, &cas);
    let e2 = create_ettle(&mut conn, &cas);

    for e in &[&e1, &e2] {
        apply_command(
            Command::GroupMemberAdd {
                group_id: group_id.clone(),
                ettle_id: e.to_string(),
            },
            None,
            &mut conn,
            &cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .expect("add should succeed");
    }

    let (r1, _) = apply_command(
        Command::GroupMemberList {
            group_id: group_id.clone(),
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    let (r2, _) = apply_command(
        Command::GroupMemberList {
            group_id,
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    assert_eq!(
        serde_json::to_string(&r1).unwrap(),
        serde_json::to_string(&r2).unwrap(),
        "GroupMemberList should be byte-identical"
    );
}

// SC-S02-67: GroupMemberList with include_tombstoned returns all records
#[test]
fn test_group_member_list_include_tombstoned() {
    let (mut conn, cas, _dir) = setup_db_with_cas();
    let group_id = create_group(&mut conn, &cas, "My Group");
    let e1 = create_ettle(&mut conn, &cas);
    let e2 = create_ettle(&mut conn, &cas);

    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: e1.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("add e1");

    apply_command(
        Command::GroupMemberAdd {
            group_id: group_id.clone(),
            ettle_id: e2.clone(),
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("add e2");

    // Remove e1 (tombstone it)
    apply_command(
        Command::GroupMemberRemove {
            group_id: group_id.clone(),
            ettle_id: e1,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("remove should succeed");

    // Without include_tombstoned: only e2
    let (active, _) = apply_command(
        Command::GroupMemberList {
            group_id: group_id.clone(),
            include_tombstoned: false,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    if let CommandResult::GroupMemberList { items } = active {
        assert_eq!(items.len(), 1, "active only: should have 1");
    } else {
        panic!("unexpected");
    }

    // With include_tombstoned: both
    let (all, _) = apply_command(
        Command::GroupMemberList {
            group_id,
            include_tombstoned: true,
        },
        None,
        &mut conn,
        &cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
    )
    .expect("should succeed");

    if let CommandResult::GroupMemberList { items } = all {
        assert_eq!(items.len(), 2, "include_tombstoned: should have 2");
    } else {
        panic!("unexpected");
    }
}
