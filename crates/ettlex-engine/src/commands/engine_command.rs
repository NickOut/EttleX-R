//! Engine-level action commands for I/O operations.
//!
//! This module provides the canonical entry point for operations that require
//! database and CAS access. All mutation operations should flow through
//! `apply_engine_command()` to ensure consistent validation and error handling.
//!
//! ## Design Rationale
//!
//! The `Command` enum lives in `ettlex-core` (pure domain, no I/O).
//! This `EngineCommand` enum lives in `ettlex-engine` and handles operations
//! that require database and CAS access (I/O operations).
//!
//! This preserves layer boundaries while ensuring action commands are the
//! canonical ingress for all mutations.

#![allow(clippy::result_large_err)]

use crate::commands::snapshot::{SnapshotCommitResult, SnapshotOptions};
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use rusqlite::Connection;

/// Engine-level commands that require I/O (database, CAS).
#[derive(Debug, Clone)]
pub enum EngineCommand {
    /// Commit a snapshot for a leaf EP.
    SnapshotCommit {
        /// Leaf EP identifier (must have no child_ettle_id)
        leaf_ep_id: String,
        /// Policy reference (e.g., "policy/default@0")
        policy_ref: String,
        /// Profile reference (e.g., "profile/default@0")
        profile_ref: String,
        /// Commit options
        options: SnapshotOptions,
    },
}

/// Result of applying an engine command.
#[derive(Debug, Clone)]
pub enum EngineCommandResult {
    /// Result of a snapshot commit operation.
    SnapshotCommit(SnapshotCommitResult),
}

/// Apply an engine command (CANONICAL entry point for I/O operations).
///
/// This is the canonical way to execute operations that require database
/// and CAS access. All mutation operations should flow through this function
/// to ensure consistent validation, error handling, and governance.
///
/// ## Arguments
///
/// - `cmd`: The engine command to execute
/// - `conn`: Database connection
/// - `cas`: CAS store instance
///
/// ## Returns
///
/// `EngineCommandResult` with operation-specific result data
///
/// ## Errors
///
/// Returns operation-specific errors (see individual command documentation)
///
/// ## Example
///
/// ```no_run
/// use ettlex_engine::commands::engine_command::{EngineCommand, apply_engine_command};
/// use ettlex_engine::commands::snapshot::SnapshotOptions;
/// use ettlex_store::cas::FsStore;
/// use rusqlite::Connection;
///
/// let mut conn = Connection::open(".ettlex/store.db").unwrap();
/// let cas = FsStore::new(".ettlex/cas");
///
/// let cmd = EngineCommand::SnapshotCommit {
///     leaf_ep_id: "ep:my-leaf:0".to_string(),
///     policy_ref: "policy/default@0".to_string(),
///     profile_ref: "profile/default@0".to_string(),
///     options: SnapshotOptions {
///         expected_head: None,
///         dry_run: false,
///     },
/// };
///
/// let result = apply_engine_command(cmd, &mut conn, &cas).unwrap();
/// ```
pub fn apply_engine_command(
    cmd: EngineCommand,
    conn: &mut Connection,
    cas: &FsStore,
) -> Result<EngineCommandResult> {
    match cmd {
        EngineCommand::SnapshotCommit {
            leaf_ep_id,
            policy_ref,
            profile_ref,
            options,
        } => {
            let result = crate::commands::snapshot::snapshot_commit_by_leaf(
                &leaf_ep_id,
                &policy_ref,
                &profile_ref,
                options,
                conn,
                cas,
            )?;
            Ok(EngineCommandResult::SnapshotCommit(result))
        }
    }
}
