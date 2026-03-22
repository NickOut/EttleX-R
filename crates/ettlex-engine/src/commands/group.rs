//! Engine handler for Group and GroupMember CRUD operations — Slice 02.
//!
//! This module owns all invariant enforcement for Group create / get / list /
//! tombstone and GroupMember add / remove / list. It delegates persistence to
//! `SqliteRepo` and never writes raw SQL itself. It MUST NOT reference
//! `command_log` or `provenance_events` — those are owned by `apply_command`.

#![allow(clippy::result_large_err)]

use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_store::model::{GroupMemberRecord, GroupRecord};
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;

use super::command::CommandResult;

type Result<T> = std::result::Result<T, ExError>;

// ---------------------------------------------------------------------------
// handle_group_create
// ---------------------------------------------------------------------------

/// Create a new group.
///
/// Invariants enforced:
/// - Name must be non-empty and non-whitespace-only (`InvalidTitle`).
pub fn handle_group_create(conn: &mut Connection, name: String) -> Result<CommandResult> {
    // Name validation
    if name.trim().is_empty() {
        return Err(ExError::new(ExErrorKind::InvalidTitle)
            .with_op("group_create")
            .with_message("group name must not be empty or whitespace-only"));
    }

    let id = format!("grp:{}", uuid::Uuid::now_v7());
    let now = chrono::Utc::now().to_rfc3339();

    let record = GroupRecord {
        id: id.clone(),
        name,
        created_at: now,
        tombstoned_at: None,
    };

    SqliteRepo::insert_group(conn, &record)?;

    Ok(CommandResult::GroupCreate { group_id: id })
}

// ---------------------------------------------------------------------------
// handle_group_get
// ---------------------------------------------------------------------------

/// Get a group by ID.
///
/// Returns `NotFound` if not found (returns even if tombstoned).
pub fn handle_group_get(conn: &Connection, group_id: String) -> Result<CommandResult> {
    let record = SqliteRepo::get_group(conn, &group_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("group_get")
            .with_entity_id(&group_id)
            .with_message(format!("Group not found: {}", group_id))
    })?;
    Ok(CommandResult::GroupGet { record })
}

// ---------------------------------------------------------------------------
// handle_group_list
// ---------------------------------------------------------------------------

/// List groups in deterministic order (created_at ASC, id ASC).
pub fn handle_group_list(conn: &Connection, include_tombstoned: bool) -> Result<CommandResult> {
    let items = SqliteRepo::list_groups(conn, include_tombstoned)?;
    Ok(CommandResult::GroupList { items })
}

// ---------------------------------------------------------------------------
// handle_group_tombstone
// ---------------------------------------------------------------------------

/// Tombstone a group (soft delete).
///
/// Invariants enforced:
/// - Group must exist (`NotFound`).
/// - Group must not already be tombstoned (`AlreadyTombstoned`).
/// - Group must have no active members (`HasActiveDependants`).
pub fn handle_group_tombstone(conn: &mut Connection, group_id: String) -> Result<CommandResult> {
    let existing = SqliteRepo::get_group(conn, &group_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("group_tombstone")
            .with_entity_id(&group_id)
            .with_message(format!("Group not found: {}", group_id))
    })?;

    if existing.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("group_tombstone")
            .with_entity_id(&group_id)
            .with_message(format!("Group is already tombstoned: {}", group_id)));
    }

    let active_members = SqliteRepo::count_active_group_members(conn, &group_id)?;
    if active_members > 0 {
        return Err(ExError::new(ExErrorKind::HasActiveDependants)
            .with_op("group_tombstone")
            .with_entity_id(&group_id)
            .with_message(format!(
                "Group has {} active member(s) that must be removed first",
                active_members
            )));
    }

    let now = chrono::Utc::now().to_rfc3339();
    SqliteRepo::tombstone_group(conn, &group_id, &now)?;

    Ok(CommandResult::GroupTombstone)
}

// ---------------------------------------------------------------------------
// handle_group_member_add
// ---------------------------------------------------------------------------

/// Add an ettle to a group.
///
/// Invariants enforced:
/// - Group must exist and not be tombstoned.
/// - Ettle must exist and not be tombstoned.
/// - No active duplicate membership (`DuplicateMapping`).
pub fn handle_group_member_add(
    conn: &mut Connection,
    group_id: String,
    ettle_id: String,
) -> Result<CommandResult> {
    // Validate group
    let group = SqliteRepo::get_group(conn, &group_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("group_member_add")
            .with_entity_id(&group_id)
            .with_message(format!("Group not found: {}", group_id))
    })?;
    if group.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("group_member_add")
            .with_entity_id(&group_id)
            .with_message(format!("Group is tombstoned: {}", group_id)));
    }

    // Validate ettle
    let ettle = SqliteRepo::get_ettle_record(conn, &ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("group_member_add")
            .with_entity_id(&ettle_id)
            .with_message(format!("Ettle not found: {}", ettle_id))
    })?;
    if ettle.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("group_member_add")
            .with_entity_id(&ettle_id)
            .with_message(format!("Ettle is tombstoned: {}", ettle_id)));
    }

    // Check for duplicate active membership
    if SqliteRepo::get_active_group_member(conn, &group_id, &ettle_id)?.is_some() {
        return Err(ExError::new(ExErrorKind::DuplicateMapping)
            .with_op("group_member_add")
            .with_entity_id(&ettle_id)
            .with_message(format!(
                "Ettle {} is already an active member of group {}",
                ettle_id, group_id
            )));
    }

    let id = format!("grpm:{}", uuid::Uuid::now_v7());
    let now = chrono::Utc::now().to_rfc3339();

    let record = GroupMemberRecord {
        id,
        group_id,
        ettle_id,
        created_at: now,
        tombstoned_at: None,
    };

    SqliteRepo::insert_group_member(conn, &record)?;

    Ok(CommandResult::GroupMemberAdd)
}

// ---------------------------------------------------------------------------
// handle_group_member_remove
// ---------------------------------------------------------------------------

/// Remove an ettle from a group (tombstones the membership record).
///
/// Invariants enforced:
/// - Active membership must exist (`NotFound`).
/// - The membership must not already be tombstoned (`AlreadyTombstoned`).
pub fn handle_group_member_remove(
    conn: &mut Connection,
    group_id: String,
    ettle_id: String,
) -> Result<CommandResult> {
    // Find the active membership
    let member =
        SqliteRepo::get_active_group_member(conn, &group_id, &ettle_id)?.ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_op("group_member_remove")
                .with_entity_id(&ettle_id)
                .with_message(format!(
                    "No active membership of ettle {} in group {}",
                    ettle_id, group_id
                ))
        })?;

    // The get_active_group_member query filters by tombstoned_at IS NULL,
    // so if we found a record, it's not tombstoned. But be defensive:
    if member.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("group_member_remove")
            .with_entity_id(&member.id)
            .with_message("Group membership is already tombstoned"));
    }

    let now = chrono::Utc::now().to_rfc3339();
    SqliteRepo::tombstone_group_member(conn, &member.id, &now)?;

    Ok(CommandResult::GroupMemberRemove)
}

// ---------------------------------------------------------------------------
// handle_group_member_list
// ---------------------------------------------------------------------------

/// List members of a group in deterministic order (created_at ASC, id ASC).
pub fn handle_group_member_list(
    conn: &Connection,
    group_id: String,
    include_tombstoned: bool,
) -> Result<CommandResult> {
    let items = SqliteRepo::list_group_members(conn, &group_id, include_tombstoned)?;
    Ok(CommandResult::GroupMemberList { items })
}
