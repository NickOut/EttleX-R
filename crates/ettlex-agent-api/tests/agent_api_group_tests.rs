//! Agent API tests for Group and Group Member operations.
//!
//! SC-37  test_agent_group_create_succeeds
//! SC-38  test_agent_group_create_empty_name_fails
//! SC-39  test_agent_group_get_returns_full_record
//! SC-40  test_agent_group_get_not_found
//! SC-41  test_agent_group_list_active_deterministic
//! SC-42  test_agent_group_member_add_succeeds
//! SC-43  test_agent_group_member_add_duplicate_fails
//! SC-44  test_agent_group_member_add_tombstoned_group_fails
//! SC-45  test_agent_group_member_remove_marks_tombstoned
//! SC-46  test_agent_group_member_remove_not_found_fails
//! SC-47  test_agent_group_member_list_by_group_id
//! SC-48  test_agent_group_member_list_no_filter_fails

use ettlex_agent_api::{
    agent_ettle_create, agent_group_create, agent_group_get, agent_group_list,
    agent_group_member_add, agent_group_member_list, agent_group_member_remove, AgentEttleCreate,
    AgentGroupMemberListOpts,
};
use ettlex_memory::{
    migrations, Connection, ExErrorKind, FsStore, NoopApprovalRouter, NoopPolicyProvider,
};
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Test harness
// ---------------------------------------------------------------------------

struct Harness {
    _tmp: TempDir,
    pub conn: Connection,
    pub cas: FsStore,
}

impl Harness {
    fn new() -> Self {
        let tmp = TempDir::new().unwrap();
        let db = tmp.path().join("test.db");
        let cas_path = tmp.path().join("cas");
        let mut conn = Connection::open(&db).unwrap();
        migrations::apply_migrations(&mut conn).unwrap();
        let cas = FsStore::new(cas_path);
        Self {
            _tmp: tmp,
            conn,
            cas,
        }
    }

    fn create_ettle(&mut self, title: &str) -> String {
        agent_ettle_create(
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
            AgentEttleCreate {
                title: title.to_string(),
                ..Default::default()
            },
            None,
        )
        .unwrap()
        .ettle_id
    }

    fn create_group(&mut self, name: &str) -> String {
        agent_group_create(
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
            name,
            None,
        )
        .unwrap()
        .group_id
    }

    fn tombstone_group(&mut self, group_id: &str) {
        use ettlex_memory::apply_command;
        use ettlex_memory::Command;
        apply_command(
            Command::GroupTombstone {
                group_id: group_id.to_string(),
            },
            None,
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// SC-37 — agent_group_create succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_create_succeeds() {
    let mut h = Harness::new();

    let result = agent_group_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        "My Group",
        None,
    )
    .unwrap();

    assert!(result.group_id.starts_with("grp:"), "expected grp: prefix");

    let record = agent_group_get(&h.conn, &result.group_id).unwrap();
    assert_eq!(record.name, "My Group");
    assert!(record.tombstoned_at.is_none());

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'group_created'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-38 — agent_group_create empty name fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_create_empty_name_fails() {
    let mut h = Harness::new();
    let err = agent_group_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        "",
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidTitle);
}

// ---------------------------------------------------------------------------
// SC-39 — agent_group_get returns full record
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_get_returns_full_record() {
    let mut h = Harness::new();
    let group_id = h.create_group("Test Group");

    let record = agent_group_get(&h.conn, &group_id).unwrap();
    assert_eq!(record.id, group_id);
    assert_eq!(record.name, "Test Group");
    assert!(record.tombstoned_at.is_none());
    assert!(!record.created_at.is_empty());
}

// ---------------------------------------------------------------------------
// SC-40 — agent_group_get not found
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_get_not_found() {
    let h = Harness::new();
    let err = agent_group_get(&h.conn, "grp:missing").unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-41 — agent_group_list returns active in deterministic order
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_list_active_deterministic() {
    let mut h = Harness::new();

    let id_a = h.create_group("Group A");
    let id_b = h.create_group("Group B");

    // Tombstone group B
    h.tombstone_group(&id_b);

    let list1 = agent_group_list(&h.conn, false).unwrap();
    let list2 = agent_group_list(&h.conn, false).unwrap();

    // Only active group should appear
    assert_eq!(list1.len(), 1, "only 1 active group");
    assert_eq!(list1[0].id, id_a);

    // Ordering is deterministic
    let ids1: Vec<_> = list1.iter().map(|g| &g.id).collect();
    let ids2: Vec<_> = list2.iter().map(|g| &g.id).collect();
    assert_eq!(ids1, ids2);
}

// ---------------------------------------------------------------------------
// SC-42 — agent_group_member_add succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_add_succeeds() {
    let mut h = Harness::new();
    let group_id = h.create_group("My Group");
    let ettle_id = h.create_ettle("My Ettle");

    let result = agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap();

    assert!(result.new_state_version > 0);

    // Verify membership
    let members = agent_group_member_list(
        &h.conn,
        &AgentGroupMemberListOpts {
            group_id: Some(group_id.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(members.len(), 1);
    assert_eq!(members[0].ettle_id, ettle_id);
    assert!(members[0].tombstoned_at.is_none());

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'group_member_added'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-43 — agent_group_member_add duplicate fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_add_duplicate_fails() {
    let mut h = Harness::new();
    let group_id = h.create_group("Group");
    let ettle_id = h.create_ettle("Ettle");

    // First add
    agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap();

    // Duplicate add (active membership)
    let err = agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap_err();

    // Should be ConstraintViolation or DuplicateMapping
    assert!(
        matches!(
            err.kind(),
            ExErrorKind::ConstraintViolation | ExErrorKind::DuplicateMapping
        ),
        "expected ConstraintViolation or DuplicateMapping, got {:?}",
        err.kind()
    );
}

// ---------------------------------------------------------------------------
// SC-44 — agent_group_member_add tombstoned group fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_add_tombstoned_group_fails() {
    let mut h = Harness::new();
    let group_id = h.create_group("Group");
    let ettle_id = h.create_ettle("Ettle");

    h.tombstone_group(&group_id);

    let err = agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-45 — agent_group_member_remove tombstones membership
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_remove_marks_tombstoned() {
    let mut h = Harness::new();
    let group_id = h.create_group("Group");
    let ettle_id = h.create_ettle("Ettle");

    agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap();

    let result = agent_group_member_remove(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap();

    assert!(result.new_state_version > 0);

    // Verify tombstoned via include_tombstoned=true
    let members = agent_group_member_list(
        &h.conn,
        &AgentGroupMemberListOpts {
            group_id: Some(group_id.clone()),
            include_tombstoned: true,
            ..Default::default()
        },
    )
    .unwrap();

    assert_eq!(members.len(), 1);
    assert!(
        members[0].tombstoned_at.is_some(),
        "membership should be tombstoned"
    );

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'group_member_removed'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-46 — agent_group_member_remove not found fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_remove_not_found_fails() {
    let mut h = Harness::new();
    let group_id = h.create_group("Group");
    let ettle_id = h.create_ettle("Ettle");

    // Never added ettle to group — remove should fail
    let err = agent_group_member_remove(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &group_id,
        &ettle_id,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-47 — agent_group_member_list by group_id
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_list_by_group_id() {
    let mut h = Harness::new();
    let g = h.create_group("G");
    let h_group = h.create_group("H");
    let e1 = h.create_ettle("E1");
    let e2 = h.create_ettle("E2");

    // Add E1 and E2 to G
    agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &g,
        &e1,
        None,
    )
    .unwrap();
    agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &g,
        &e2,
        None,
    )
    .unwrap();
    // Add E1 to H
    agent_group_member_add(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &h_group,
        &e1,
        None,
    )
    .unwrap();

    let members = agent_group_member_list(
        &h.conn,
        &AgentGroupMemberListOpts {
            group_id: Some(g.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    let ettle_ids: Vec<_> = members.iter().map(|m| m.ettle_id.as_str()).collect();
    assert!(ettle_ids.contains(&e1.as_str()), "E1 should be in G");
    assert!(ettle_ids.contains(&e2.as_str()), "E2 should be in G");
    assert_eq!(members.len(), 2, "only G's members, not H's");
}

// ---------------------------------------------------------------------------
// SC-48 — agent_group_member_list no filter fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_group_member_list_no_filter_fails() {
    let h = Harness::new();
    let err = agent_group_member_list(
        &h.conn,
        &AgentGroupMemberListOpts {
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}
