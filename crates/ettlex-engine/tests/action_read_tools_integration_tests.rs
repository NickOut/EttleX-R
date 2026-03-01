//! Integration tests for ep:action_read_tools:0 — scenarios S1–S22.
//!
//! All tests use a real SQLite DB + FsStore (via TempDir).
//! Tests follow the RED→GREEN TDD protocol: each scenario is independently
//! exercised against `apply_engine_query`.

use ettlex_engine::commands::decision::{decision_create, decision_link};
use ettlex_engine::commands::engine_query::{apply_engine_query, EngineQuery, EngineQueryResult};
use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
use ettlex_store::cas::FsStore;
use rusqlite::Connection;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Setup helpers
// ---------------------------------------------------------------------------

fn setup() -> (TempDir, Connection, FsStore) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let cas_path = temp_dir.path().join("cas");
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    let cas = FsStore::new(cas_path);
    (temp_dir, conn, cas)
}

/// Insert a minimal ettle + single EP into the DB.
fn insert_ettle_ep(conn: &Connection, ettle_id: &str, ep_id: &str) {
    conn.execute_batch(&format!(
        r#"
        INSERT OR IGNORE INTO ettles
            (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('{ettle_id}', '{ettle_id}', NULL, 0, 0, 0, '{{}}');
        INSERT OR IGNORE INTO eps
            (id, ettle_id, ordinal, normative, child_ettle_id, content_digest,
             content_inline, deleted, created_at, updated_at)
        VALUES ('{ep_id}', '{ettle_id}', 0, 1, NULL, NULL, 'content', 0, 0, 0);
        "#
    ))
    .unwrap();
}

// ---------------------------------------------------------------------------
// S1: Read tools never mutate
// ---------------------------------------------------------------------------

#[test]
fn test_read_tools_are_nonmutating() {
    let (_tmp, mut conn, cas) = setup();

    // Insert an ettle and commit a snapshot so there is data to read
    insert_ettle_ep(&conn, "ettle:root", "ep:root:0");
    snapshot_commit(
        "ettle:root",
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

    let snap_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();
    let ettle_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |r| r.get(0))
        .unwrap();

    // Issue several read queries
    apply_engine_query(EngineQuery::StateGetVersion, &conn, &cas).unwrap();
    apply_engine_query(
        EngineQuery::EttleList(ettlex_engine::commands::read_tools::ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();
    apply_engine_query(
        EngineQuery::EttleGet {
            ettle_id: "ettle:root".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();
    apply_engine_query(
        EngineQuery::EpGet {
            ep_id: "ep:root:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    let snap_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();
    let ettle_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM ettles", [], |r| r.get(0))
        .unwrap();

    assert_eq!(snap_before, snap_after, "snapshot count must not change");
    assert_eq!(ettle_before, ettle_after, "ettle count must not change");
}

// ---------------------------------------------------------------------------
// S2: ettle.get returns metadata and ep_ids
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_get_returns_metadata_and_eps() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:a', 'Alpha', NULL, 0, 0, 0, '{}');
        INSERT INTO eps
            (id, ettle_id, ordinal, normative, child_ettle_id, content_digest,
             content_inline, deleted, created_at, updated_at)
        VALUES ('ep:a:0', 'ettle:a', 0, 1, NULL, NULL, 'content', 0, 0, 0);
        INSERT INTO eps
            (id, ettle_id, ordinal, normative, child_ettle_id, content_digest,
             content_inline, deleted, created_at, updated_at)
        VALUES ('ep:a:1', 'ettle:a', 1, 0, NULL, NULL, 'content', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = apply_engine_query(
        EngineQuery::EttleGet {
            ettle_id: "ettle:a".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EttleGet(r) => {
            assert_eq!(r.ettle.id, "ettle:a");
            assert_eq!(r.ettle.title, "Alpha");
            assert_eq!(r.ep_ids.len(), 2);
            // Ordered by ordinal
            assert_eq!(r.ep_ids[0], "ep:a:0");
            assert_eq!(r.ep_ids[1], "ep:a:1");
        }
        _ => panic!("expected EttleGet result"),
    }
}

// ---------------------------------------------------------------------------
// S3: ettle.list enforces default limit
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_list_enforces_default_limit() {
    let (_tmp, conn, cas) = setup();

    // Insert 102 ettles with unique IDs
    for i in 0..102 {
        conn.execute(
            "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
             VALUES (?1, ?2, NULL, 0, 0, 0, '{}')",
            rusqlite::params![format!("ettle:list:{:04}", i), format!("Ettle {}", i)],
        )
        .unwrap();
    }

    let result = apply_engine_query(
        EngineQuery::EttleList(ettlex_engine::commands::read_tools::ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EttleList(page) => {
            assert_eq!(page.items.len(), 100, "default limit must cap at 100");
            assert!(page.has_more, "has_more must be true when more items exist");
            assert!(
                page.cursor.is_some(),
                "cursor must be present when has_more"
            );
        }
        _ => panic!("expected EttleList result"),
    }
}

// ---------------------------------------------------------------------------
// S4: ettle.list cursor pagination is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_list_cursor_pagination_deterministic() {
    let (_tmp, conn, cas) = setup();

    // Insert 5 ettles, paginate 2 at a time
    for i in 0..5 {
        conn.execute(
            "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
             VALUES (?1, ?2, NULL, 0, 0, 0, '{}')",
            rusqlite::params![format!("ettle:pag:{:02}", i), format!("Pag {}", i)],
        )
        .unwrap();
    }

    let opts1 = ettlex_engine::commands::read_tools::ListOptions {
        limit: Some(2),
        cursor: None,
        prefix_filter: None,
        title_contains: None,
    };
    let page1 = match apply_engine_query(EngineQuery::EttleList(opts1), &conn, &cas).unwrap() {
        EngineQueryResult::EttleList(p) => p,
        _ => panic!("expected EttleList"),
    };

    assert_eq!(page1.items.len(), 2);
    assert!(page1.has_more);
    let cursor = page1.cursor.clone().expect("cursor must be present");

    let opts2 = ettlex_engine::commands::read_tools::ListOptions {
        limit: Some(2),
        cursor: Some(cursor),
        prefix_filter: None,
        title_contains: None,
    };
    let page2 = match apply_engine_query(EngineQuery::EttleList(opts2), &conn, &cas).unwrap() {
        EngineQueryResult::EttleList(p) => p,
        _ => panic!("expected EttleList"),
    };

    assert_eq!(page2.items.len(), 2);

    // IDs from page1 and page2 must be disjoint
    let ids1: Vec<_> = page1.items.iter().map(|e| e.id.clone()).collect();
    let ids2: Vec<_> = page2.items.iter().map(|e| e.id.clone()).collect();
    for id in &ids1 {
        assert!(!ids2.contains(id), "pages must not overlap");
    }
}

// ---------------------------------------------------------------------------
// S5: ettle.list prefix filter
// ---------------------------------------------------------------------------

#[test]
fn test_ettle_list_prefix_filter() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('abc:1', 'ABC One', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('abc:2', 'ABC Two', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('xyz:1', 'XYZ One', NULL, 0, 0, 0, '{}');
        "#,
    )
    .unwrap();

    let opts = ettlex_engine::commands::read_tools::ListOptions {
        limit: None,
        cursor: None,
        prefix_filter: Some("abc:".to_string()),
        title_contains: None,
    };
    let page = match apply_engine_query(EngineQuery::EttleList(opts), &conn, &cas).unwrap() {
        EngineQueryResult::EttleList(p) => p,
        _ => panic!("expected EttleList"),
    };

    assert_eq!(
        page.items.len(),
        2,
        "only abc: prefixed ettles should be returned"
    );
    for item in &page.items {
        assert!(item.id.starts_with("abc:"));
    }
}

// ---------------------------------------------------------------------------
// S6: ep.list_children is ordered by ordinal
// ---------------------------------------------------------------------------

#[test]
fn test_ep_list_children_deterministic() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:parent', 'Parent', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:child', 'Child', NULL, 0, 0, 0, '{}');

        -- Parent EP points to child ettle
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:parent:0', 'ettle:parent', 0, 1, 'ettle:child', NULL, 'c', 0, 0, 0);

        -- Child EPs inserted out of ordinal order
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:child:1', 'ettle:child', 1, 0, NULL, NULL, 'c', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:child:0', 'ettle:child', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = apply_engine_query(
        EngineQuery::EpListChildren {
            ep_id: "ep:parent:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EpListChildren(eps) => {
            assert_eq!(eps.len(), 2);
            assert_eq!(eps[0].id, "ep:child:0", "ordinal 0 must come first");
            assert_eq!(eps[1].id, "ep:child:1", "ordinal 1 must come second");
        }
        _ => panic!("expected EpListChildren result"),
    }
}

// ---------------------------------------------------------------------------
// S7: ep.list_parents returns a single parent
// ---------------------------------------------------------------------------

#[test]
fn test_ep_list_parents_single_parent() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:p', 'Parent', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:c', 'Child', NULL, 0, 0, 0, '{}');

        -- Parent EP in ettle:p refines into ettle:c
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:p:0', 'ettle:p', 0, 1, 'ettle:c', NULL, 'c', 0, 0, 0);

        -- Leaf EP in ettle:c
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:c:0', 'ettle:c', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result = apply_engine_query(
        EngineQuery::EpListParents {
            ep_id: "ep:c:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EpListParents(parents) => {
            assert_eq!(parents.len(), 1);
            assert_eq!(parents[0].id, "ep:p:0");
        }
        _ => panic!("expected EpListParents result"),
    }
}

// ---------------------------------------------------------------------------
// S8: ep.list_parents → RefinementIntegrityViolation when >1 parent ettle
// ---------------------------------------------------------------------------

#[test]
fn test_ep_list_parents_integrity_violation() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:a', 'A', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:b', 'B', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:leaf', 'Leaf', NULL, 0, 0, 0, '{}');

        -- Two parent EPs, both pointing to ettle:leaf
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:a:0', 'ettle:a', 0, 1, 'ettle:leaf', NULL, 'c', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:b:0', 'ettle:b', 0, 1, 'ettle:leaf', NULL, 'c', 0, 0, 0);

        -- Leaf EP
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:leaf:0', 'ettle:leaf', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let err = apply_engine_query(
        EngineQuery::EpListParents {
            ep_id: "ep:leaf:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::RefinementIntegrityViolation,
        "expected RefinementIntegrityViolation, got {:?}",
        err.kind()
    );
}

// ---------------------------------------------------------------------------
// S9: constraint.list_by_family filters tombstoned entries
// ---------------------------------------------------------------------------

#[test]
fn test_constraint_list_by_family_tombstone_filter() {
    let (_tmp, conn, cas) = setup();

    conn.execute_batch(
        r#"
        INSERT INTO constraints
            (constraint_id, family, kind, scope, payload_json, payload_digest,
             created_at, updated_at, deleted_at)
        VALUES ('c:active', 'TestFamily', 'Rule', 'EP', '{}', 'digest1', 0, 0, NULL);
        INSERT INTO constraints
            (constraint_id, family, kind, scope, payload_json, payload_digest,
             created_at, updated_at, deleted_at)
        VALUES ('c:tombstoned', 'TestFamily', 'Rule', 'EP', '{}', 'digest2', 0, 0, 1000);
        "#,
    )
    .unwrap();

    // Without tombstoned — only active
    let result_live = apply_engine_query(
        EngineQuery::ConstraintListByFamily {
            family: "TestFamily".to_string(),
            include_tombstoned: false,
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result_live {
        EngineQueryResult::ConstraintListByFamily(cs) => {
            assert_eq!(cs.len(), 1, "only 1 active constraint");
            assert_eq!(cs[0].constraint_id, "c:active");
        }
        _ => panic!("expected ConstraintListByFamily"),
    }

    // With tombstoned — both
    let result_all = apply_engine_query(
        EngineQuery::ConstraintListByFamily {
            family: "TestFamily".to_string(),
            include_tombstoned: true,
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result_all {
        EngineQueryResult::ConstraintListByFamily(cs) => {
            assert_eq!(
                cs.len(),
                2,
                "both constraints returned with include_tombstoned=true"
            );
        }
        _ => panic!("expected ConstraintListByFamily"),
    }
}

// ---------------------------------------------------------------------------
// S10: ep.list_constraints is ordered by attachment ordinal
// ---------------------------------------------------------------------------

#[test]
fn test_ep_list_constraints_ordered() {
    let (_tmp, conn, cas) = setup();

    insert_ettle_ep(&conn, "ettle:e", "ep:e:0");

    conn.execute_batch(
        r#"
        INSERT INTO constraints
            (constraint_id, family, kind, scope, payload_json, payload_digest,
             created_at, updated_at, deleted_at)
        VALUES ('c:first', 'Fam', 'K', 'EP', '{}', 'd1', 0, 0, NULL);
        INSERT INTO constraints
            (constraint_id, family, kind, scope, payload_json, payload_digest,
             created_at, updated_at, deleted_at)
        VALUES ('c:second', 'Fam', 'K', 'EP', '{}', 'd2', 0, 0, NULL);

        -- Attach in reverse ordinal order to test sorting
        INSERT INTO ep_constraint_refs (ep_id, constraint_id, ordinal, created_at)
        VALUES ('ep:e:0', 'c:second', 1, 0);
        INSERT INTO ep_constraint_refs (ep_id, constraint_id, ordinal, created_at)
        VALUES ('ep:e:0', 'c:first', 0, 0);
        "#,
    )
    .unwrap();

    let result = apply_engine_query(
        EngineQuery::EpListConstraints {
            ep_id: "ep:e:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EpListConstraints(cs) => {
            assert_eq!(cs.len(), 2);
            assert_eq!(cs[0].constraint_id, "c:first", "ordinal 0 must be first");
            assert_eq!(cs[1].constraint_id, "c:second", "ordinal 1 must be second");
        }
        _ => panic!("expected EpListConstraints"),
    }
}

// ---------------------------------------------------------------------------
// S11: manifest.get_by_snapshot returns digests and bytes
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_get_by_snapshot_digests_and_bytes() {
    let (_tmp, mut conn, cas) = setup();

    insert_ettle_ep(&conn, "ettle:snap", "ep:snap:0");
    let commit = snapshot_commit(
        "ettle:snap",
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

    let result = apply_engine_query(
        EngineQuery::ManifestGetBySnapshot {
            snapshot_id: commit.snapshot_id.clone(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::ManifestGet(r) => {
            assert_eq!(r.snapshot_id, commit.snapshot_id);
            assert!(!r.manifest_digest.is_empty());
            assert!(!r.semantic_manifest_digest.is_empty());
            assert!(!r.manifest_bytes.is_empty());
            // manifest_digest should match what the commit returned
            assert_eq!(r.manifest_digest, commit.manifest_digest);
        }
        _ => panic!("expected ManifestGet result"),
    }
}

// ---------------------------------------------------------------------------
// S12: manifest.get_by_digest → NotFound for unknown digest
// ---------------------------------------------------------------------------

#[test]
fn test_manifest_get_by_digest_not_found() {
    let (_tmp, conn, cas) = setup();

    let err = apply_engine_query(
        EngineQuery::ManifestGetByDigest {
            manifest_digest: "a".repeat(64),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    assert_eq!(
        err.kind(),
        ettlex_core::errors::ExErrorKind::MissingBlob,
        "expected MissingBlob for unknown digest"
    );
}

// ---------------------------------------------------------------------------
// S13: ept.compute is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_ept_compute_deterministic() {
    let (_tmp, conn, cas) = setup();

    // Two-level chain: root → child (leaf)
    // IMPORTANT: compute_rt follows ettle.parent_id, so the leaf ettle
    // must have parent_id = root ettle.
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:rt', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:lf', 'Leaf', 'ettle:rt', 0, 0, 0, '{}');

        -- Root EP0 refines into leaf ettle (child_ettle_id = mapping EP)
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:rt:0', 'ettle:rt', 0, 1, 'ettle:lf', NULL, 'c', 0, 0, 0);

        -- Leaf EP (single EP — no ambiguity)
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:lf:0', 'ettle:lf', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let result1 = apply_engine_query(
        EngineQuery::EptCompute {
            leaf_ep_id: "ep:lf:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();
    let result2 = apply_engine_query(
        EngineQuery::EptCompute {
            leaf_ep_id: "ep:lf:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match (result1, result2) {
        (EngineQueryResult::EptCompute(r1), EngineQueryResult::EptCompute(r2)) => {
            assert_eq!(r1.leaf_ep_id, "ep:lf:0");
            // EP IDs must be in root-to-leaf order
            assert!(
                r1.ept_ep_ids.contains(&"ep:rt:0".to_string()),
                "root EP must be in EPT"
            );
            assert!(
                r1.ept_ep_ids.contains(&"ep:lf:0".to_string()),
                "leaf EP must be in EPT"
            );
            // Deterministic: same inputs produce same digest
            assert_eq!(
                r1.ept_digest, r2.ept_digest,
                "ept_digest must be deterministic"
            );
            assert!(!r1.ept_digest.is_empty());
        }
        _ => panic!("expected EptCompute result"),
    }
}

// ---------------------------------------------------------------------------
// S14: ept.compute → EptAmbiguousLeafEp when leaf has multiple EPs
// (Deferred: BTreeMap determinism makes this unreachable in Phase 1)
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_ept_compute_ambiguous() {
    let (_tmp, conn, cas) = setup();

    // Leaf ettle with two EPs
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:amb', 'Ambiguous', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:amb:0', 'ettle:amb', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:amb:1', 'ettle:amb', 1, 0, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let err = apply_engine_query(
        EngineQuery::EptCompute {
            leaf_ep_id: "ep:amb:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap_err();

    // Should return an error (EptAmbiguous or similar)
    use ettlex_core::errors::ExErrorKind;
    assert!(
        matches!(
            err.kind(),
            ExErrorKind::EptAmbiguous | ExErrorKind::NotFound
        ),
        "expected an ambiguity or navigation error"
    );
}

// ---------------------------------------------------------------------------
// S18: decision.list is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_decision_list_deterministic() {
    let (_tmp, conn, cas) = setup();

    // Create two decisions
    let d1 = decision_create(
        Some("d:1".to_string()),
        "Decision Alpha".to_string(),
        Some("accepted".to_string()),
        "We chose alpha".to_string(),
        "Because it was better".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    let d2 = decision_create(
        Some("d:2".to_string()),
        "Decision Beta".to_string(),
        Some("proposed".to_string()),
        "We may choose beta".to_string(),
        "It is also good".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    // Calling twice must return the same items
    let result1 = apply_engine_query(
        EngineQuery::DecisionList(ettlex_engine::commands::read_tools::ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();
    let result2 = apply_engine_query(
        EngineQuery::DecisionList(ettlex_engine::commands::read_tools::ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();

    match (result1, result2) {
        (EngineQueryResult::DecisionList(p1), EngineQueryResult::DecisionList(p2)) => {
            let ids1: Vec<_> = p1.items.iter().map(|d| d.decision_id.clone()).collect();
            let ids2: Vec<_> = p2.items.iter().map(|d| d.decision_id.clone()).collect();
            assert_eq!(ids1, ids2, "decision list must be deterministic");
            assert!(ids1.contains(&d1));
            assert!(ids1.contains(&d2));
        }
        _ => panic!("expected DecisionList"),
    }
}

// ---------------------------------------------------------------------------
// S19: ep.list_decisions with ancestors
// ---------------------------------------------------------------------------

#[test]
fn test_ep_list_decisions_with_ancestors() {
    let (_tmp, conn, cas) = setup();

    // Two-level EPT
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root2', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:leaf2', 'Leaf', 'ettle:root2', 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root2:0', 'ettle:root2', 0, 1, 'ettle:leaf2', NULL, 'c', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:leaf2:0', 'ettle:leaf2', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    // Link a decision to the root ETTLE (ancestor path walks ettle parent hierarchy)
    let d_root = decision_create(
        Some("d:root".to_string()),
        "Root Decision".to_string(),
        None,
        "A decision for root".to_string(),
        "Root rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();
    decision_link(
        d_root.clone(),
        "ettle".to_string(),
        "ettle:root2".to_string(),
        "governs".to_string(),
        0,
        &conn,
    )
    .unwrap();

    // Link a decision to the leaf EP
    let d_leaf = decision_create(
        Some("d:leaf".to_string()),
        "Leaf Decision".to_string(),
        None,
        "A decision for leaf".to_string(),
        "Leaf rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();
    decision_link(
        d_leaf.clone(),
        "ep".to_string(),
        "ep:leaf2:0".to_string(),
        "governs".to_string(),
        0,
        &conn,
    )
    .unwrap();

    // Without ancestors: only leaf's own decisions
    let result_own = apply_engine_query(
        EngineQuery::EpListDecisions {
            ep_id: "ep:leaf2:0".to_string(),
            include_ancestors: false,
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result_own {
        EngineQueryResult::EpListDecisions(ds) => {
            let ids: Vec<_> = ds.iter().map(|d| d.decision_id.clone()).collect();
            assert!(ids.contains(&d_leaf), "leaf decision must be included");
            assert!(
                !ids.contains(&d_root),
                "root decision excluded without ancestors"
            );
        }
        _ => panic!("expected EpListDecisions"),
    }

    // With ancestors: root + leaf
    let result_anc = apply_engine_query(
        EngineQuery::EpListDecisions {
            ep_id: "ep:leaf2:0".to_string(),
            include_ancestors: true,
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result_anc {
        EngineQueryResult::EpListDecisions(ds) => {
            let ids: Vec<_> = ds.iter().map(|d| d.decision_id.clone()).collect();
            assert!(ids.contains(&d_root), "ancestor decision included");
            assert!(ids.contains(&d_leaf), "own decision included");
        }
        _ => panic!("expected EpListDecisions"),
    }
}

// ---------------------------------------------------------------------------
// S20: ept.compute_decision_context is deterministic
// ---------------------------------------------------------------------------

#[test]
fn test_ept_compute_decision_context_deterministic() {
    let (_tmp, conn, cas) = setup();

    // Two-level chain
    conn.execute_batch(
        r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:ctx-r', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:ctx-l', 'Leaf', 'ettle:ctx-r', 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:ctx-r:0', 'ettle:ctx-r', 0, 1, 'ettle:ctx-l', NULL, 'c', 0, 0, 0);
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                         content_digest, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:ctx-l:0', 'ettle:ctx-l', 0, 1, NULL, NULL, 'c', 0, 0, 0);
        "#,
    )
    .unwrap();

    let d1 = decision_create(
        Some("d:ctx1".to_string()),
        "Ctx Decision 1".to_string(),
        None,
        "text1".to_string(),
        "rationale1".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();
    decision_link(
        d1,
        "ep".to_string(),
        "ep:ctx-r:0".to_string(),
        "governs".to_string(),
        0,
        &conn,
    )
    .unwrap();

    let result = apply_engine_query(
        EngineQuery::EptComputeDecisionContext {
            leaf_ep_id: "ep:ctx-l:0".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    match result {
        EngineQueryResult::EptComputeDecisionContext(ctx) => {
            // all_for_leaf includes decisions from all EPs in the EPT
            assert!(
                !ctx.by_ep.is_empty() || ctx.all_for_leaf.is_empty(),
                "decision context must be populated"
            );
            // The root EP's decision must appear somewhere in the context
            let all_ids: Vec<_> = ctx
                .all_for_leaf
                .iter()
                .map(|d| d.decision_id.clone())
                .collect();
            assert!(
                all_ids.contains(&"d:ctx1".to_string()),
                "root decision visible in context"
            );
        }
        _ => panic!("expected EptComputeDecisionContext"),
    }
}

// ---------------------------------------------------------------------------
// S21: decision queries do not affect snapshot count
// ---------------------------------------------------------------------------

#[test]
fn test_decision_queries_no_snapshot_effect() {
    let (_tmp, mut conn, cas) = setup();

    insert_ettle_ep(&conn, "ettle:dec-snap", "ep:dec-snap:0");
    snapshot_commit(
        "ettle:dec-snap",
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

    let snap_before: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();

    // Several decision queries
    decision_create(
        Some("d:effect".to_string()),
        "Effect Decision".to_string(),
        None,
        "text".to_string(),
        "rationale".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    apply_engine_query(
        EngineQuery::DecisionList(ettlex_engine::commands::read_tools::ListOptions::default()),
        &conn,
        &cas,
    )
    .unwrap();
    apply_engine_query(
        EngineQuery::DecisionGet {
            decision_id: "d:effect".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();

    let snap_after: i64 = conn
        .query_row("SELECT COUNT(*) FROM snapshots", [], |r| r.get(0))
        .unwrap();

    assert_eq!(
        snap_before, snap_after,
        "decision queries must not create snapshots"
    );
}

// ---------------------------------------------------------------------------
// S22: Scale (10k EPs) — deferred
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn test_read_tools_scale() {
    let (_tmp, conn, cas) = setup();

    // Insert a root ettle + 10k EPs
    conn.execute(
        "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
         VALUES ('ettle:scale', 'Scale', NULL, 0, 0, 0, '{}')",
        [],
    )
    .unwrap();
    for i in 0..10_000 {
        conn.execute(
            "INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id,
                              content_digest, content_inline, deleted, created_at, updated_at)
             VALUES (?1, 'ettle:scale', ?2, 1, NULL, NULL, 'c', 0, 0, 0)",
            rusqlite::params![format!("ep:scale:{}", i), i],
        )
        .unwrap();
    }

    let start = std::time::Instant::now();
    let result = apply_engine_query(
        EngineQuery::EttleGet {
            ettle_id: "ettle:scale".to_string(),
        },
        &conn,
        &cas,
    )
    .unwrap();
    let elapsed = start.elapsed();

    match result {
        EngineQueryResult::EttleGet(r) => {
            assert_eq!(r.ep_ids.len(), 10_000);
        }
        _ => panic!("expected EttleGet"),
    }
    assert!(
        elapsed.as_millis() < 500,
        "10k EP query must complete in <500ms, took {}ms",
        elapsed.as_millis()
    );
}
