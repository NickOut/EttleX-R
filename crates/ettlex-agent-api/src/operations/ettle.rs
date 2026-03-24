//! Agent API operations for Ettle CRUD.
//!
//! All read operations delegate to `SqliteRepo` (via `ettlex_memory::SqliteRepo`).
//! All write operations route through `ettlex_memory::apply_command`.
//!
//! ## Lifecycle logging
//!
//! Each public function emits exactly one `start` event and one `end`/`end_error`
//! event using `log_op_start!` / `log_op_end!` / `log_op_error!`.  WHY / WHAT / HOW
//! content MUST NOT appear as log field values.
//!
//! ## OCC
//!
//! Write functions accept `expected_state_version: Option<u64>`.  Pass `None` to
//! skip the OCC check, or `Some(v)` to assert that the current state version equals
//! `v` before executing the command.
//!
//! ## Cursor encoding
//!
//! `agent_ettle_list` accepts an opaque `cursor` string (base64 URL-safe-no-pad
//! encoded `"{created_at},{id}"`).  The cursor is decoded and forwarded to
//! `SqliteRepo::list_ettles`.  Encoding is handled by the store layer.

#![allow(clippy::result_large_err)]

use ettlex_memory::{
    apply_command, ApprovalRouter, Command, CommandResult, Connection, EttleContext, EttleCursor,
    EttleListOpts, EttleListPage, EttleRecord, ExError, ExErrorKind, FsStore, PolicyProvider,
    SqliteRepo,
};
use ettlex_memory::{log_op_end, log_op_error, log_op_start};

use crate::memory_manager_instance;

/// Result of a successful `agent_ettle_create` call.
#[derive(Debug, Clone)]
pub struct AgentEttleCreateResult {
    /// The auto-generated Ettle ID (e.g. `"ettle:019cf…"`).
    pub ettle_id: String,
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Input for `agent_ettle_create`.
#[derive(Debug, Clone, Default)]
pub struct AgentEttleCreate {
    pub title: String,
    /// Must be `None`.  If supplied, `agent_ettle_create` returns `InvalidInput`.
    pub ettle_id: Option<String>,
    pub why: Option<String>,
    pub what: Option<String>,
    pub how: Option<String>,
    pub reasoning_link_id: Option<String>,
    pub reasoning_link_type: Option<String>,
}

/// Result of a successful `agent_ettle_update` call.
#[derive(Debug, Clone)]
pub struct AgentEttleUpdateResult {
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Input for `agent_ettle_update`.
#[derive(Debug, Clone, Default)]
pub struct AgentEttleUpdate {
    pub ettle_id: String,
    pub title: Option<String>,
    pub why: Option<String>,
    pub what: Option<String>,
    pub how: Option<String>,
    /// - `None` → do not update
    /// - `Some(None)` → clear the link
    /// - `Some(Some(id))` → set the link
    pub reasoning_link_id: Option<Option<String>>,
    pub reasoning_link_type: Option<Option<String>>,
}

/// Result of a successful `agent_ettle_tombstone` call.
#[derive(Debug, Clone)]
pub struct AgentEttleTombstoneResult {
    /// The new state version after the command was applied.
    pub new_state_version: u64,
}

/// Options for `agent_ettle_list`.
#[derive(Debug, Clone)]
pub struct AgentEttleListOpts {
    /// Number of items per page. Must be > 0.
    pub limit: u32,
    /// Opaque base64 cursor from a previous `agent_ettle_list` response.
    pub cursor: Option<String>,
    pub include_tombstoned: bool,
}

// ---------------------------------------------------------------------------
// Read operations
// ---------------------------------------------------------------------------

/// Fetch a single Ettle record by ID (including tombstoned).
///
/// Returns `ExErrorKind::NotFound` if the ID does not exist.
pub fn agent_ettle_get(conn: &Connection, ettle_id: &str) -> Result<EttleRecord, ExError> {
    log_op_start!("agent_ettle_get", ettle_id = ettle_id);
    let start = std::time::Instant::now();
    let result = _agent_ettle_get_inner(conn, ettle_id);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_get", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_get", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_ettle_get_inner(conn: &Connection, ettle_id: &str) -> Result<EttleRecord, ExError> {
    SqliteRepo::get_ettle_record(conn, ettle_id)?.ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("agent_ettle_get")
            .with_entity_id(ettle_id)
            .with_message(format!("Ettle not found: {ettle_id}"))
    })
}

/// Assemble a rich EttleContext for the given Ettle ID.
///
/// Returns WHY/WHAT/HOW fields along with active relations and active group memberships.
/// Returns `ExErrorKind::NotFound` if the Ettle does not exist.
pub fn agent_ettle_context(conn: &Connection, ettle_id: &str) -> Result<EttleContext, ExError> {
    log_op_start!("agent_ettle_context", ettle_id = ettle_id);
    let start = std::time::Instant::now();
    let mm = memory_manager_instance();
    let result = mm.assemble_ettle_context(ettle_id, conn);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_context", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_context", e_clone, duration_ms = elapsed);
        }
    }
    result
}

/// List Ettles with cursor-based pagination.
///
/// Returns `ExErrorKind::InvalidInput` if `opts.limit == 0`.
pub fn agent_ettle_list(
    conn: &Connection,
    opts: &AgentEttleListOpts,
) -> Result<EttleListPage, ExError> {
    log_op_start!("agent_ettle_list");
    let start = std::time::Instant::now();
    let result = _agent_ettle_list_inner(conn, opts);
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_list", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_list", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_ettle_list_inner(
    conn: &Connection,
    opts: &AgentEttleListOpts,
) -> Result<EttleListPage, ExError> {
    if opts.limit == 0 {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_ettle_list")
            .with_message("limit must be > 0"));
    }
    // Decode opaque cursor string into EttleCursor
    let cursor = opts.cursor.as_deref().map(decode_cursor).transpose()?;

    let store_opts = EttleListOpts {
        limit: opts.limit,
        cursor,
        include_tombstoned: opts.include_tombstoned,
    };
    SqliteRepo::list_ettles(conn, &store_opts)
}

/// Decode a base64 cursor string into an EttleCursor.
fn decode_cursor(s: &str) -> Result<EttleCursor, ExError> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(s)
        .map_err(|_| {
            ExError::new(ExErrorKind::InvalidInput)
                .with_op("agent_ettle_list")
                .with_message("invalid cursor encoding")
        })?;
    let decoded = String::from_utf8(bytes).map_err(|_| {
        ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_ettle_list")
            .with_message("cursor is not valid UTF-8")
    })?;
    // Format: "{created_at},{id}"
    let comma = decoded.find(',').ok_or_else(|| {
        ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_ettle_list")
            .with_message("cursor has invalid format")
    })?;
    Ok(EttleCursor {
        created_at: decoded[..comma].to_string(),
        id: decoded[comma + 1..].to_string(),
    })
}

// ---------------------------------------------------------------------------
// Write operations
// ---------------------------------------------------------------------------

/// Create a new Ettle.
///
/// `cmd.ettle_id` MUST be `None`; if supplied, returns `ExErrorKind::InvalidInput`.
/// `cmd.title` must be non-empty.
/// If `cmd.reasoning_link_id` is set, `cmd.reasoning_link_type` must also be set.
pub fn agent_ettle_create(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentEttleCreate,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleCreateResult, ExError> {
    log_op_start!("agent_ettle_create");
    let start = std::time::Instant::now();
    let result = _agent_ettle_create_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        cmd,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_create", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_create", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_ettle_create_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentEttleCreate,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleCreateResult, ExError> {
    if cmd.ettle_id.is_some() {
        return Err(ExError::new(ExErrorKind::InvalidInput)
            .with_op("agent_ettle_create")
            .with_message("ettle_id must not be supplied by caller"));
    }
    let engine_cmd = Command::EttleCreate {
        title: cmd.title,
        ettle_id: None,
        why: cmd.why,
        what: cmd.what,
        how: cmd.how,
        reasoning_link_id: cmd.reasoning_link_id,
        reasoning_link_type: cmd.reasoning_link_type,
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
        CommandResult::EttleCreate { ettle_id } => Ok(AgentEttleCreateResult {
            ettle_id,
            new_state_version,
        }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_ettle_create")
            .with_message("unexpected command result")),
    }
}

/// Update an existing Ettle's content fields.
///
/// At least one field must be specified. Omitted fields are preserved.
/// Use `reasoning_link_id: Some(None)` to clear the link.
pub fn agent_ettle_update(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentEttleUpdate,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleUpdateResult, ExError> {
    log_op_start!("agent_ettle_update", ettle_id = cmd.ettle_id.as_str());
    let start = std::time::Instant::now();
    let result = _agent_ettle_update_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        cmd,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_update", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_update", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_ettle_update_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    cmd: AgentEttleUpdate,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleUpdateResult, ExError> {
    let engine_cmd = Command::EttleUpdate {
        ettle_id: cmd.ettle_id,
        title: cmd.title,
        why: cmd.why,
        what: cmd.what,
        how: cmd.how,
        reasoning_link_id: cmd.reasoning_link_id,
        reasoning_link_type: cmd.reasoning_link_type,
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
        CommandResult::EttleUpdate => Ok(AgentEttleUpdateResult { new_state_version }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_ettle_update")
            .with_message("unexpected command result")),
    }
}

/// Tombstone (soft-delete) an Ettle.
///
/// Returns `ExErrorKind::HasActiveDependants` if any active Ettles reference this one
/// as a reasoning link.
pub fn agent_ettle_tombstone(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleTombstoneResult, ExError> {
    log_op_start!("agent_ettle_tombstone", ettle_id = ettle_id);
    let start = std::time::Instant::now();
    let result = _agent_ettle_tombstone_inner(
        conn,
        cas,
        policy_provider,
        approval_router,
        ettle_id,
        expected_state_version,
    );
    let elapsed = start.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => log_op_end!("agent_ettle_tombstone", duration_ms = elapsed),
        Err(e) => {
            let e_clone = e.clone();
            log_op_error!("agent_ettle_tombstone", e_clone, duration_ms = elapsed);
        }
    }
    result
}

fn _agent_ettle_tombstone_inner(
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
    ettle_id: &str,
    expected_state_version: Option<u64>,
) -> Result<AgentEttleTombstoneResult, ExError> {
    let engine_cmd = Command::EttleTombstone {
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
        CommandResult::EttleTombstone => Ok(AgentEttleTombstoneResult { new_state_version }),
        _ => Err(ExError::new(ExErrorKind::Internal)
            .with_op("agent_ettle_tombstone")
            .with_message("unexpected command result")),
    }
}
