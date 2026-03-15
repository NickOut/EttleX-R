//! Engine handler for Ettle CRUD operations — Slice 01.
//!
//! This module owns all invariant enforcement for Ettle create / update /
//! tombstone / get / list.  It delegates persistence to `SqliteRepo` and
//! never writes raw SQL itself.

#![allow(clippy::result_large_err)]

use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_store::model::{EttleListOpts, EttleListPage, EttleRecord};
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;

type Result<T> = std::result::Result<T, ExError>;

// ---------------------------------------------------------------------------
// handle_ettle_create
// ---------------------------------------------------------------------------

/// Create a new Ettle with the given fields.
///
/// Invariants enforced:
/// - Title must be non-empty and non-whitespace-only (`InvalidTitle`).
/// - `reasoning_link_id` and `reasoning_link_type` must both be present or
///   both absent (`MissingLinkType`).
/// - If `reasoning_link_id` is supplied, the target must exist (`NotFound`)
///   and must not be tombstoned (`AlreadyTombstoned`).
pub(crate) fn handle_ettle_create(
    conn: &mut Connection,
    title: &str,
    why: Option<&str>,
    what: Option<&str>,
    how: Option<&str>,
    reasoning_link_id: Option<&str>,
    reasoning_link_type: Option<&str>,
) -> Result<String> {
    // Title validation
    if title.trim().is_empty() {
        return Err(ExError::new(ExErrorKind::InvalidTitle)
            .with_op("ettle_create")
            .with_message("title must not be empty or whitespace-only"));
    }

    // Link consistency: id and type must both be present or both absent
    match (reasoning_link_id, reasoning_link_type) {
        (Some(_), None) | (None, Some(_)) => {
            return Err(ExError::new(ExErrorKind::MissingLinkType)
                .with_op("ettle_create")
                .with_message(
                    "reasoning_link_id and reasoning_link_type must be supplied together",
                ));
        }
        _ => {}
    }

    // If a link is supplied, validate the target
    if let Some(link_id) = reasoning_link_id {
        let target = SqliteRepo::get_ettle_record(conn, link_id)?.ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_op("ettle_create")
                .with_entity_id(link_id)
                .with_message(format!("reasoning_link target not found: {}", link_id))
        })?;
        if target.tombstoned_at.is_some() {
            return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
                .with_op("ettle_create")
                .with_entity_id(link_id)
                .with_message(format!("reasoning_link target is tombstoned: {}", link_id)));
        }
    }

    // Generate ID and timestamps
    let id = format!("ettle:{}", uuid::Uuid::now_v7());
    let now = chrono::Utc::now().to_rfc3339();

    SqliteRepo::insert_ettle(
        conn,
        &id,
        title,
        why.unwrap_or(""),
        what.unwrap_or(""),
        how.unwrap_or(""),
        reasoning_link_id,
        reasoning_link_type,
        &now,
        &now,
    )?;

    Ok(id)
}

// ---------------------------------------------------------------------------
// handle_ettle_update
// ---------------------------------------------------------------------------

/// Update an existing Ettle's content fields.
///
/// Invariants enforced:
/// - At least one field must be `Some` (`EmptyUpdate`).
/// - Target must exist (`NotFound`) and not be tombstoned (`AlreadyTombstoned`).
/// - Self-referential link: `reasoning_link_id == Some(Some(ettle_id))` (`SelfReferentialLink`).
/// - After merge, if link id is set but type is absent (neither supplied nor in existing
///   record), returns `MissingLinkType`.
/// - Link target must exist and not be tombstoned.
#[allow(clippy::too_many_arguments)]
pub(crate) fn handle_ettle_update(
    conn: &mut Connection,
    ettle_id: &str,
    title: Option<&str>,
    why: Option<&str>,
    what: Option<&str>,
    how: Option<&str>,
    reasoning_link_id: Option<Option<&str>>,
    reasoning_link_type: Option<Option<&str>>,
) -> Result<()> {
    // EmptyUpdate guard: at least one field must be supplied
    let any_supplied = title.is_some()
        || why.is_some()
        || what.is_some()
        || how.is_some()
        || reasoning_link_id.is_some()
        || reasoning_link_type.is_some();
    if !any_supplied {
        return Err(ExError::new(ExErrorKind::EmptyUpdate)
            .with_op("ettle_update")
            .with_entity_id(ettle_id)
            .with_message("EttleUpdate requires at least one field"));
    }

    // Fetch existing record
    let existing = SqliteRepo::get_ettle_record(conn, ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("ettle_update")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle not found: {}", ettle_id))
    })?;

    if existing.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("ettle_update")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle is tombstoned: {}", ettle_id)));
    }

    // Self-referential link check
    if let Some(Some(link_id)) = reasoning_link_id {
        if link_id == ettle_id {
            return Err(ExError::new(ExErrorKind::SelfReferentialLink)
                .with_op("ettle_update")
                .with_entity_id(ettle_id)
                .with_message("An Ettle cannot link to itself"));
        }
    }

    // Compute merged link state to validate consistency
    let merged_link_id: Option<&str> = match reasoning_link_id {
        Some(Some(v)) => Some(v),
        Some(None) => None,                            // cleared
        None => existing.reasoning_link_id.as_deref(), // preserved
    };
    let merged_link_type: Option<&str> = match reasoning_link_type {
        Some(Some(v)) => Some(v),
        Some(None) => None,
        None => existing.reasoning_link_type.as_deref(),
    };

    // If we're clearing the link id, also clear the type automatically (even if type was
    // being preserved). This handles SC-28 where clearing reasoning_link_id should also
    // clear reasoning_link_type.
    let (effective_link_id, effective_link_type) = if reasoning_link_id == Some(None) {
        (None, None)
    } else {
        (merged_link_id, merged_link_type)
    };

    // Link consistency: id and type must both be present or both absent
    match (effective_link_id, effective_link_type) {
        (Some(_), None) | (None, Some(_)) => {
            return Err(ExError::new(ExErrorKind::MissingLinkType)
                .with_op("ettle_update")
                .with_entity_id(ettle_id)
                .with_message(
                    "reasoning_link_id and reasoning_link_type must be supplied together",
                ));
        }
        _ => {}
    }

    // Validate link target if set
    if let Some(link_id) = effective_link_id {
        let target = SqliteRepo::get_ettle_record(conn, link_id)?.ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_op("ettle_update")
                .with_entity_id(link_id)
                .with_message(format!("reasoning_link target not found: {}", link_id))
        })?;
        if target.tombstoned_at.is_some() {
            return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
                .with_op("ettle_update")
                .with_entity_id(link_id)
                .with_message(format!("reasoning_link target is tombstoned: {}", link_id)));
        }
    }

    let now = chrono::Utc::now().to_rfc3339();

    // When clearing the link (reasoning_link_id == Some(None)), we need to pass
    // Some(None) for BOTH link_id and link_type to the store layer.
    let store_link_id = if reasoning_link_id == Some(None) {
        Some(None) // clear
    } else {
        reasoning_link_id
    };
    let store_link_type = if reasoning_link_id == Some(None) {
        Some(None) // also clear type when link is cleared
    } else {
        reasoning_link_type
    };

    SqliteRepo::update_ettle(
        conn,
        ettle_id,
        title,
        why,
        what,
        how,
        store_link_id,
        store_link_type,
        &now,
    )?;

    Ok(())
}

// ---------------------------------------------------------------------------
// handle_ettle_tombstone
// ---------------------------------------------------------------------------

/// Tombstone an Ettle (soft delete).
///
/// Invariants enforced:
/// - Target must exist (`NotFound`) and not already be tombstoned (`AlreadyTombstoned`).
/// - Must have no active (non-tombstoned) dependants (`HasActiveDependants`).
pub(crate) fn handle_ettle_tombstone(conn: &mut Connection, ettle_id: &str) -> Result<()> {
    // Fetch existing record
    let existing = SqliteRepo::get_ettle_record(conn, ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("ettle_tombstone")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle not found: {}", ettle_id))
    })?;

    if existing.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("ettle_tombstone")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle is already tombstoned: {}", ettle_id)));
    }

    // Check active dependants
    let count = SqliteRepo::get_active_ettle_dependants_count(conn, ettle_id)?;
    if count > 0 {
        return Err(ExError::new(ExErrorKind::HasActiveDependants)
            .with_op("ettle_tombstone")
            .with_entity_id(ettle_id)
            .with_message(format!(
                "Ettle has {} active dependant(s) that must be tombstoned first",
                count
            )));
    }

    let now = chrono::Utc::now().to_rfc3339();
    SqliteRepo::tombstone_ettle(conn, ettle_id, &now)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_ettle_get
// ---------------------------------------------------------------------------

/// Get an Ettle record by ID.
///
/// Returns `NotFound` if the Ettle does not exist.
/// Returns the record even if tombstoned (callers can inspect `tombstoned_at`).
pub fn handle_ettle_get(conn: &Connection, ettle_id: &str) -> Result<EttleRecord> {
    SqliteRepo::get_ettle_record(conn, ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("ettle_get")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle not found: {}", ettle_id))
    })
}

// ---------------------------------------------------------------------------
// handle_ettle_list
// ---------------------------------------------------------------------------

/// List Ettles with cursor-based pagination.
///
/// Invariants enforced:
/// - `limit` must be 1..=500 (`InvalidInput`).
pub fn handle_ettle_list(conn: &Connection, opts: EttleListOpts) -> Result<EttleListPage> {
    if opts.limit == 0 {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("ettle_list")
            .with_message("limit must be at least 1"));
    }
    if opts.limit > 500 {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("ettle_list")
            .with_message("limit must not exceed 500"));
    }

    SqliteRepo::list_ettles(conn, &opts)
}
