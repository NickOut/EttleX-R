//! MCP-layer command dispatch.
//!
//! `McpCommand` is the JSON-deserialisable command enum that arrives from the
//! MCP `ettlex.apply` tool.  `apply_mcp_command` dispatches to the appropriate
//! engine or store function, wraps the result in `McpCommandResult`, and
//! appends a row to `mcp_command_log` (optimistic-concurrency counter).

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::model::{Constraint, Ep, EpConstraintRef};
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::commands::ettle::{handle_ettle_create, handle_ettle_tombstone, handle_ettle_update};

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
        /// Optional. If absent/null the action layer resolves the policy ref via
        /// `PolicyProvider::get_default_policy_ref()`; if that also returns `None`,
        /// permissive pass-through is used (empty string recorded in the manifest).
        #[serde(default)]
        policy_ref: Option<String>,
        profile_ref: Option<String>,
        #[serde(default)]
        dry_run: bool,
        expected_head: Option<String>,
    },

    // ── Ettle ────────────────────────────────────────────────────────────────
    /// Create a new Ettle.
    ///
    /// `ettle_id` MUST be `None` (or absent from JSON). If a caller supplies
    /// an `ettle_id`, the command is rejected with `InvalidInput`.
    EttleCreate {
        title: String,
        /// Must not be supplied — ID is auto-generated.
        #[serde(default)]
        ettle_id: Option<String>,
        #[serde(default)]
        why: Option<String>,
        #[serde(default)]
        what: Option<String>,
        #[serde(default)]
        how: Option<String>,
        #[serde(default)]
        reasoning_link_id: Option<String>,
        #[serde(default)]
        reasoning_link_type: Option<String>,
    },

    /// Update an existing Ettle's content fields.
    ///
    /// At least one field must be supplied; omitted fields are preserved.
    /// `reasoning_link_id: null` clears the link. `reasoning_link_id` absent
    /// preserves the existing value.
    EttleUpdate {
        ettle_id: String,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        why: Option<String>,
        #[serde(default)]
        what: Option<String>,
        #[serde(default)]
        how: Option<String>,
        #[serde(default, deserialize_with = "deserialize_double_option")]
        reasoning_link_id: Option<Option<String>>,
        #[serde(default, deserialize_with = "deserialize_double_option")]
        reasoning_link_type: Option<Option<String>>,
    },

    /// Soft-delete (tombstone) an Ettle.
    ///
    /// The Ettle must have no active dependants.
    EttleTombstone { ettle_id: String },

    // ── EP ───────────────────────────────────────────────────────────────────
    /// Create a new EP.
    ///
    /// `ep_id` MUST be `None` (or absent from JSON). If a caller supplies an
    /// `ep_id`, the command is rejected with `InvalidInput`.
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
        /// Must not be supplied — ID is auto-generated.
        #[serde(default)]
        ep_id: Option<String>,
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

    // ── Policy ───────────────────────────────────────────────────────────────
    /// Create a new policy document in the policy provider.
    ///
    /// `policy_ref` must be non-empty and contain `@` separator.
    /// `text` must be non-empty.
    PolicyCreate { policy_ref: String, text: String },
}

fn default_true() -> bool {
    true
}

/// Deserializer for `Option<Option<T>>` that distinguishes absent from null.
///
/// - Field absent → `None` (do not update)
/// - Field `null` → `Some(None)` (clear)
/// - Field `"value"` → `Some(Some("value"))` (set)
fn deserialize_double_option<'de, D, T>(
    deserializer: D,
) -> std::result::Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: serde::Deserialize<'de>,
{
    use serde::Deserialize;
    Ok(Some(Option::<T>::deserialize(deserializer)?))
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
    EttleUpdate,
    EttleTombstone,
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
    PolicyCreate {
        policy_ref: String,
    },
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

    // 4. Append provenance event for successful ettle mutations
    let prov_kind_and_id: Option<(&str, String)> = match &result {
        McpCommandResult::EttleCreate { ettle_id } => Some(("ettle_created", ettle_id.clone())),
        McpCommandResult::EttleUpdate => None, // correlation id not available here
        McpCommandResult::EttleTombstone => None,
        _ => None,
    };
    // We capture what we can; for update/tombstone we just record the kind
    let prov_kind: Option<&str> = match &result {
        McpCommandResult::EttleCreate { .. } => Some("ettle_created"),
        McpCommandResult::EttleUpdate => Some("ettle_updated"),
        McpCommandResult::EttleTombstone => Some("ettle_tombstoned"),
        _ => None,
    };
    if let Some(kind) = prov_kind {
        let correlation_id = match &prov_kind_and_id {
            Some((_, id)) => id.clone(),
            None => uuid::Uuid::now_v7().to_string(),
        };
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        conn.execute(
            "INSERT INTO provenance_events (kind, correlation_id, timestamp) VALUES (?1, ?2, ?3)",
            rusqlite::params![kind, correlation_id, now_ms],
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("apply_mcp_command")
                .with_message(format!("Failed to insert provenance_events row: {}", e))
        })?;
    }

    // 5. Insert log row
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

        McpCommand::EttleCreate {
            title,
            ettle_id,
            why,
            what,
            how,
            reasoning_link_id,
            reasoning_link_type,
        } => {
            // Identity contract: caller must not supply an ettle_id
            if ettle_id.is_some() {
                return Err(ExError::new(ExErrorKind::InvalidInput)
                    .with_op("ettle_create")
                    .with_message("ettle_id must not be supplied; it is auto-generated"));
            }
            let new_id = handle_ettle_create(
                conn,
                &title,
                why.as_deref(),
                what.as_deref(),
                how.as_deref(),
                reasoning_link_id.as_deref(),
                reasoning_link_type.as_deref(),
            )?;
            Ok(McpCommandResult::EttleCreate { ettle_id: new_id })
        }

        McpCommand::EttleUpdate {
            ettle_id,
            title,
            why,
            what,
            how,
            reasoning_link_id,
            reasoning_link_type,
        } => {
            // Convert Option<Option<String>> → Option<Option<&str>>
            // We must bind the inner Option<String> to a local so the &str lives long enough.
            let link_id_inner: Option<Option<String>> = reasoning_link_id;
            let link_type_inner: Option<Option<String>> = reasoning_link_type;
            let link_id_ref: Option<Option<&str>> = link_id_inner.as_ref().map(|v| v.as_deref());
            let link_type_ref: Option<Option<&str>> =
                link_type_inner.as_ref().map(|v| v.as_deref());
            handle_ettle_update(
                conn,
                &ettle_id,
                title.as_deref(),
                why.as_deref(),
                what.as_deref(),
                how.as_deref(),
                link_id_ref,
                link_type_ref,
            )?;
            Ok(McpCommandResult::EttleUpdate)
        }

        McpCommand::EttleTombstone { ettle_id } => {
            handle_ettle_tombstone(conn, &ettle_id)?;
            Ok(McpCommandResult::EttleTombstone)
        }

        McpCommand::EpCreate {
            ettle_id,
            ordinal,
            normative,
            why,
            what,
            how,
            ep_id,
        } => {
            // Identity contract: caller must not supply an ep_id
            if ep_id.is_some() {
                return Err(ExError::new(ExErrorKind::InvalidInput)
                    .with_op("ep_create")
                    .with_message("ep_id must not be supplied; it is auto-generated"));
            }
            // Validate that the referenced ettle exists (use v2 record API)
            if SqliteRepo::get_ettle_record(conn, &ettle_id)?.is_none() {
                return Err(ExError::new(ExErrorKind::NotFound)
                    .with_op("ep_create")
                    .with_entity_id(&ettle_id)
                    .with_message(format!("Ettle not found: {}", ettle_id)));
            }
            let generated_ep_id = format!("ep:{}:{}", uuid::Uuid::now_v7(), ordinal);
            let ep = Ep::new(
                generated_ep_id.clone(),
                ettle_id,
                ordinal,
                normative,
                why,
                what,
                how,
            );
            SqliteRepo::persist_ep(conn, &ep)?;
            Ok(McpCommandResult::EpCreate {
                ep_id: generated_ep_id,
            })
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

        McpCommand::PolicyCreate { policy_ref, text } => {
            // Validate at the action layer (also validated inside FilePolicyProvider)
            if policy_ref.is_empty() {
                return Err(ExError::new(ExErrorKind::InvalidInput)
                    .with_op("policy_create")
                    .with_message("policy_ref must not be empty"));
            }
            if !policy_ref.contains('@') {
                return Err(ExError::new(ExErrorKind::InvalidInput)
                    .with_op("policy_create")
                    .with_message("policy_ref must contain '@' version separator"));
            }
            if text.is_empty() {
                return Err(ExError::new(ExErrorKind::InvalidInput)
                    .with_op("policy_create")
                    .with_message("policy text must not be empty"));
            }
            policy_provider.policy_create(&policy_ref, &text)?;
            Ok(McpCommandResult::PolicyCreate {
                policy_ref: policy_ref.clone(),
            })
        }
    }
}
