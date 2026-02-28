// Integration tests for decision command handlers.
// Covers create, update, tombstone, link, unlink, and supersede operations.

use ettlex_engine::commands::decision::{
    decision_create, decision_link, decision_supersede, decision_tombstone, decision_unlink,
    decision_update,
};
use rusqlite::Connection;
use tempfile::TempDir;

fn setup_db() -> (TempDir, Connection) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let mut conn = Connection::open(&db_path).unwrap();
    ettlex_store::migrations::apply_migrations(&mut conn).unwrap();
    (temp_dir, conn)
}

fn seed_ep(conn: &Connection) {
    conn.execute_batch(r#"
        INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
        VALUES ('ettle:root', 'Root', NULL, 0, 0, 0, '{}');
        INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at)
        VALUES ('ep:root:0', 'ettle:root', 0, 1, NULL, 'leaf content', 0, 0, 0);
    "#).unwrap();
}

// ---------------------------------------------------------------------------
// decision_create
// ---------------------------------------------------------------------------

#[test]
fn test_decision_create_happy_path() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        None,
        "Adopt Rust for backend".to_string(),
        Some("proposed".to_string()),
        "We adopt Rust as the primary backend language.".to_string(),
        "Performance and memory safety.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    assert!(!id.is_empty());

    // Verify in DB
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM decisions WHERE decision_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_decision_create_with_explicit_id() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        Some("decision:001".to_string()),
        "Use TDD".to_string(),
        None,
        "Test-driven development is mandatory.".to_string(),
        "Better correctness.".to_string(),
        Some("No alternatives considered.".to_string()),
        Some("More time writing tests.".to_string()),
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    assert_eq!(id, "decision:001");
}

#[test]
fn test_decision_create_with_excerpt_evidence() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        None,
        "Evidence decision".to_string(),
        Some("accepted".to_string()),
        "Decision body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "excerpt".to_string(),
        Some("This excerpt supports the decision.".to_string()),
        None,
        None,
        &conn,
    )
    .unwrap();

    assert!(!id.is_empty());
}

// ---------------------------------------------------------------------------
// decision_update
// ---------------------------------------------------------------------------

#[test]
fn test_decision_update_title() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        Some("decision:upd".to_string()),
        "Original title".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    decision_update(
        id.clone(),
        Some("Updated title".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    let title: String = conn
        .query_row(
            "SELECT title FROM decisions WHERE decision_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(title, "Updated title");
}

// ---------------------------------------------------------------------------
// decision_tombstone
// ---------------------------------------------------------------------------

#[test]
fn test_decision_tombstone() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        Some("decision:tomb".to_string()),
        "To be tombstoned".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    decision_tombstone(id.clone(), &conn).unwrap();

    let tombstoned_at: Option<i64> = conn
        .query_row(
            "SELECT tombstoned_at FROM decisions WHERE decision_id = ?1",
            [&id],
            |r| r.get(0),
        )
        .unwrap();
    assert!(tombstoned_at.is_some());
}

#[test]
fn test_decision_tombstone_nonexistent_fails() {
    let (_tmp, conn) = setup_db();

    let result = decision_tombstone("decision:nonexistent".to_string(), &conn);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// decision_link / decision_unlink
// ---------------------------------------------------------------------------

#[test]
fn test_decision_link_to_ep() {
    let (_tmp, conn) = setup_db();
    seed_ep(&conn);

    let decision_id = decision_create(
        Some("decision:lnk".to_string()),
        "Link decision".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
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
        decision_id.clone(),
        "ep".to_string(),
        "ep:root:0".to_string(),
        "grounds".to_string(),
        0,
        &conn,
    )
    .unwrap();

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM decision_links WHERE decision_id = ?1",
            [&decision_id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1);
}

#[test]
fn test_decision_unlink_from_ep() {
    let (_tmp, conn) = setup_db();
    seed_ep(&conn);

    let decision_id = decision_create(
        Some("decision:unlnk".to_string()),
        "Unlink decision".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
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
        decision_id.clone(),
        "ep".to_string(),
        "ep:root:0".to_string(),
        "grounds".to_string(),
        0,
        &conn,
    )
    .unwrap();

    decision_unlink(
        decision_id.clone(),
        "ep".to_string(),
        "ep:root:0".to_string(),
        "grounds".to_string(),
        &conn,
    )
    .unwrap();
}

// ---------------------------------------------------------------------------
// decision_supersede
// ---------------------------------------------------------------------------

#[test]
fn test_decision_supersede() {
    let (_tmp, conn) = setup_db();

    let old_id = decision_create(
        Some("decision:old".to_string()),
        "Old decision".to_string(),
        Some("superseded".to_string()),
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    let new_id = decision_create(
        Some("decision:new".to_string()),
        "New decision".to_string(),
        Some("accepted".to_string()),
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "none".to_string(),
        None,
        None,
        None,
        &conn,
    )
    .unwrap();

    decision_supersede(old_id, new_id, &conn).unwrap();
}

// ---------------------------------------------------------------------------
// Error path tests (cover log_op_error! branches)
// ---------------------------------------------------------------------------

#[test]
fn test_decision_update_nonexistent_fails() {
    let (_tmp, conn) = setup_db();

    let result = decision_update(
        "decision:nonexistent".to_string(),
        Some("Title".to_string()),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        &conn,
    );
    assert!(result.is_err());
}

#[test]
fn test_decision_link_nonexistent_decision_fails() {
    let (_tmp, conn) = setup_db();
    seed_ep(&conn);

    let result = decision_link(
        "decision:nonexistent".to_string(),
        "ep".to_string(),
        "ep:root:0".to_string(),
        "grounds".to_string(),
        0,
        &conn,
    );
    assert!(result.is_err());
}

#[test]
fn test_decision_supersede_nonexistent_fails() {
    let (_tmp, conn) = setup_db();

    let result = decision_supersede(
        "decision:nonexistent-old".to_string(),
        "decision:nonexistent-new".to_string(),
        &conn,
    );
    assert!(result.is_err());
}

#[test]
fn test_decision_create_with_capture_content() {
    let (_tmp, conn) = setup_db();

    let id = decision_create(
        Some("decision:cap".to_string()),
        "Capture evidence".to_string(),
        None,
        "Body.".to_string(),
        "Rationale.".to_string(),
        None,
        None,
        "capture".to_string(),
        None,
        Some("Captured conversation content.".to_string()),
        None,
        &conn,
    )
    .unwrap();

    assert_eq!(id, "decision:cap");

    // Evidence item should exist
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM decision_evidence_items", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(count, 1);
}
