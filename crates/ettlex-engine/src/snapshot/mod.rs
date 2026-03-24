//! Snapshot pipeline stub — EP Retirement (Slice 03).
//!
//! The snapshot commit pipeline has been deferred pending re-specification
//! against the Ettle/Relation model. All operations return `NotImplemented`.

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use rusqlite::Connection;

/// Options for snapshot commit operations.
#[derive(Debug, Clone, Default)]
pub struct SnapshotOptions {
    pub expected_head: Option<String>,
    pub dry_run: bool,
    pub allow_dedup: bool,
}

/// Result of a successful snapshot commit.
#[derive(Debug, Clone)]
pub struct SnapshotCommitResult {
    pub snapshot_id: String,
    pub manifest_digest: String,
}

/// Outcome of a snapshot commit attempt.
#[derive(Debug, Clone)]
pub enum SnapshotCommitOutcome {
    Committed(SnapshotCommitResult),
    RoutedForApproval(RoutedForApprovalResult),
}

/// Result when a snapshot commit is routed for approval.
#[derive(Debug, Clone)]
pub struct RoutedForApprovalResult {
    pub approval_token: String,
}

/// Commit a snapshot for a leaf EP — STUB, returns `NotImplemented`.
///
/// The snapshot pipeline has been deferred pending re-specification against
/// the Ettle/Relation model (EP construct retired in Slice 03).
///
/// # Errors
/// Always returns `NotImplemented` — snapshot pipeline deferred in Slice 03.
#[allow(unused_variables, clippy::too_many_arguments)]
pub fn snapshot_commit_by_leaf(
    leaf_ep_id: &str,
    policy_ref: Option<&str>,
    profile_ref: Option<&str>,
    options: SnapshotOptions,
    conn: &mut Connection,
    cas: &FsStore,
    policy_provider: &dyn PolicyProvider,
    approval_router: &dyn ApprovalRouter,
) -> Result<SnapshotCommitOutcome> {
    Err(ExError::new(ExErrorKind::NotImplemented)
        .with_op("snapshot_commit_by_leaf")
        .with_message(
            "Snapshot pipeline deferred — EP construct retired in Slice 03. \
             Re-specify against Ettle/Relation model.",
        ))
}
