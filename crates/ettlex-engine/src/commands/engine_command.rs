//! Engine-level action commands for I/O operations.

#![allow(clippy::result_large_err)]

use crate::commands::snapshot::{
    RoutedForApprovalResult, SnapshotCommitOutcome, SnapshotCommitResult, SnapshotOptions,
};
use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::policy::CommitPolicyHook;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use rusqlite::Connection;

/// Engine-level commands that require I/O (database, CAS).
#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// Commit a snapshot for a leaf EP.
    SnapshotCommit {
        leaf_ep_id: String,
        policy_ref: String,
        /// Optional profile ref; None triggers deterministic defaulting.
        profile_ref: Option<String>,
        options: SnapshotOptions,
    },
}

/// Result of applying an engine command.
#[derive(Debug, Clone)]
pub enum EngineCommandResult {
    /// Snapshot was successfully committed.
    SnapshotCommit(SnapshotCommitResult),
    /// Snapshot commit was routed for approval.
    SnapshotCommitRouted(RoutedForApprovalResult),
}

/// Apply an engine command with policy hook and approval router.
pub fn apply_engine_command(
    cmd: EngineCommand,
    conn: &mut Connection,
    cas: &FsStore,
    policy_hook: &dyn CommitPolicyHook,
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
                &policy_ref,
                profile_ref.as_deref(),
                options,
                conn,
                cas,
                policy_hook,
                approval_router,
            )?;
            match outcome {
                SnapshotCommitOutcome::Committed(r) => Ok(EngineCommandResult::SnapshotCommit(r)),
                SnapshotCommitOutcome::RoutedForApproval(r) => {
                    Ok(EngineCommandResult::SnapshotCommitRouted(r))
                }
            }
        }
    }
}
