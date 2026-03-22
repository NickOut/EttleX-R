//! Engine command dispatch.
//!
//! `Command` is the JSON-deserialisable command enum that arrives from the
//! MCP `ettlex.apply` tool.  `apply_command` dispatches to the appropriate
//! engine or store function, wraps the result in `CommandResult`, and
//! appends a row to `command_log` (optimistic-concurrency counter).

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::model::Ep;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::model::{GroupMemberRecord, GroupRecord, RelationRecord};
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::commands::ettle::{handle_ettle_create, handle_ettle_tombstone, handle_ettle_update};
use crate::commands::group::{
    handle_group_create, handle_group_get, handle_group_list, handle_group_member_add,
    handle_group_member_list, handle_group_member_remove, handle_group_tombstone,
};
use crate::commands::relation::{
    handle_relation_create, handle_relation_get, handle_relation_list, handle_relation_tombstone,
    handle_relation_update,
};

use crate::commands::engine_command::{apply_engine_command, EngineCommand, EngineCommandResult};
use crate::commands::snapshot::SnapshotOptions;

// ---------------------------------------------------------------------------
// Command — serialisable command vocabulary
// ---------------------------------------------------------------------------

/// All write operations available via the MCP `ettlex.apply` tool.
///
/// Serialised as a tagged JSON object: `{ "tag": "...", ...fields }`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "tag")]
pub enum Command {
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

    // ── Relations ─────────────────────────────────────────────────────────────
    /// Create a new relation between two ettles.
    ///
    /// `relation_id` MUST be `None` (or absent from JSON). If a caller supplies
    /// a `relation_id`, the command is rejected with `InvalidInput`.
    RelationCreate {
        source_ettle_id: String,
        target_ettle_id: String,
        relation_type: String,
        #[serde(default)]
        properties_json: Option<JsonValue>,
        /// Must not be supplied — ID is auto-generated.
        #[serde(default)]
        relation_id: Option<String>,
    },

    /// Update a relation's properties_json.
    ///
    /// At least one field must be supplied (`EmptyUpdate`).
    RelationUpdate {
        relation_id: String,
        /// New properties_json (required — must be present)
        properties_json: Option<JsonValue>,
    },

    /// Get a relation by ID.
    RelationGet { relation_id: String },

    /// List relations with optional filters.
    ///
    /// At least one of source_ettle_id, target_ettle_id, or relation_type must be supplied.
    RelationList {
        #[serde(default)]
        source_ettle_id: Option<String>,
        #[serde(default)]
        target_ettle_id: Option<String>,
        #[serde(default)]
        relation_type: Option<String>,
        #[serde(default)]
        include_tombstoned: bool,
    },

    /// Tombstone a relation.
    RelationTombstone { relation_id: String },

    // ── Groups ────────────────────────────────────────────────────────────────
    /// Create a new group.
    GroupCreate { name: String },

    /// Get a group by ID.
    GroupGet { group_id: String },

    /// List groups.
    GroupList {
        #[serde(default)]
        include_tombstoned: bool,
    },

    /// Tombstone a group.
    GroupTombstone { group_id: String },

    /// Add an ettle to a group.
    GroupMemberAdd { group_id: String, ettle_id: String },

    /// Remove an ettle from a group (tombstones the membership record).
    GroupMemberRemove { group_id: String, ettle_id: String },

    /// List members of a group.
    GroupMemberList {
        group_id: String,
        #[serde(default)]
        include_tombstoned: bool,
    },
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
// CommandResult
// ---------------------------------------------------------------------------

/// Result of a successful `apply_command` call.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "tag")]
pub enum CommandResult {
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
    ProfileCreate,
    ProfileSetDefault,
    PolicyCreate {
        policy_ref: String,
    },
    RelationCreate {
        relation_id: String,
    },
    RelationUpdate,
    RelationGet {
        record: RelationRecord,
    },
    RelationList {
        items: Vec<RelationRecord>,
    },
    RelationTombstone,
    GroupCreate {
        group_id: String,
    },
    GroupGet {
        record: GroupRecord,
    },
    GroupList {
        items: Vec<GroupRecord>,
    },
    GroupTombstone,
    GroupMemberAdd,
    GroupMemberRemove,
    GroupMemberList {
        items: Vec<GroupMemberRecord>,
    },
}

// ---------------------------------------------------------------------------
// apply_command
// ---------------------------------------------------------------------------

/// Apply a command, enforcing OCC via `command_log`.
///
/// Steps:
/// 1. Read current `state_version` (COUNT(*) from `command_log`).
/// 2. If `expected_state_version` is `Some(v)` and `v != current` → `HeadMismatch`.
/// 3. Execute the command.
/// 4. Append provenance event for successful mutations.
/// 5. Insert a row into `command_log` → `new_state_version = current + 1`.
///
/// Returns `(CommandResult, new_state_version)`.
pub fn apply_command(
    cmd: Command,
    expected_state_version: Option<u64>,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<(CommandResult, u64)> {
    // 1. Read state_version
    let current_sv: u64 = conn
        .query_row("SELECT COUNT(*) FROM command_log", [], |r| r.get(0))
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("apply_command")
                .with_message(format!("Failed to read command_log: {}", e))
        })?;

    // 2. OCC check
    if let Some(expected) = expected_state_version {
        if expected != current_sv {
            return Err(ExError::new(ExErrorKind::HeadMismatch)
                .with_op("apply_command")
                .with_message(format!(
                    "state_version mismatch: expected {} but current is {}",
                    expected, current_sv
                )));
        }
    }

    // 3. Dispatch
    let result = dispatch_command(cmd, conn, cas, policy_provider, approval_router)?;

    // 4. Append provenance event for successful mutations
    let prov_kind: Option<(&str, String)> = match &result {
        CommandResult::EttleCreate { ettle_id } => Some(("ettle_created", ettle_id.clone())),
        CommandResult::EttleUpdate => Some(("ettle_updated", uuid::Uuid::now_v7().to_string())),
        CommandResult::EttleTombstone => {
            Some(("ettle_tombstoned", uuid::Uuid::now_v7().to_string()))
        }
        CommandResult::RelationCreate { relation_id } => {
            Some(("relation_created", relation_id.clone()))
        }
        CommandResult::RelationUpdate => {
            Some(("relation_updated", uuid::Uuid::now_v7().to_string()))
        }
        CommandResult::RelationTombstone => {
            Some(("relation_tombstoned", uuid::Uuid::now_v7().to_string()))
        }
        CommandResult::GroupCreate { group_id } => Some(("group_created", group_id.clone())),
        CommandResult::GroupTombstone => {
            Some(("group_tombstoned", uuid::Uuid::now_v7().to_string()))
        }
        CommandResult::GroupMemberAdd => {
            Some(("group_member_added", uuid::Uuid::now_v7().to_string()))
        }
        CommandResult::GroupMemberRemove => {
            Some(("group_member_removed", uuid::Uuid::now_v7().to_string()))
        }
        _ => None,
    };

    if let Some((kind, correlation_id)) = prov_kind {
        let now_iso = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO provenance_events (kind, correlation_id, occurred_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![kind, correlation_id, now_iso],
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("apply_command")
                .with_message(format!("Failed to insert provenance_events row: {}", e))
        })?;
    }

    // 5. Insert log row with ISO-8601 timestamp
    let now_iso = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO command_log (applied_at) VALUES (?1)",
        rusqlite::params![now_iso],
    )
    .map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("apply_command")
            .with_message(format!("Failed to insert command_log row: {}", e))
    })?;

    Ok((result, current_sv + 1))
}

// ---------------------------------------------------------------------------
// Internal dispatch
// ---------------------------------------------------------------------------

fn dispatch_command(
    cmd: Command,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<CommandResult> {
    match cmd {
        Command::SnapshotCommit {
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
                EngineCommandResult::SnapshotCommit(r) => Ok(CommandResult::SnapshotCommit {
                    snapshot_id: r.snapshot_id,
                    manifest_digest: r.manifest_digest,
                }),
                EngineCommandResult::SnapshotCommitRouted(r) => {
                    Ok(CommandResult::RoutedForApproval {
                        approval_token: r.approval_token,
                    })
                }
                _ => Err(ExError::new(ExErrorKind::Internal)
                    .with_op("dispatch_command")
                    .with_message("Unexpected EngineCommandResult variant")),
            }
        }

        Command::EttleCreate {
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
            Ok(CommandResult::EttleCreate { ettle_id: new_id })
        }

        Command::EttleUpdate {
            ettle_id,
            title,
            why,
            what,
            how,
            reasoning_link_id,
            reasoning_link_type,
        } => {
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
            Ok(CommandResult::EttleUpdate)
        }

        Command::EttleTombstone { ettle_id } => {
            handle_ettle_tombstone(conn, &ettle_id)?;
            Ok(CommandResult::EttleTombstone)
        }

        Command::EpCreate {
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
            // Validate that the referenced ettle exists
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
            Ok(CommandResult::EpCreate {
                ep_id: generated_ep_id,
            })
        }

        Command::EpUpdate {
            ep_id,
            why,
            what,
            how,
            title,
        } => {
            if why.is_none() && what.is_none() && how.is_none() && title.is_none() {
                return Err(ExError::new(ExErrorKind::EmptyUpdate)
                    .with_ep_id(&ep_id)
                    .with_op("ep_update")
                    .with_message("EpUpdate requires at least one field"));
            }

            let mut ep = SqliteRepo::get_ep(conn, &ep_id)?.ok_or_else(|| {
                ExError::new(ExErrorKind::NotFound)
                    .with_ep_id(&ep_id)
                    .with_op("ep_update")
                    .with_message(format!("EP not found: {}", ep_id))
            })?;

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

            ep.recompute_content_digest();
            ep.updated_at = chrono::Utc::now();

            SqliteRepo::persist_ep(conn, &ep)?;
            Ok(CommandResult::EpUpdate {
                ep_id: ep_id.clone(),
            })
        }

        Command::ProfileCreate {
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
            Ok(CommandResult::ProfileCreate)
        }

        Command::ProfileSetDefault { profile_ref } => {
            let engine_cmd = EngineCommand::ProfileSetDefault { profile_ref };
            apply_engine_command(engine_cmd, conn, cas, policy_provider, approval_router)?;
            Ok(CommandResult::ProfileSetDefault)
        }

        Command::PolicyCreate { policy_ref, text } => {
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
            Ok(CommandResult::PolicyCreate {
                policy_ref: policy_ref.clone(),
            })
        }

        Command::RelationCreate {
            source_ettle_id,
            target_ettle_id,
            relation_type,
            properties_json,
            relation_id,
        } => handle_relation_create(
            conn,
            source_ettle_id,
            target_ettle_id,
            relation_type,
            properties_json,
            relation_id,
        ),

        Command::RelationUpdate {
            relation_id,
            properties_json,
        } => handle_relation_update(conn, relation_id, properties_json),

        Command::RelationGet { relation_id } => handle_relation_get(conn, relation_id),

        Command::RelationList {
            source_ettle_id,
            target_ettle_id,
            relation_type,
            include_tombstoned,
        } => handle_relation_list(
            conn,
            source_ettle_id,
            target_ettle_id,
            relation_type,
            include_tombstoned,
        ),

        Command::RelationTombstone { relation_id } => handle_relation_tombstone(conn, relation_id),

        Command::GroupCreate { name } => handle_group_create(conn, name),

        Command::GroupGet { group_id } => handle_group_get(conn, group_id),

        Command::GroupList { include_tombstoned } => handle_group_list(conn, include_tombstoned),

        Command::GroupTombstone { group_id } => handle_group_tombstone(conn, group_id),

        Command::GroupMemberAdd { group_id, ettle_id } => {
            handle_group_member_add(conn, group_id, ettle_id)
        }

        Command::GroupMemberRemove { group_id, ettle_id } => {
            handle_group_member_remove(conn, group_id, ettle_id)
        }

        Command::GroupMemberList {
            group_id,
            include_tombstoned,
        } => handle_group_member_list(conn, group_id, include_tombstoned),
    }
}
