//! Agent API tests for Relation operations.
//!
//! SC-23  test_agent_relation_create_succeeds
//! SC-24  test_agent_relation_create_rejects_caller_id
//! SC-25  test_agent_relation_create_unknown_type
//! SC-26  test_agent_relation_create_self_referential
//! SC-27  test_agent_relation_create_missing_source
//! SC-28  test_agent_relation_create_tombstoned_source
//! SC-29  test_agent_relation_get_returns_full_record
//! SC-30  test_agent_relation_get_not_found
//! SC-31  test_agent_relation_list_by_source
//! SC-32  test_agent_relation_list_no_filter_fails
//! SC-33  test_agent_relation_list_ordering_deterministic
//! SC-34  test_agent_relation_tombstone_marks_inactive
//! SC-35  test_agent_relation_tombstone_not_found
//! SC-36  test_agent_relation_tombstone_already_tombstoned

use ettlex_agent_api::agent_ettle_create;
use ettlex_agent_api::{
    agent_ettle_tombstone, agent_relation_create, agent_relation_get, agent_relation_list,
    agent_relation_tombstone, AgentEttleCreate, AgentRelationCreate, AgentRelationListOpts,
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

    fn create_relation(&mut self, source: &str, target: &str) -> String {
        agent_relation_create(
            &mut self.conn,
            &self.cas,
            &NoopPolicyProvider,
            &NoopApprovalRouter,
            AgentRelationCreate {
                source_ettle_id: source.to_string(),
                target_ettle_id: target.to_string(),
                relation_type: "refinement".to_string(),
                ..Default::default()
            },
            None,
        )
        .unwrap()
        .relation_id
    }
}

// ---------------------------------------------------------------------------
// SC-23 — agent_relation_create succeeds
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_succeeds() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");

    let result = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: a.clone(),
            target_ettle_id: b.clone(),
            relation_type: "refinement".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap();

    assert!(
        result.relation_id.starts_with("rel:"),
        "expected rel: prefix"
    );

    let record = agent_relation_get(&h.conn, &result.relation_id).unwrap();
    assert_eq!(record.source_ettle_id, a);
    assert_eq!(record.target_ettle_id, b);
    assert_eq!(record.relation_type, "refinement");
    assert!(record.tombstoned_at.is_none());

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'relation_created'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-24 — agent_relation_create rejects caller-supplied id
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_rejects_caller_id() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");

    let err = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: a,
            target_ettle_id: b,
            relation_type: "refinement".to_string(),
            relation_id: Some("rel:manual".to_string()),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-25 — agent_relation_create unknown relation type
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_unknown_type() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");

    let err = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: a,
            target_ettle_id: b,
            relation_type: "does-not-exist".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-26 — agent_relation_create rejects self-referential
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_self_referential() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");

    let err = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: a.clone(),
            target_ettle_id: a.clone(),
            relation_type: "refinement".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::SelfReferentialLink);
}

// ---------------------------------------------------------------------------
// SC-27 — agent_relation_create rejects missing source
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_missing_source() {
    let mut h = Harness::new();
    let b = h.create_ettle("B");

    let err = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: "ettle:missing".to_string(),
            target_ettle_id: b,
            relation_type: "refinement".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-28 — agent_relation_create rejects tombstoned source
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_create_tombstoned_source() {
    let mut h = Harness::new();
    let p = h.create_ettle("P");
    let q = h.create_ettle("Q");

    agent_ettle_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &p,
        None,
    )
    .unwrap();

    let err = agent_relation_create(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        AgentRelationCreate {
            source_ettle_id: p,
            target_ettle_id: q,
            relation_type: "refinement".to_string(),
            ..Default::default()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}

// ---------------------------------------------------------------------------
// SC-29 — agent_relation_get returns full record
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_get_returns_full_record() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");
    let rel_id = h.create_relation(&a, &b);

    let record = agent_relation_get(&h.conn, &rel_id).unwrap();
    assert_eq!(record.id, rel_id);
    assert_eq!(record.source_ettle_id, a);
    assert_eq!(record.target_ettle_id, b);
    assert_eq!(record.relation_type, "refinement");
    assert!(record.tombstoned_at.is_none());
    assert!(!record.created_at.is_empty());
}

// ---------------------------------------------------------------------------
// SC-30 — agent_relation_get not found
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_get_not_found() {
    let h = Harness::new();
    let err = agent_relation_get(&h.conn, "rel:missing").unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-31 — agent_relation_list by source
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_list_by_source() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");
    let c = h.create_ettle("C");

    let rel_ab = h.create_relation(&a, &b);
    let rel_ac = h.create_relation(&a, &c);
    let _rel_bc = h.create_relation(&b, &c);

    let results = agent_relation_list(
        &h.conn,
        &AgentRelationListOpts {
            source_ettle_id: Some(a.clone()),
            ..Default::default()
        },
    )
    .unwrap();

    let ids: Vec<_> = results.iter().map(|r| r.id.as_str()).collect();
    assert!(ids.contains(&rel_ab.as_str()), "A→B should appear");
    assert!(ids.contains(&rel_ac.as_str()), "A→C should appear");
    assert_eq!(results.len(), 2, "only A's outgoing relations");
}

// ---------------------------------------------------------------------------
// SC-32 — agent_relation_list no filter fails
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_list_no_filter_fails() {
    let h = Harness::new();
    let err = agent_relation_list(
        &h.conn,
        &AgentRelationListOpts {
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::InvalidInput);
}

// ---------------------------------------------------------------------------
// SC-33 — agent_relation_list ordering deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_list_ordering_deterministic() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");
    let c = h.create_ettle("C");

    h.create_relation(&a, &b);
    h.create_relation(&a, &c);

    let r1 = agent_relation_list(
        &h.conn,
        &AgentRelationListOpts {
            source_ettle_id: Some(a.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    let r2 = agent_relation_list(
        &h.conn,
        &AgentRelationListOpts {
            source_ettle_id: Some(a),
            ..Default::default()
        },
    )
    .unwrap();

    let ids1: Vec<_> = r1.iter().map(|r| &r.id).collect();
    let ids2: Vec<_> = r2.iter().map(|r| &r.id).collect();
    assert_eq!(ids1, ids2, "ordering should be deterministic");
}

// ---------------------------------------------------------------------------
// SC-34 — agent_relation_tombstone marks inactive
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_tombstone_marks_inactive() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");
    let rel_id = h.create_relation(&a, &b);

    let result = agent_relation_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &rel_id,
        None,
    )
    .unwrap();

    assert!(result.new_state_version > 0);

    let record = agent_relation_get(&h.conn, &rel_id).unwrap();
    assert!(
        record.tombstoned_at.is_some(),
        "tombstoned_at should be set"
    );

    // Provenance event
    let count: i64 = h
        .conn
        .query_row(
            "SELECT COUNT(*) FROM provenance_events WHERE kind = 'relation_tombstoned'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(count >= 1);
}

// ---------------------------------------------------------------------------
// SC-35 — agent_relation_tombstone not found
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_tombstone_not_found() {
    let mut h = Harness::new();
    let err = agent_relation_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        "rel:missing",
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::NotFound);
}

// ---------------------------------------------------------------------------
// SC-36 — agent_relation_tombstone already tombstoned
// ---------------------------------------------------------------------------

#[test]
fn test_agent_relation_tombstone_already_tombstoned() {
    let mut h = Harness::new();
    let a = h.create_ettle("A");
    let b = h.create_ettle("B");
    let rel_id = h.create_relation(&a, &b);

    // First tombstone
    agent_relation_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &rel_id,
        None,
    )
    .unwrap();

    // Second tombstone should fail
    let err = agent_relation_tombstone(
        &mut h.conn,
        &h.cas,
        &NoopPolicyProvider,
        &NoopApprovalRouter,
        &rel_id,
        None,
    )
    .unwrap_err();
    assert_eq!(err.kind(), ExErrorKind::AlreadyTombstoned);
}
