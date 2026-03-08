//! Engine-level action commands for I/O operations.

#![allow(clippy::result_large_err)]

use crate::commands::snapshot::{
    RoutedForApprovalResult, SnapshotCommitOutcome, SnapshotCommitResult, SnapshotOptions,
};
use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::profile::{create_profile, set_default_profile};
use rusqlite::Connection;

/// Engine-level commands that require I/O (database, CAS).
#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// Commit a snapshot for a leaf EP.
    SnapshotCommit {
        leaf_ep_id: String,
        /// Optional policy ref. If `None`, the action layer resolves via
        /// `PolicyProvider::get_default_policy_ref()`; if that also returns `None`,
        /// a permissive pass-through (empty string recorded in the manifest) is used.
        policy_ref: Option<String>,
        /// Optional profile ref; None triggers deterministic defaulting.
        profile_ref: Option<String>,
        options: SnapshotOptions,
    },
    /// Create a profile (idempotent on same canonical content; ProfileConflict on mismatch).
    ProfileCreate {
        profile_ref: String,
        payload_json: serde_json::Value,
        source: Option<String>,
    },
    /// Set a profile as the repository default.
    ProfileSetDefault { profile_ref: String },
}

/// Result of applying an engine command.
#[derive(Debug, Clone)]
pub enum EngineCommandResult {
    /// Snapshot was successfully committed.
    SnapshotCommit(SnapshotCommitResult),
    /// Snapshot commit was routed for approval.
    SnapshotCommitRouted(RoutedForApprovalResult),
    /// Profile was created (or already existed with same content).
    ProfileCreate,
    /// Profile default was updated.
    ProfileSetDefault,
}

/// Apply an engine command with policy provider and approval router.
pub fn apply_engine_command(
    cmd: EngineCommand,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<EngineCommandResult> {
    match cmd {
        EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref,
            profile_ref,
            options,
        } => {
            let outcome = crate::commands::snapshot::snapshot_commit_by_leaf(
                &leaf_ep_id,
                policy_ref.as_deref(),
                profile_ref.as_deref(),
                options,
                conn,
                cas,
                policy_provider,
                approval_router,
            )?;
            match outcome {
                SnapshotCommitOutcome::Committed(r) => Ok(EngineCommandResult::SnapshotCommit(r)),
                SnapshotCommitOutcome::RoutedForApproval(r) => {
                    Ok(EngineCommandResult::SnapshotCommitRouted(r))
                }
            }
        }
        EngineCommand::ProfileCreate {
            profile_ref,
            payload_json,
            ..
        } => {
            create_profile(conn, &profile_ref, &payload_json)?;
            Ok(EngineCommandResult::ProfileCreate)
        }
        EngineCommand::ProfileSetDefault { profile_ref } => {
            set_default_profile(conn, &profile_ref)?;
            Ok(EngineCommandResult::ProfileSetDefault)
        }
    }
}
