//! Agent API operations for Relation CRUD.
//!
//! All read operations delegate to `SqliteRepo` (via `ettlex_memory::SqliteRepo`).
//! All write operations route through `ettlex_memory::apply_command`.
//!
//! ## Filter requirement
//!
//! `agent_relation_list` requires at least one of `source_ettle_id`, `target_ettle_id`,
//! or `relation_type` to be specified.  Calling with all three `None` returns
//! `ExErrorKind::InvalidInput`.

#![allow(clippy::result_large_err)]

use ettlex_memory::{
    apply_command, ApprovalRouter, Command, CommandResult, Connection, ExError, ExErrorKind,
    FsStore, PolicyProvider, RelationListOpts, RelationRecord, SqliteRepo,
};
use ettlex_memory::{log_op_end, log_op_error, log_op_start};

/// Result of a successful `agent_relation_create` call.
#[derive(Debug, Clone)]
pub struct AgentRelationCreateResult {
    /// The auto-generated Relation ID (e.g. `"rel:019cf…"`).
    pub relation_id: String,
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Input for `agent_relation_create`.
#[derive(Debug, Clone, Default)]
pub struct AgentRelationCreate {
    pub source_ettle_id: String,
    pub target_ettle_id: String,
    pub relation_type: String,
    pub properties_json: Option<serde_json::Value>,
    /// Must be `None`.  If supplied, `agent_relation_create` returns `InvalidInput`.
    pub relation_id: Option<String>,
}

/// Result of a successful `agent_relation_tombstone` call.
#[derive(Debug, Clone)]
pub struct AgentRelationTombstoneResult {
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Filter options for `agent_relation_list`.
#[derive(Debug, Clone, Default)]
pub struct AgentRelationListOpts {
    pub source_ettle_id: Option<String>,
    pub target_ettle_id: Option<String>,
    pub relation_type: Option<String>,
    pub include_tombstoned: bool,
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// Fetch a single Relation record by ID.
///
/// Returns `ExErrorKind::NotFound` if the ID does not exist.
pub fn agent_relation_get(conn: &Connection, relation_id: &str) -> Result<RelationRecord, ExError> {
    log_op_start!("agent_relation_get", relation_id = relation_id);
    let start = std::time::Instant::now();
    let result = _agent_relation_get_inner(conn, relation_id);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_relation_get", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_relation_get", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_relation_get_inner(
    conn: &Connection,
    relation_id: &str,
) -> Result<RelationRecord, ExError> {
    SqliteRepo::get_relation(conn, relation_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("agent_relation_get")
            .with_entity_id(relation_id)
            .with_message(format!("Relation not found: {relation_id}"))
    })
}

/// List Relations matching the given filter.
///
/// At least one of `source_ettle_id`, `target_ettle_id`, or `relation_type` must be set.
/// Returns `ExErrorKind::InvalidInput` if all three are `None`.
pub fn agent_relation_list(
    conn: &Connection,
    opts: &AgentRelationListOpts,
) -> Result<Vec<RelationRecord>, ExError> {
    log_op_start!("agent_relation_list");
    let start = std::time::Instant::now();
    let result = _agent_relation_list_inner(conn, opts);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_relation_list", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_relation_list", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_relation_list_inner(
    conn: &Connection,
    opts: &AgentRelationListOpts,
) -> Result<Vec<RelationRecord>, ExError> {
    if opts.source_ettle_id.is_none()
        && opts.target_ettle_id.is_none()
        && opts.relation_type.is_none()
    {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_relation_list")
            .with_message("at least one of source_ettle_id, target_ettle_id, or relation_type must be specified"));
    }
    let store_opts = RelationListOpts {
        source_ettle_id: opts.source_ettle_id.clone(),
        target_ettle_id: opts.target_ettle_id.clone(),
        relation_type: opts.relation_type.clone(),
        include_tombstoned: opts.include_tombstoned,
    };
    SqliteRepo::list_relations(conn, &store_opts)
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/// Create a new Relation between two Ettles.
///
/// `cmd.relation_id` MUST be `None`; if supplied, returns `ExErrorKind::InvalidInput`.
pub fn agent_relation_create(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentRelationCreate,
    expected_state_version: Option<u64>,
) -> Result<AgentRelationCreateResult, ExError> {
    log_op_start!("agent_relation_create");
    let start = std::time::Instant::now();
    let result = _agent_relation_create_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        cmd,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_relation_create", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_relation_create", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_relation_create_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentRelationCreate,
    expected_state_version: Option<u64>,
) -> Result<AgentRelationCreateResult, ExError> {
    if cmd.relation_id.is_some() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_relation_create")
            .with_message("relation_id must not be supplied by caller"));
    }
    let engine_cmd = Command::RelationCreate {
        source_ettle_id: cmd.source_ettle_id,
        target_ettle_id: cmd.target_ettle_id,
        relation_type: cmd.relation_type,
        properties_json: cmd.properties_json,
        relation_id: None,
    };
    let (result, new_state_version) = apply_command(
        engine_cmd,
        expected_state_version,
        conn,
        cas,
        policy_provider,
        approval_router,
    )?;
    match result {
        CommandResult::RelationCreate { relation_id } => Ok(AgentRelationCreateResult {
            relation_id,
            new_state_version,
        }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_relation_create")
            .with_message("unexpected command result")),
    }
}

/// Tombstone (soft-delete) a Relation.
///
/// Returns `ExErrorKind::NotFound` if the relation does not exist.
/// Returns `ExErrorKind::AlreadyTombstoned` if already tombstoned.
pub fn agent_relation_tombstone(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    relation_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentRelationTombstoneResult, ExError> {
    log_op_start!("agent_relation_tombstone", relation_id = relation_id);
    let start = std::time::Instant::now();
    let result = _agent_relation_tombstone_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        relation_id,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_relation_tombstone", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_relation_tombstone", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_relation_tombstone_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    relation_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentRelationTombstoneResult, ExError> {
    let engine_cmd = Command::RelationTombstone {
        relation_id: relation_id.to_string(),
    };
    let (result, new_state_version) = apply_command(
        engine_cmd,
        expected_state_version,
        conn,
        cas,
        policy_provider,
        approval_router,
    )?;
    match result {
        CommandResult::RelationTombstone => Ok(AgentRelationTombstoneResult { new_state_version }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_relation_tombstone")
            .with_message("unexpected command result")),
    }
}
