//! Engine handler for Relation CRUD operations — Slice 02.
//!
//! This module owns all invariant enforcement for Relation create / update /
//! get / list / tombstone. It delegates persistence to `SqliteRepo` and
//! never writes raw SQL itself. It MUST NOT reference `command_log` or
//! `provenance_events` tables — those are owned by `apply_command`.

#![allow(clippy::result_large_err)]

use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_store::model::{RelationListOpts, RelationRecord};
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use serde_json::Value as JsonValue;

use super::command::CommandResult;

type Result<T> = std::result::Result<T, ExError>;

// ---------------------------------------------------------------------------
// handle_relation_create
// ---------------------------------------------------------------------------

/// Create a new relation between two ettles.
///
/// Invariants enforced:
/// - `relation_id` must be None — caller-supplied IDs rejected (`InvalidInput`).
/// - `relation_type` must exist in registry and not be tombstoned (`InvalidInput`).
/// - `source_ettle_id` must exist and not be tombstoned (`NotFound` / `AlreadyTombstoned`).
/// - `target_ettle_id` must exist and not be tombstoned (`NotFound` / `AlreadyTombstoned`).
/// - source must not equal target (`SelfReferentialLink`).
/// - If `cycle_check = true` in the registry entry, no cycle may be created (`CycleDetected`).
pub fn handle_relation_create(
    conn: &mut Connection,
    source_ettle_id: String,
    target_ettle_id: String,
    relation_type: String,
    properties_json: Option<JsonValue>,
    relation_id: Option<String>,
) -> Result<CommandResult> {
    // Reject caller-supplied relation_id
    if relation_id.is_some() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("relation_create")
            .with_message("relation_id must not be supplied; it is auto-generated"));
    }

    // Self-referential check
    if source_ettle_id == target_ettle_id {
        return Err(ExError::new(ExErrorKind::SelfReferentialLink)
            .with_op("relation_create")
            .with_entity_id(&source_ettle_id)
            .with_message("source and target ettle must be different"));
    }

    // Validate relation_type in registry
    let type_entry =
        SqliteRepo::get_relation_type_entry(conn, &relation_type)?.ok_or_else(|| {
            ExError::new(ExErrorKind::InvalidInput)
                .with_op("relation_create")
                .with_message(format!("Unknown relation type: {}", relation_type))
        })?;

    if type_entry.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("relation_create")
            .with_message(format!("Relation type is tombstoned: {}", relation_type)));
    }

    // Validate source ettle
    let source = SqliteRepo::get_ettle_record(conn, &source_ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("relation_create")
            .with_entity_id(&source_ettle_id)
            .with_message(format!("Source ettle not found: {}", source_ettle_id))
    })?;
    if source.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("relation_create")
            .with_entity_id(&source_ettle_id)
            .with_message(format!("Source ettle is tombstoned: {}", source_ettle_id)));
    }

    // Validate target ettle
    let target = SqliteRepo::get_ettle_record(conn, &target_ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("relation_create")
            .with_entity_id(&target_ettle_id)
            .with_message(format!("Target ettle not found: {}", target_ettle_id))
    })?;
    if target.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("relation_create")
            .with_entity_id(&target_ettle_id)
            .with_message(format!("Target ettle is tombstoned: {}", target_ettle_id)));
    }

    // Cycle detection (only when registry entry has cycle_check = true)
    let cycle_check = is_cycle_check_enabled(&type_entry.properties_json);
    if cycle_check {
        // Check if target can reach source via constraint relations (would form a cycle)
        if would_create_cycle(conn, &source_ettle_id, &target_ettle_id, &relation_type)? {
            return Err(ExError::new(ExErrorKind::CycleDetected)
                .with_op("relation_create")
                .with_entity_id(&source_ettle_id)
                .with_message("Creating this relation would introduce a cycle"));
        }
    }

    // Generate ID with rel: prefix
    let id = format!("rel:{}", uuid::Uuid::now_v7());
    let now = chrono::Utc::now().to_rfc3339();
    let props = match properties_json {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        None => "{}".to_string(),
    };

    let record = RelationRecord {
        id: id.clone(),
        source_ettle_id,
        target_ettle_id,
        relation_type,
        properties_json: props,
        created_at: now,
        tombstoned_at: None,
    };

    SqliteRepo::insert_relation(conn, &record)?;

    Ok(CommandResult::RelationCreate { relation_id: id })
}

// ---------------------------------------------------------------------------
// Cycle detection helper
// ---------------------------------------------------------------------------

/// Returns true if adding source→target would create a cycle.
///
/// A cycle exists if target can already reach source via the same relation type.
/// Uses BFS from target, following outgoing relations of the given type.
fn would_create_cycle(
    conn: &Connection,
    source_ettle_id: &str,
    target_ettle_id: &str,
    relation_type: &str,
) -> Result<bool> {
    use std::collections::{HashSet, VecDeque};

    let mut visited: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();
    queue.push_back(target_ettle_id.to_string());

    while let Some(current) = queue.pop_front() {
        if current == source_ettle_id {
            return Ok(true);
        }
        if visited.contains(&current) {
            continue;
        }
        visited.insert(current.clone());

        let next_targets =
            SqliteRepo::get_active_outgoing_relations_of_type(conn, &current, relation_type)?;
        for next in next_targets {
            if !visited.contains(&next) {
                queue.push_back(next);
            }
        }
    }

    Ok(false)
}

/// Parse the cycle_check flag from the registry entry's properties_json.
fn is_cycle_check_enabled(properties_json: &str) -> bool {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(properties_json) {
        if let Some(b) = val.get("cycle_check").and_then(|v| v.as_bool()) {
            return b;
        }
    }
    false
}

// ---------------------------------------------------------------------------
// handle_relation_update
// ---------------------------------------------------------------------------

/// Update a relation's properties_json.
///
/// Invariants enforced:
/// - Relation must exist (`NotFound`).
/// - Relation must not be tombstoned (`AlreadyTombstoned`).
/// - `properties_json` must be `Some(...)` — `None` is rejected (`EmptyUpdate`).
pub fn handle_relation_update(
    conn: &mut Connection,
    relation_id: String,
    properties_json: Option<JsonValue>,
) -> Result<CommandResult> {
    // EmptyUpdate: at least one field required
    if properties_json.is_none() {
        return Err(ExError::new(ExErrorKind::EmptyUpdate)
            .with_op("relation_update")
            .with_entity_id(&relation_id)
            .with_message("RelationUpdate requires at least one field (properties_json)"));
    }

    // Fetch existing record
    let existing = SqliteRepo::get_relation(conn, &relation_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("relation_update")
            .with_entity_id(&relation_id)
            .with_message(format!("Relation not found: {}", relation_id))
    })?;

    if existing.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("relation_update")
            .with_entity_id(&relation_id)
            .with_message(format!("Relation is tombstoned: {}", relation_id)));
    }

    let props_str = match properties_json {
        Some(v) => serde_json::to_string(&v).unwrap_or_else(|_| "{}".to_string()),
        None => unreachable!("checked above"),
    };

    SqliteRepo::update_relation_properties(conn, &relation_id, &props_str)?;

    Ok(CommandResult::RelationUpdate)
}

// ---------------------------------------------------------------------------
// handle_relation_get
// ---------------------------------------------------------------------------

/// Get a relation by ID.
///
/// Returns `NotFound` if the relation does not exist.
/// Returns the record even if tombstoned.
pub fn handle_relation_get(conn: &Connection, relation_id: String) -> Result<CommandResult> {
    let record = SqliteRepo::get_relation(conn, &relation_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("relation_get")
            .with_entity_id(&relation_id)
            .with_message(format!("Relation not found: {}", relation_id))
    })?;
    Ok(CommandResult::RelationGet { record })
}

// ---------------------------------------------------------------------------
// handle_relation_list
// ---------------------------------------------------------------------------

/// List relations with optional filters.
///
/// Invariants enforced:
/// - At least one of `source_ettle_id`, `target_ettle_id`, or `relation_type`
///   must be supplied (`InvalidInput`).
/// - Results are sorted by (created_at ASC, id ASC).
pub fn handle_relation_list(
    conn: &Connection,
    source_ettle_id: Option<String>,
    target_ettle_id: Option<String>,
    relation_type: Option<String>,
    include_tombstoned: bool,
) -> Result<CommandResult> {
    // Require at least one filter
    if source_ettle_id.is_none() && target_ettle_id.is_none() && relation_type.is_none() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("relation_list")
            .with_message(
                "At least one filter (source_ettle_id, target_ettle_id, or relation_type) \
                 must be supplied",
            ));
    }

    let opts = RelationListOpts {
        source_ettle_id,
        target_ettle_id,
        relation_type,
        include_tombstoned,
    };

    let items = SqliteRepo::list_relations(conn, &opts)?;
    Ok(CommandResult::RelationList { items })
}

// ---------------------------------------------------------------------------
// handle_relation_tombstone
// ---------------------------------------------------------------------------

/// Tombstone a relation (soft delete).
///
/// Invariants enforced:
/// - Relation must exist (`NotFound`).
/// - Relation must not already be tombstoned (`AlreadyTombstoned`).
pub fn handle_relation_tombstone(
    conn: &mut Connection,
    relation_id: String,
) -> Result<CommandResult> {
    let existing = SqliteRepo::get_relation(conn, &relation_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("relation_tombstone")
            .with_entity_id(&relation_id)
            .with_message(format!("Relation not found: {}", relation_id))
    })?;

    if existing.tombstoned_at.is_some() {
        return Err(ExError::new(ExErrorKind::AlreadyTombstoned)
            .with_op("relation_tombstone")
            .with_entity_id(&relation_id)
            .with_message(format!("Relation is already tombstoned: {}", relation_id)));
    }

    let now = chrono::Utc::now().to_rfc3339();
    SqliteRepo::tombstone_relation(conn, &relation_id, &now)?;

    Ok(CommandResult::RelationTombstone)
}
