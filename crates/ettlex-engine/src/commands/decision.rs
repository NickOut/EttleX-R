//! Decision command handlers with boundary logging.
//!
//! This module provides command handlers for decision operations:
//! - Create, update, tombstone decisions
//! - Link/unlink decisions to targets
//! - Supersede decisions
//!
//! ## Logging Ownership
//!
//! The engine layer owns lifecycle logging for decision operations:
//! - `log_op_start!` at entry
//! - `log_op_end!` on success
//! - `log_op_error!` on failure
//!
//! Lower layers (store, core) use only `tracing::debug!()` for internal details.

#![allow(clippy::result_large_err)]

use ettlex_core::ops::decision_ops;
use ettlex_core::{log_op_end, log_op_error, log_op_start};
use ettlex_store::errors::Result;
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;

/// Create a new decision
///
/// ## Arguments
///
/// - `decision_id`: Optional decision ID (generates UUIDv7 if None)
/// - `title`: Decision title
/// - `status`: Decision status (e.g., "proposed", "accepted")
/// - `decision_text`: Decision text
/// - `rationale`: Decision rationale
/// - `alternatives_text`: Optional alternatives considered
/// - `consequences_text`: Optional consequences
/// - `evidence_kind`: Evidence kind ("none", "excerpt", "capture", "file")
/// - `evidence_excerpt`: Optional evidence excerpt
/// - `evidence_capture_content`: Optional evidence capture content
/// - `evidence_file_path`: Optional evidence file path
/// - `conn`: Database connection
///
/// ## Returns
///
/// Decision ID (generated or provided)
///
/// ## Errors
///
/// - `InvalidDecision`: Validation failed
/// - `InvalidEvidence`: Evidence validation failed
/// - `Persistence`: Database error
#[allow(clippy::too_many_arguments)]
pub fn decision_create(
    decision_id: Option<String>,
    title: String,
    status: Option<String>,
    decision_text: String,
    rationale: String,
    alternatives_text: Option<String>,
    consequences_text: Option<String>,
    evidence_kind: String,
    evidence_excerpt: Option<String>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<String>,
    conn: &Connection,
) -> Result<String> {
    log_op_start!("decision_create", title = &title);
    let start = std::time::Instant::now();

    let result = decision_create_impl(
        decision_id,
        title,
        status,
        decision_text,
        rationale,
        alternatives_text,
        consequences_text,
        evidence_kind,
        evidence_excerpt,
        evidence_capture_content,
        evidence_file_path,
        conn,
    )
    .map_err(|e| {
        log_op_error!(
            "decision_create",
            e.clone(),
            duration_ms = start.elapsed().as_millis() as u64
        );
        e
    })?;

    log_op_end!(
        "decision_create",
        duration_ms = start.elapsed().as_millis() as u64,
        decision_id = &result
    );

    Ok(result)
}

#[allow(clippy::too_many_arguments)]
fn decision_create_impl(
    decision_id: Option<String>,
    title: String,
    status: Option<String>,
    decision_text: String,
    rationale: String,
    alternatives_text: Option<String>,
    consequences_text: Option<String>,
    evidence_kind: String,
    evidence_excerpt: Option<String>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<String>,
    conn: &Connection,
) -> Result<String> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    let decision_id = decision_ops::create_decision(
        &mut store,
        decision_id,
        title,
        status,
        decision_text,
        rationale,
        alternatives_text,
        consequences_text,
        evidence_kind,
        evidence_excerpt,
        evidence_capture_content,
        evidence_file_path,
    )?;

    // Persist decision
    let decision = store.get_decision(&decision_id)?;
    SqliteRepo::persist_decision(conn, decision)?;

    // Persist evidence item if created
    if let Some(ref capture_id) = decision.evidence_capture_id {
        if let Ok(item) = store.get_evidence_item(capture_id) {
            SqliteRepo::persist_evidence_item(conn, item)?;
        }
    }

    Ok(decision_id)
}

/// Update a decision
///
/// ## Arguments
///
/// - `decision_id`: Decision ID to update
/// - `title`: Optional new title
/// - `status`: Optional new status
/// - `decision_text`: Optional new decision text
/// - `rationale`: Optional new rationale
/// - `alternatives_text`: Optional new alternatives (double Option for clearing)
/// - `consequences_text`: Optional new consequences (double Option for clearing)
/// - `evidence_kind`: Optional new evidence kind
/// - `evidence_excerpt`: Optional new evidence excerpt (double Option for clearing)
/// - `evidence_capture_content`: Optional new evidence capture content
/// - `evidence_file_path`: Optional new evidence file path (double Option for clearing)
/// - `conn`: Database connection
///
/// ## Errors
///
/// - `DecisionNotFound`: Decision doesn't exist
/// - `DecisionDeleted`: Decision was tombstoned
/// - `InvalidEvidence`: Evidence validation failed
/// - `Persistence`: Database error
#[allow(clippy::too_many_arguments)]
pub fn decision_update(
    decision_id: String,
    title: Option<String>,
    status: Option<String>,
    decision_text: Option<String>,
    rationale: Option<String>,
    alternatives_text: Option<Option<String>>,
    consequences_text: Option<Option<String>>,
    evidence_kind: Option<String>,
    evidence_excerpt: Option<Option<String>>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<Option<String>>,
    conn: &Connection,
) -> Result<()> {
    log_op_start!("decision_update", decision_id = &decision_id);
    let start = std::time::Instant::now();

    decision_update_impl(
        decision_id.clone(),
        title,
        status,
        decision_text,
        rationale,
        alternatives_text,
        consequences_text,
        evidence_kind,
        evidence_excerpt,
        evidence_capture_content,
        evidence_file_path,
        conn,
    )
    .map_err(|e| {
        log_op_error!(
            "decision_update",
            e.clone(),
            duration_ms = start.elapsed().as_millis() as u64
        );
        e
    })?;

    log_op_end!(
        "decision_update",
        duration_ms = start.elapsed().as_millis() as u64
    );

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn decision_update_impl(
    decision_id: String,
    title: Option<String>,
    status: Option<String>,
    decision_text: Option<String>,
    rationale: Option<String>,
    alternatives_text: Option<Option<String>>,
    consequences_text: Option<Option<String>>,
    evidence_kind: Option<String>,
    evidence_excerpt: Option<Option<String>>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<Option<String>>,
    conn: &Connection,
) -> Result<()> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    decision_ops::update_decision(
        &mut store,
        &decision_id,
        title,
        status,
        decision_text,
        rationale,
        alternatives_text,
        consequences_text,
        evidence_kind,
        evidence_excerpt,
        evidence_capture_content,
        evidence_file_path,
    )?;

    // Persist decision
    let decision = store.get_decision(&decision_id)?;
    SqliteRepo::persist_decision(conn, decision)?;

    // Persist evidence item if created
    if let Some(ref capture_id) = decision.evidence_capture_id {
        if let Ok(item) = store.get_evidence_item(capture_id) {
            SqliteRepo::persist_evidence_item(conn, item)?;
        }
    }

    Ok(())
}

/// Tombstone a decision (soft delete)
///
/// ## Arguments
///
/// - `decision_id`: Decision ID to tombstone
/// - `conn`: Database connection
///
/// ## Errors
///
/// - `DecisionNotFound`: Decision doesn't exist
/// - `DecisionDeleted`: Decision already tombstoned
/// - `Persistence`: Database error
pub fn decision_tombstone(decision_id: String, conn: &Connection) -> Result<()> {
    log_op_start!("decision_tombstone", decision_id = &decision_id);
    let start = std::time::Instant::now();

    decision_tombstone_impl(decision_id.clone(), conn).map_err(|e| {
        log_op_error!(
            "decision_tombstone",
            e.clone(),
            duration_ms = start.elapsed().as_millis() as u64
        );
        e
    })?;

    log_op_end!(
        "decision_tombstone",
        duration_ms = start.elapsed().as_millis() as u64
    );

    Ok(())
}

fn decision_tombstone_impl(decision_id: String, conn: &Connection) -> Result<()> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    decision_ops::tombstone_decision(&mut store, &decision_id)?;

    // Persist decision
    let decision = store.get_decision(&decision_id)?;
    SqliteRepo::persist_decision(conn, decision)?;

    Ok(())
}

/// Link a decision to a target (EP/Ettle/Constraint/Decision)
///
/// ## Arguments
///
/// - `decision_id`: Decision ID to link
/// - `target_kind`: Target kind ("ep", "ettle", "constraint", "decision")
/// - `target_id`: Target ID
/// - `relation_kind`: Relation kind ("grounds", "constrains", "motivates", "supersedes")
/// - `ordinal`: Link ordinal for deterministic ordering
/// - `conn`: Database connection
///
/// ## Errors
///
/// - `DecisionNotFound`: Decision doesn't exist
/// - `DecisionTombstoned`: Decision was tombstoned
/// - `InvalidTargetKind`: Target kind not allowed
/// - `EpNotFound`/`EttleNotFound`: Target doesn't exist
/// - `DuplicateDecisionLink`: Link already exists
/// - `Persistence`: Database error
pub fn decision_link(
    decision_id: String,
    target_kind: String,
    target_id: String,
    relation_kind: String,
    ordinal: i32,
    conn: &Connection,
) -> Result<()> {
    log_op_start!(
        "decision_link",
        decision_id = &decision_id,
        target_id = &target_id
    );
    let start = std::time::Instant::now();

    decision_link_impl(
        decision_id.clone(),
        target_kind,
        target_id,
        relation_kind,
        ordinal,
        conn,
    )
    .map_err(|e| {
        log_op_error!(
            "decision_link",
            e.clone(),
            duration_ms = start.elapsed().as_millis() as u64
        );
        e
    })?;

    log_op_end!(
        "decision_link",
        duration_ms = start.elapsed().as_millis() as u64
    );

    Ok(())
}

fn decision_link_impl(
    decision_id: String,
    target_kind: String,
    target_id: String,
    relation_kind: String,
    ordinal: i32,
    conn: &Connection,
) -> Result<()> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    decision_ops::attach_decision_to_target(
        &mut store,
        &decision_id,
        target_kind.clone(),
        target_id.clone(),
        relation_kind.clone(),
        ordinal,
    )?;

    // Persist decision link
    if let Some(link) =
        store.get_decision_link(&decision_id, &target_kind, &target_id, &relation_kind)
    {
        SqliteRepo::persist_decision_link(conn, link)?;
    }

    Ok(())
}

/// Unlink a decision from a target
///
/// ## Arguments
///
/// - `decision_id`: Decision ID to unlink
/// - `target_kind`: Target kind
/// - `target_id`: Target ID
/// - `relation_kind`: Relation kind
/// - `conn`: Database connection
///
/// ## Errors
///
/// - `DecisionLinkNotFound`: Link doesn't exist
/// - `Persistence`: Database error
pub fn decision_unlink(
    decision_id: String,
    target_kind: String,
    target_id: String,
    relation_kind: String,
    conn: &Connection,
) -> Result<()> {
    log_op_start!(
        "decision_unlink",
        decision_id = &decision_id,
        target_id = &target_id
    );
    let start = std::time::Instant::now();

    decision_unlink_impl(
        decision_id.clone(),
        target_kind,
        target_id,
        relation_kind,
        conn,
    )
    .map_err(|e| {
        log_op_error!(
            "decision_unlink",
            e.clone(),
            duration_ms = start.elapsed().as_millis() as u64
        );
        e
    })?;

    log_op_end!(
        "decision_unlink",
        duration_ms = start.elapsed().as_millis() as u64
    );

    Ok(())
}

fn decision_unlink_impl(
    decision_id: String,
    target_kind: String,
    target_id: String,
    relation_kind: String,
    conn: &Connection,
) -> Result<()> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    decision_ops::detach_decision_from_target(
        &mut store,
        &decision_id,
        &target_kind,
        &target_id,
        &relation_kind,
    )?;

    // Note: Link removal is handled by decision_ops::detach_decision_from_target
    // No persistence needed for removed links in Phase 1 (they're removed from store)

    Ok(())
}

/// Mark one decision as superseding another
///
/// ## Arguments
///
/// - `old_decision_id`: Decision being superseded
/// - `new_decision_id`: Decision that supersedes
/// - `conn`: Database connection
///
/// ## Errors
///
/// - `DecisionNotFound`: Either decision doesn't exist
/// - `DuplicateDecisionLink`: Supersession link already exists
/// - `Persistence`: Database error
pub fn decision_supersede(
    old_decision_id: String,
    new_decision_id: String,
    conn: &Connection,
) -> Result<()> {
    log_op_start!(
        "decision_supersede",
        old_decision_id = &old_decision_id,
        new_decision_id = &new_decision_id
    );
    let start = std::time::Instant::now();

    decision_supersede_impl(old_decision_id.clone(), new_decision_id.clone(), conn).map_err(
        |e| {
            log_op_error!(
                "decision_supersede",
                e.clone(),
                duration_ms = start.elapsed().as_millis() as u64
            );
            e
        },
    )?;

    log_op_end!(
        "decision_supersede",
        duration_ms = start.elapsed().as_millis() as u64
    );

    Ok(())
}

fn decision_supersede_impl(
    old_decision_id: String,
    new_decision_id: String,
    conn: &Connection,
) -> Result<()> {
    // Load current store
    let mut store = ettlex_store::repo::hydration::load_tree(conn)?;

    // Apply command
    decision_ops::supersede_decision(&mut store, &old_decision_id, &new_decision_id)?;

    // Persist decision link (decision -> decision with relation_kind = "supersedes")
    if let Some(link) =
        store.get_decision_link(&new_decision_id, "decision", &old_decision_id, "supersedes")
    {
        SqliteRepo::persist_decision_link(conn, link)?;
    }

    Ok(())
}
