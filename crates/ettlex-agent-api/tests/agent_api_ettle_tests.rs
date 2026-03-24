//! Agent API tests for Ettle operations.
//!
//! SC-01  test_agent_ettle_get_returns_full_record
//! SC-02  test_agent_ettle_get_returns_tombstoned
//! SC-03  test_agent_ettle_get_not_found
//! SC-04  test_agent_ettle_get_byte_identical
//! SC-05  test_agent_ettle_get_lifecycle_events
//! SC-06  test_agent_ettle_context_assembled
//! SC-07  test_agent_ettle_context_not_found
//! SC-08  test_agent_ettle_list_active_deterministic
//! SC-09  test_agent_ettle_list_pagination
//! SC-10  test_agent_ettle_list_limit_zero_rejected
//! SC-11  test_agent_ettle_create_title_only
//! SC-12  test_agent_ettle_create_empty_title
//! SC-13  test_agent_ettle_create_rejects_caller_id
//! SC-14  test_agent_ettle_create_link_without_type
//! SC-15  test_agent_ettle_update_fields
//! SC-16  test_agent_ettle_update_clears_reasoning_link
//! SC-17  test_agent_ettle_update_preserves_unspecified
//! SC-18  test_agent_ettle_update_rejects_tombstoned
//! SC-19  test_agent_ettle_tombstone_marks_inactive
//! SC-20  test_agent_ettle_tombstone_rejects_active_dependants
//! SC-21  test_agent_occ_correct_version
//! SC-22  test_agent_occ_wrong_version

use ettlex_agent_api::{
    agent_ettle_context, agent_ettle_create, agent_ettle_get, agent_ettle_list,
    agent_ettle_tombstone, agent_ettle_update, AgentEttleCreate, AgentEttleListOpts,
    AgentEttleUpdate,
};
use ettlex_memory::{
    init_test_capture, migrations, Connection, ExErrorKind, FsStore, NoopApprovalRouter,
    NoopPolicyProvider,
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
        let result = agent_ettle_create(
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
        .unwrap();
        result.ettle_id
    }

    fn tombstone_ettle(&mut self, ettle_id: &str) {
        agent_ettle_tombstone(
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
            ettle_id,
            None,
        )
        .unwrap();
    }
}

// ---------------------------------------------------------------------------
// SC-01 — EttleGet returns full record
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_get_returns_full_record() {
    let mut h = Harness::new();

    // Create an ettle with all fields
    let create_result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Full Record Title".to_string(),
            why: Some("because why".to_string()),
            what: Some("what it is".to_string()),
            how: Some("how to do it".to_string()),
            reasoning_link_id: None,
            reasoning_link_type: None,
            ettle_id: None,
        },
        None,
    )
    .unwrap();

    let record = agent_ettle_get(&h.conn, &create_result.ettle_id).unwrap();
    assert_eq!(record.id, create_result.ettle_id);
    assert_eq!(record.title, "Full Record Title");
    assert_eq!(record.why, "because why");
    assert_eq!(record.what, "what it is");
    assert_eq!(record.how, "how to do it");
    assert!(record.tombstoned_at.is_none());
    assert!(!record.created_at.is_empty());
}

// ---------------------------------------------------------------------------
// SC-02 — EttleGet returns tombstoned record
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_get_returns_tombstoned() {
    let mut h = Harness::new();
    let id = h.create_ettle("Tombstone Me");
    h.tombstone_ettle(&id);

    let record = agent_ettle_get(&h.conn, &id).unwrap();
    assert!(record.tombstoned_at.is_some());
    // tombstoned_at should be ISO-8601
    let ts = record.tombstoned_at.unwrap();
    assert!(ts.contains('T'), "expected ISO-8601 timestamp, got: {ts}");
}

// ---------------------------------------------------------------------------
// SC-03 — EttleGet NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_get_not_found() {
    let h = Harness::new();
    let err = agent_ettle_get(&h.conn, "ettle:does-not-exist").unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-04 — EttleGet byte-identical
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_get_byte_identical() {
    let mut h = Harness::new();
    let id = h.create_ettle("Byte Identical");

    let r1 = agent_ettle_get(&h.conn, &id).unwrap();
    let r2 = agent_ettle_get(&h.conn, &id).unwrap();

    assert_eq!(r1.id, r2.id);
    assert_eq!(r1.title, r2.title);
    assert_eq!(r1.why, r2.why);
    assert_eq!(r1.what, r2.what);
    assert_eq!(r1.how, r2.how);
    assert_eq!(r1.created_at, r2.created_at);
    assert_eq!(r1.updated_at, r2.updated_at);
    assert_eq!(r1.tombstoned_at, r2.tombstoned_at);
}

// ---------------------------------------------------------------------------
// SC-05 — EttleGet lifecycle events owned by boundary
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_get_lifecycle_events() {
    let handle = init_test_capture();
    let mut h = Harness::new();
    let id = h.create_ettle("Lifecycle Test");

    // Clear any events from the create
    let _ = handle.events();

    let _ = agent_ettle_get(&h.conn, &id);

    let events = handle.events();
    let get_events: Vec<_> = events
        .iter()
        .filter(|e| {
            e.fields
                .get("op")
                .map(|v| v == "agent_ettle_get")
                .unwrap_or(false)
        })
        .collect();

    // Should have exactly one start and one end event
    let starts: Vec<_> = get_events
        .iter()
        .filter(|e| e.fields.get("event").map(|v| v == "start").unwrap_or(false))
        .collect();
    let ends: Vec<_> = get_events
        .iter()
        .filter(|e| {
            e.fields
                .get("event")
                .map(|v| v == "end" || v == "end_error")
                .unwrap_or(false)
        })
        .collect();

    assert_eq!(
        starts.len(),
        1,
        "expected exactly 1 start event for agent_ettle_get"
    );
    assert_eq!(
        ends.len(),
        1,
        "expected exactly 1 end event for agent_ettle_get"
    );
}

// ---------------------------------------------------------------------------
// SC-06 — agent_ettle_context returns assembled context
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_context_assembled() {
    let mut h = Harness::new();

    // Create an ettle with fields
    let create_result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Context Test".to_string(),
            why: Some("why content".to_string()),
            what: Some("what content".to_string()),
            how: Some("how content".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let ctx = agent_ettle_context(&h.conn, &create_result.ettle_id).unwrap();
    assert_eq!(ctx.ettle_id, create_result.ettle_id);
    assert_eq!(ctx.why, Some("why content".to_string()));
    assert_eq!(ctx.what, Some("what content".to_string()));
    assert_eq!(ctx.how, Some("how content".to_string()));
    // No relations or groups created yet
    assert!(ctx.relations.is_empty());
    assert!(ctx.groups.is_empty());
}

// ---------------------------------------------------------------------------
// SC-07 — agent_ettle_context NotFound
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_context_not_found() {
    let h = Harness::new();
    let err = agent_ettle_context(&h.conn, "ettle:missing").unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-08 — EttleList active Ettles in deterministic order
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_list_active_deterministic() {
    let mut h = Harness::new();

    let id_a = h.create_ettle("Ettle A");
    let id_b = h.create_ettle("Ettle B");

    // Tombstone id_b
    h.tombstone_ettle(&id_b);

    let page = agent_ettle_list(
        &h.conn,
        &AgentEttleListOpts {
            limit: 100,
            cursor: None,
            include_tombstoned: false,
        },
    )
    .unwrap();

    // Only active ettle should appear
    assert_eq!(page.items.len(), 1);
    assert_eq!(page.items[0].id, id_a);
    assert!(page.next_cursor.is_none());
}

// ---------------------------------------------------------------------------
// SC-09 — EttleList pagination deterministic and complete
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_list_pagination() {
    let mut h = Harness::new();

    // Create 10 ettles
    for i in 0..10 {
        h.create_ettle(&format!("Ettle {i:02}"));
    }

    // First page: 5 items
    let page1 = agent_ettle_list(
        &h.conn,
        &AgentEttleListOpts {
            limit: 5,
            cursor: None,
            include_tombstoned: false,
        },
    )
    .unwrap();

    assert_eq!(page1.items.len(), 5, "expected 5 items in first page");
    assert!(
        page1.next_cursor.is_some(),
        "expected next_cursor in first page"
    );

    // Second page: remaining 5 items
    let page2 = agent_ettle_list(
        &h.conn,
        &AgentEttleListOpts {
            limit: 5,
            cursor: page1.next_cursor.clone(),
            include_tombstoned: false,
        },
    )
    .unwrap();

    assert_eq!(page2.items.len(), 5, "expected 5 items in second page");
    assert!(
        page2.next_cursor.is_none(),
        "expected no next_cursor on last page"
    );

    // No overlap between pages
    let ids1: std::collections::HashSet<_> = page1.items.iter().map(|i| &i.id).collect();
    let ids2: std::collections::HashSet<_> = page2.items.iter().map(|i| &i.id).collect();
    assert!(ids1.is_disjoint(&ids2), "pages should not overlap");
}

// ---------------------------------------------------------------------------
// SC-10 — EttleList limit=0 rejected
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_list_limit_zero_rejected() {
    let h = Harness::new();
    let err = agent_ettle_list(
        &h.conn,
        &AgentEttleListOpts {
            limit: 0,
            cursor: None,
            include_tombstoned: false,
        },
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-11 — EttleCreate title only succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_create_title_only() {
    let mut h = Harness::new();

    let result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "My Ettle".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    assert!(
        result.ettle_id.starts_with("ettle:"),
        "expected ettle: prefix"
    );

    // Verify via get
    let record = agent_ettle_get(&h.conn, &result.ettle_id).unwrap();
    assert_eq!(record.title, "My Ettle");
    assert_eq!(record.why, "");
    assert_eq!(record.what, "");
    assert_eq!(record.how, "");

    // Verify provenance event recorded
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'ettle_created'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(
        count >= 1,
        "expected at least 1 ettle_created provenance event"
    );
}

// ---------------------------------------------------------------------------
// SC-12 — EttleCreate rejects empty title
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_create_empty_title() {
    let mut h = Harness::new();
    let err = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidTitle);

    // No provenance event
    let count: i64 = h
        .conn
        .query_row("SELECT COUNT(*) FROM provenance_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

// ---------------------------------------------------------------------------
// SC-13 — EttleCreate rejects caller-supplied ettle_id
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_create_rejects_caller_id() {
    let mut h = Harness::new();
    let err = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Valid Title".to_string(),
            ettle_id: Some("ettle:caller-supplied".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-14 — EttleCreate rejects reasoning_link_id without type
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_create_link_without_type() {
    let mut h = Harness::new();
    // First create a parent ettle so the link_id is valid (if type was specified)
    let parent_id = h.create_ettle("Parent");

    let err = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Child".to_string(),
            reasoning_link_id: Some(parent_id),
            reasoning_link_type: None,
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::MissingLinkType);
}

// ---------------------------------------------------------------------------
// SC-15 — EttleUpdate changes WHY/WHAT/HOW
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_update_fields() {
    let mut h = Harness::new();
    let id = h.create_ettle("Update Test");

    let update_result = agent_ettle_update(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleUpdate {
            ettle_id: id.clone(),
            why: Some("W2".to_string()),
            what: Some("X2".to_string()),
            how: Some("H2".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    assert!(update_result.new_state_version > 0);

    let record = agent_ettle_get(&h.conn, &id).unwrap();
    assert_eq!(record.why, "W2");
    assert_eq!(record.what, "X2");
    assert_eq!(record.how, "H2");

    // Verify provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'ettle_updated'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-16 — EttleUpdate clears reasoning_link via double-Option
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_update_clears_reasoning_link() {
    let mut h = Harness::new();

    // Create parent
    let parent_id = h.create_ettle("Parent");

    // Create child with reasoning_link
    let child_result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Child".to_string(),
            reasoning_link_id: Some(parent_id.clone()),
            reasoning_link_type: Some("refinement".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    // Verify link was set
    let record = agent_ettle_get(&h.conn, &child_result.ettle_id).unwrap();
    assert_eq!(record.reasoning_link_id, Some(parent_id.clone()));

    // Clear the link via double-Option: Some(None)
    agent_ettle_update(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleUpdate {
            ettle_id: child_result.ettle_id.clone(),
            reasoning_link_id: Some(None),
            reasoning_link_type: Some(None),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let updated = agent_ettle_get(&h.conn, &child_result.ettle_id).unwrap();
    assert!(
        updated.reasoning_link_id.is_none(),
        "reasoning_link_id should be null after clear"
    );
}

// ---------------------------------------------------------------------------
// SC-17 — EttleUpdate preserves unspecified fields
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_update_preserves_unspecified() {
    let mut h = Harness::new();

    let create_result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Preserve Test".to_string(),
            why: Some("W".to_string()),
            what: Some("X".to_string()),
            how: Some("H".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    // Only update title
    agent_ettle_update(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleUpdate {
            ettle_id: create_result.ettle_id.clone(),
            title: Some("T2".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    let record = agent_ettle_get(&h.conn, &create_result.ettle_id).unwrap();
    assert_eq!(record.title, "T2");
    assert_eq!(record.why, "W", "why should be preserved");
    assert_eq!(record.what, "X", "what should be preserved");
    assert_eq!(record.how, "H", "how should be preserved");
}

// ---------------------------------------------------------------------------
// SC-18 — EttleUpdate rejects tombstoned Ettle
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_update_rejects_tombstoned() {
    let mut h = Harness::new();
    let id = h.create_ettle("Tombstoned Update");
    h.tombstone_ettle(&id);

    let err = agent_ettle_update(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleUpdate {
            ettle_id: id.clone(),
            title: Some("New Title".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-19 — EttleTombstone marks Ettle inactive
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_tombstone_marks_inactive() {
    let mut h = Harness::new();
    let id = h.create_ettle("Tombstone Test");

    let result = agent_ettle_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &id,
        None,
    )
    .unwrap();

    assert!(result.new_state_version > 0);

    let record = agent_ettle_get(&h.conn, &id).unwrap();
    assert!(
        record.tombstoned_at.is_some(),
        "tombstoned_at should be set"
    );

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'ettle_tombstoned'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-20 — EttleTombstone rejects Ettle with active dependants
// ---------------------------------------------------------------------------

#[test]
fn test_agent_ettle_tombstone_rejects_active_dependants() {
    let mut h = Harness::new();

    let parent_id = h.create_ettle("Parent");

    // Create child with reasoning link to parent
    agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "Child".to_string(),
            reasoning_link_id: Some(parent_id.clone()),
            reasoning_link_type: Some("refinement".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    // Attempt to tombstone parent
    let err = agent_ettle_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &parent_id,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::HasActiveDependants);
}

// ---------------------------------------------------------------------------
// SC-21 — OCC correct version succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_agent_occ_correct_version() {
    let mut h = Harness::new();

    // Read current state version
    let sv: u64 = h
        .conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();

    // Create with correct expected_state_version
    let result = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "OCC Test".to_string(),
            ..Default::default()
        },
        Some(sv),
    )
    .unwrap();

    assert_eq!(result.new_state_version, sv + 1);
}

// ---------------------------------------------------------------------------
// SC-22 — OCC wrong version fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_occ_wrong_version() {
    let mut h = Harness::new();

    // First create to bump state version
    h.create_ettle("First");

    // Read current state version (should be 1)
    let sv: u64 = h
        .conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert!(sv > 0);

    // Call with wrong (stale) version
    let err = agent_ettle_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentEttleCreate {
            title: "OCC Fail".to_string(),
            ..Default::default()
        },
        Some(sv - 1),
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::HeadMismatch);

    // Verify no provenance event was appended for the failed command
    let count_before: i64 = h
        .conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count_before, sv as i64, "command_log should not have grown");
}
