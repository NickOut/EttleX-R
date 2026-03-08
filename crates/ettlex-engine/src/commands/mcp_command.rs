//! MCP-layer command dispatch.
//!
//! `McpCommand` is the JSON-deserialisable command enum that arrives from the
//! MCP `ettlex.apply` tool.  `apply_mcp_command` dispatches to the appropriate
//! engine or store function, wraps the result in `McpCommandResult`, and
//! appends a row to `mcp_command_log` (optimistic-concurrency counter).

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::model::{Constraint, Ep, EpConstraintRef, Ettle};
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::commands::engine_command::{apply_engine_command, EngineCommand, EngineCommandResult};
use crate::commands::snapshot::SnapshotOptions;

// ---------------------------------------------------------------------------
// McpCommand — serialisable command vocabulary
// ---------------------------------------------------------------------------

/// All write operations available via the MCP `ettlex.apply` tool.
///
/// Serialised as a tagged JSON object: `{ "tag": "...", ...fields }`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "tag")]
pub enum McpCommand {
    // ── Snapshot ──────────────────────────────────────────────────────────────
    /// Commit a snapshot for a leaf EP.
    SnapshotCommit {
        leaf_ep_id: String,
        policy_ref: String,
        profile_ref: Option<String>,
        #[serde(default)]
        dry_run: bool,
        expected_head: Option<String>,
    },

    // ── Ettle ────────────────────────────────────────────────────────────────
    /// Create a new Ettle.
    EttleCreate { title: String },

    // ── EP ───────────────────────────────────────────────────────────────────
    /// Create a new EP.
    EpCreate {
        ettle_id: String,
        ordinal: u32,
        #[serde(default = "default_true")]
        normative: bool,
        #[serde(default)]
        why: String,
        #[serde(default)]
        what: String,
        #[serde(default)]
        how: String,
    },

    /// Update an existing EP's content fields.
    ///
    /// At least one field must be supplied; omitted fields are preserved.
    /// Increments `state_version` and sets `updated_at`.
    EpUpdate {
        /// EP to update
        ep_id: String,
        /// New WHY text (preserved if absent)
        why: Option<String>,
        /// New WHAT text (preserved if absent)
        what: Option<String>,
        /// New HOW text (preserved if absent)
        how: Option<String>,
        /// New display title (preserved if absent)
        title: Option<String>,
    },

    // ── Constraint ───────────────────────────────────────────────────────────
    /// Create a constraint.
    ConstraintCreate {
        constraint_id: String,
        family: String,
        kind: String,
        scope: String,
        payload_json: JsonValue,
    },
    /// Attach a constraint to an EP.
    ConstraintAttachToEp {
        ep_id: String,
        constraint_id: String,
        ordinal: i32,
    },

    // ── Profile ───────────────────────────────────────────────────────────────
    /// Create a profile.
    ProfileCreate {
        profile_ref: String,
        payload_json: JsonValue,
        source: Option<String>,
    },
    /// Set the repository default profile.
    ProfileSetDefault { profile_ref: String },
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// McpCommandResult
// ---------------------------------------------------------------------------

/// Result of a successful `apply_mcp_command` call.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "tag")]
pub enum McpCommandResult {
    SnapshotCommit {
        snapshot_id: String,
        manifest_digest: String,
    },
    RoutedForApproval {
        approval_token: String,
    },
    EttleCreate {
        ettle_id: String,
    },
    EpCreate {
        ep_id: String,
    },
    EpUpdate {
        ep_id: String,
    },
    ConstraintCreate {
        constraint_id: String,
    },
    ConstraintAttachToEp,
    ProfileCreate,
    ProfileSetDefault,
}

// ---------------------------------------------------------------------------
// apply_mcp_command
// ---------------------------------------------------------------------------

/// Apply an MCP command, enforcing OCC via `mcp_command_log`.
///
/// Steps:
/// 1. Read current `state_version` (COUNT(*) from `mcp_command_log`).
/// 2. If `expected_state_version` is `Some(v)` and `v != current` → `HeadMismatch`.
/// 3. Execute the command.
/// 4. Insert a row into `mcp_command_log` → `new_state_version = current + 1`.
///
/// Returns `(McpCommandResult, new_state_version)`.
pub fn apply_mcp_command(
    cmd: McpCommand,
    expected_state_version: Option<u64>,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<(McpCommandResult, u64)> {
    // 1. Read state_version
    let current_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM mcp_command_log", [], |r| r.get(0))
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("apply_mcp_command")
                .with_message(format!("Failed to read mcp_command_log: {}", e))
        })?;

    // 2. OCC check
    if let Some(expected) = expected_state_version {
        if expected != current_sv {
            return Err(ExError::new(ExErrorKind::HeadMismatch)
                .with_op("apply_mcp_command")
                .with_message(format!(
                    "state_version mismatch: expected {} but current is {}",
                    expected, current_sv
                )));
        }
    }

    // 3. Dispatch
    let result = dispatch_mcp_command(cmd, conn, cas, policy_provider, approval_router)?;

    // 4. Insert log row
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO mcp_command_log (applied_at) VALUES (?1)",
        [now_ms],
    )
    .map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("apply_mcp_command")
            .with_message(format!("Failed to insert mcp_command_log row: {}", e))
    })?;

    Ok((result, current_sv + 1))
}

// ---------------------------------------------------------------------------
// Internal dispatch
// ---------------------------------------------------------------------------

fn dispatch_mcp_command(
    cmd: McpCommand,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<McpCommandResult> {
    match cmd {
        McpCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref,
            profile_ref,
            dry_run,
            expected_head,
        } => {
            let options = SnapshotOptions {
                expected_head,
                dry_run,
                allow_dedup: false,
            };
            let engine_cmd = EngineCommand::SnapshotCommit {
                leaf_ep_id,
                policy_ref,
                profile_ref,
                options,
            };
            let result =
                apply_engine_command(engine_cmd, conn, cas, policy_provider, approval_router)?;
            match result {
                EngineCommandResult::SnapshotCommit(r) => Ok(McpCommandResult::SnapshotCommit {
                    snapshot_id: r.snapshot_id,
                    manifest_digest: r.manifest_digest,
                }),
                EngineCommandResult::SnapshotCommitRouted(r) => {
                    Ok(McpCommandResult::RoutedForApproval {
                        approval_token: r.approval_token,
                    })
                }
                _ => Err(ExError::new(ExErrorKind::Internal)
                    .with_op("dispatch_mcp_command")
                    .with_message("Unexpected EngineCommandResult variant")),
            }
        }

        McpCommand::EttleCreate { title } => {
            let ettle_id = format!("ettle:{}", uuid::Uuid::now_v7());
            let ettle = Ettle::new(ettle_id.clone(), title);
            SqliteRepo::persist_ettle(conn, &ettle)?;
            Ok(McpCommandResult::EttleCreate { ettle_id })
        }

        McpCommand::EpCreate {
            ettle_id,
            ordinal,
            normative,
            why,
            what,
            how,
        } => {
            let ep_id = format!("ep:{}:{}", uuid::Uuid::now_v7(), ordinal);
            let ep = Ep::new(ep_id.clone(), ettle_id, ordinal, normative, why, what, how);
            SqliteRepo::persist_ep(conn, &ep)?;
            Ok(McpCommandResult::EpCreate { ep_id })
        }

        McpCommand::EpUpdate {
            ep_id,
            why,
            what,
            how,
            title,
        } => {
            // Reject empty update (at least one field must be supplied)
            if why.is_none() && what.is_none() && how.is_none() && title.is_none() {
                return Err(ExError::new(ExErrorKind::EmptyUpdate)
                    .with_ep_id(&ep_id)
                    .with_op("ep_update")
                    .with_message("EpUpdate requires at least one field"));
            }

            // Fetch existing EP
            let mut ep = SqliteRepo::get_ep(conn, &ep_id)?.ok_or_else(|| {
                ExError::new(ExErrorKind::NotFound)
                    .with_ep_id(&ep_id)
                    .with_op("ep_update")
                    .with_message(format!("EP not found: {}", ep_id))
            })?;

            // Apply supplied fields; omitted fields are preserved
            if let Some(new_why) = why {
                ep.why = new_why;
            }
            if let Some(new_what) = what {
                ep.what = new_what;
            }
            if let Some(new_how) = how {
                ep.how = new_how;
            }
            if let Some(new_title) = title {
                ep.title = Some(new_title);
            }

            // Recompute content digest and set updated_at
            ep.recompute_content_digest();
            ep.updated_at = chrono::Utc::now();

            SqliteRepo::persist_ep(conn, &ep)?;
            Ok(McpCommandResult::EpUpdate {
                ep_id: ep_id.clone(),
            })
        }

        McpCommand::ConstraintCreate {
            constraint_id,
            family,
            kind,
            scope,
            payload_json,
        } => {
            // Validate family is non-empty
            if family.trim().is_empty() {
                return Err(ExError::new(ExErrorKind::InvalidConstraintFamily)
                    .with_entity_id(&constraint_id)
                    .with_message("Constraint family cannot be empty"));
            }

            // Check for duplicate
            if let Some(_existing) = SqliteRepo::get_constraint(conn, &constraint_id)? {
                return Err(ExError::new(ExErrorKind::AlreadyExists)
                    .with_entity_id(&constraint_id)
                    .with_message("Constraint already exists"));
            }

            let constraint =
                Constraint::new(constraint_id.clone(), family, kind, scope, payload_json);
            SqliteRepo::persist_constraint(conn, &constraint)?;
            Ok(McpCommandResult::ConstraintCreate { constraint_id })
        }

        McpCommand::ConstraintAttachToEp {
            ep_id,
            constraint_id,
            ordinal,
        } => {
            // Verify constraint exists and is active
            let constraint =
                SqliteRepo::get_constraint(conn, &constraint_id)?.ok_or_else(|| {
                    ExError::new(ExErrorKind::NotFound)
                        .with_entity_id(&constraint_id)
                        .with_message("Constraint not found")
                })?;
            if constraint.deleted_at.is_some() {
                return Err(ExError::new(ExErrorKind::ConstraintTombstoned)
                    .with_entity_id(&constraint_id)
                    .with_message("Constraint is tombstoned and cannot be attached"));
            }

            // Check for duplicate attachment
            let existing_refs = SqliteRepo::list_ep_constraint_refs(conn, &ep_id)?;
            if existing_refs
                .iter()
                .any(|r| r.constraint_id == constraint_id)
            {
                return Err(ExError::new(ExErrorKind::DuplicateAttachment)
                    .with_entity_id(&constraint_id)
                    .with_ep_id(&ep_id)
                    .with_message("Constraint already attached to EP"));
            }

            let ep_ref = EpConstraintRef::new(ep_id.clone(), constraint_id.clone(), ordinal);
            SqliteRepo::persist_ep_constraint_ref(conn, &ep_ref)?;
            Ok(McpCommandResult::ConstraintAttachToEp)
        }

        McpCommand::ProfileCreate {
            profile_ref,
            payload_json,
            source: _,
        } => {
            let engine_cmd = EngineCommand::ProfileCreate {
                profile_ref,
                payload_json,
                source: None,
            };
            apply_engine_command(engine_cmd, conn, cas, policy_provider, approval_router)?;
            Ok(McpCommandResult::ProfileCreate)
        }

        McpCommand::ProfileSetDefault { profile_ref } => {
            let engine_cmd = EngineCommand::ProfileSetDefault { profile_ref };
            apply_engine_command(engine_cmd, conn, cas, policy_provider, approval_router)?;
            Ok(McpCommandResult::ProfileSetDefault)
        }
    }
}
