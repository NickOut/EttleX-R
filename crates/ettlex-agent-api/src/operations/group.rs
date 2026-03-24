//! Agent API operations for Group CRUD and Group Member management.
//!
//! All read operations delegate to `SqliteRepo` (via `ettlex_memory::SqliteRepo`).
//! All write operations route through `ettlex_memory::apply_command`.
//!
//! ## Filter requirement
//!
//! `agent_group_member_list` requires at least one of `group_id` or `ettle_id` to be
//! specified.  Calling with both `None` returns `ExErrorKind::InvalidInput`.

#![allow(clippy::result_large_err)]

use ettlex_memory::{
    apply_command, ApprovalRouter, Command, CommandResult, Connection, ExError, ExErrorKind,
    FsStore, GroupMemberRecord, GroupRecord, PolicyProvider, SqliteRepo,
};
use ettlex_memory::{log_op_end, log_op_error, log_op_start};

/// Result of a successful `agent_group_create` call.
#[derive(Debug, Clone)]
pub struct AgentGroupCreateResult {
    /// The auto-generated Group ID (e.g. `"grp:019cf…"`).
    pub group_id: String,
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Result of a successful `agent_group_member_add` call.
#[derive(Debug, Clone)]
pub struct AgentGroupMemberAddResult {
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Result of a successful `agent_group_member_remove` call.
#[derive(Debug, Clone)]
pub struct AgentGroupMemberRemoveResult {
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Filter options for `agent_group_member_list`.
#[derive(Debug, Clone, Default)]
pub struct AgentGroupMemberListOpts {
    pub group_id: Option<String>,
    pub ettle_id: Option<String>,
    pub include_tombstoned: bool,
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// Fetch a single Group record by ID.
///
/// Returns `ExErrorKind::NotFound` if the ID does not exist.
pub fn agent_group_get(conn: &Connection, group_id: &str) -> Result<GroupRecord, ExError> {
    log_op_start!("agent_group_get", group_id = group_id);
    let start = std::time::Instant::now();
    let result = _agent_group_get_inner(conn, group_id);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_get", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_get", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_group_get_inner(conn: &Connection, group_id: &str) -> Result<GroupRecord, ExError> {
    SqliteRepo::get_group(conn, group_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("agent_group_get")
            .with_entity_id(group_id)
            .with_message(format!("Group not found: {group_id}"))
    })
}

/// List Groups.
///
/// Results are ordered `created_at ASC, id ASC` (deterministic).
pub fn agent_group_list(
    conn: &Connection,
    include_tombstoned: bool,
) -> Result<Vec<GroupRecord>, ExError> {
    log_op_start!("agent_group_list");
    let start = std::time::Instant::now();
    let result = SqliteRepo::list_groups(conn, include_tombstoned);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_list", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_list", e_clone, duration_ms = elapsed);
        }
    }
    result
}

/// List Group Members matching the given filter.
///
/// At least one of `group_id` or `ettle_id` must be set.
/// Returns `ExErrorKind::InvalidInput` if both are `None`.
pub fn agent_group_member_list(
    conn: &Connection,
    opts: &AgentGroupMemberListOpts,
) -> Result<Vec<GroupMemberRecord>, ExError> {
    log_op_start!("agent_group_member_list");
    let start = std::time::Instant::now();
    let result = _agent_group_member_list_inner(conn, opts);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_member_list", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_member_list", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_group_member_list_inner(
    conn: &Connection,
    opts: &AgentGroupMemberListOpts,
) -> Result<Vec<GroupMemberRecord>, ExError> {
    if opts.group_id.is_none() && opts.ettle_id.is_none() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_group_member_list")
            .with_message("at least one of group_id or ettle_id must be specified"));
    }
    SqliteRepo::list_group_members_by_filter(
        conn,
        opts.group_id.as_deref(),
        opts.ettle_id.as_deref(),
        opts.include_tombstoned,
    )
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/// Create a new Group.
///
/// `name` must be non-empty.
pub fn agent_group_create(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    name: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupCreateResult, ExError> {
    log_op_start!("agent_group_create");
    let start = std::time::Instant::now();
    let result = _agent_group_create_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        name,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_create", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_create", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_group_create_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    name: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupCreateResult, ExError> {
    let engine_cmd = Command::GroupCreate {
        name: name.to_string(),
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
        CommandResult::GroupCreate { group_id } => Ok(AgentGroupCreateResult {
            group_id,
            new_state_version,
        }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_group_create")
            .with_message("unexpected command result")),
    }
}

/// Add an Ettle to a Group.
///
/// Returns `ExErrorKind::AlreadyTombstoned` if the group is tombstoned.
/// Returns `ExErrorKind::ConstraintViolation` (or `DuplicateMapping`) if the ettle is already
/// an active member.
pub fn agent_group_member_add(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    group_id: &str,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupMemberAddResult, ExError> {
    log_op_start!("agent_group_member_add", group_id = group_id);
    let start = std::time::Instant::now();
    let result = _agent_group_member_add_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        group_id,
        ettle_id,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_member_add", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_member_add", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_group_member_add_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    group_id: &str,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupMemberAddResult, ExError> {
    let engine_cmd = Command::GroupMemberAdd {
        group_id: group_id.to_string(),
        ettle_id: ettle_id.to_string(),
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
        CommandResult::GroupMemberAdd => Ok(AgentGroupMemberAddResult { new_state_version }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_group_member_add")
            .with_message("unexpected command result")),
    }
}

/// Remove an Ettle from a Group (tombstones the membership record).
///
/// Returns `ExErrorKind::NotFound` if the membership record does not exist.
pub fn agent_group_member_remove(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    group_id: &str,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupMemberRemoveResult, ExError> {
    log_op_start!("agent_group_member_remove", group_id = group_id);
    let start = std::time::Instant::now();
    let result = _agent_group_member_remove_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        group_id,
        ettle_id,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_group_member_remove", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_group_member_remove", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_group_member_remove_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    group_id: &str,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentGroupMemberRemoveResult, ExError> {
    let engine_cmd = Command::GroupMemberRemove {
        group_id: group_id.to_string(),
        ettle_id: ettle_id.to_string(),
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
        CommandResult::GroupMemberRemove => Ok(AgentGroupMemberRemoveResult { new_state_version }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_group_member_remove")
            .with_message("unexpected command result")),
    }
}
