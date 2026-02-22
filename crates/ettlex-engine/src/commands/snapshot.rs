//! Snapshot commit orchestration.
//!
//! This module orchestrates the full snapshot commit pipeline:
//! 1. Hydrate current tree state from database
//! 2. Compute EPT for the root ettle
//! 3. Generate snapshot manifest
//! 4. Persist to CAS + ledger atomically
//!
//! ## Logging Ownership
//!
//! The engine layer owns lifecycle logging for snapshot_commit operations:
//! - `log_op_start!` at entry
//! - `log_op_end!` on success
//! - `log_op_error!` on failure
//!
//! Lower layers (store, core) use only `tracing::debug!()` for internal details.

#![allow(clippy::result_large_err)]

use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::snapshot::manifest::generate_manifest;
use ettlex_core::traversal::ept::compute_ept;
use ettlex_core::{log_op_end, log_op_error, log_op_start};
use ettlex_store::cas::FsStore;
use ettlex_store::errors::Result;
use ettlex_store::snapshot::persist::commit_snapshot as persist_commit_snapshot;
use rusqlite::{Connection, OptionalExtension};

/// Options for snapshot commit operation.
#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    /// Expected current head snapshot ID (for optimistic concurrency)
    pub expected_head: Option<String>,
    /// If true, compute manifest but don't persist
    pub dry_run: bool,
}

/// Result of a snapshot commit operation.
#[derive(Debug, Clone)]
pub struct SnapshotCommitResult {
    /// Unique snapshot identifier (UUIDv7, empty in dry-run)
    pub snapshot_id: String,
    /// Full manifest digest (CAS key)
    pub manifest_digest: String,
    /// Semantic digest (excludes created_at, for idempotency)
    pub semantic_manifest_digest: String,
    /// Whether this was a duplicate (idempotent return)
    pub was_duplicate: bool,
}

/// Commit a snapshot of the current tree state.
///
/// Orchestrates the full pipeline:
/// - Hydrates tree from database
/// - Computes EPT (Effective Processing Tree)
/// - Generates snapshot manifest
/// - Persists to CAS + ledger atomically
///
/// ## Arguments
///
/// - `root_ettle_id`: Root ettle to snapshot
/// - `policy_ref`: Policy identifier (e.g., "policy/default@0")
/// - `profile_ref`: Profile identifier (e.g., "profile/default@0")
/// - `options`: Commit options (expected_head, dry_run)
/// - `conn`: Database connection
/// - `cas`: CAS store instance
///
/// ## Returns
///
/// `SnapshotCommitResult` with snapshot ID and digests
///
/// ## Errors
///
/// - `ExErrorKind::NotFound`: Root ettle not found
/// - `ExErrorKind::CycleDetected`: Tree contains a cycle
/// - `ExErrorKind::Concurrency`: Expected head mismatch
/// - `ExErrorKind::Persistence`: Database or CAS error
///
/// ## Example
///
/// ```no_run
/// use ettlex_engine::commands::snapshot::{snapshot_commit, SnapshotOptions};
/// use ettlex_store::cas::FsStore;
/// use rusqlite::Connection;
///
/// let mut conn = Connection::open(".ettlex/store.db").unwrap();
/// let cas = FsStore::new(".ettlex/cas");
///
/// let result = snapshot_commit(
///     "ettle:root",
///     "policy/default@0",
///     "profile/default@0",
///     SnapshotOptions {
///         expected_head: None,
///         dry_run: false,
///     },
///     &mut conn,
///     &cas,
/// ).unwrap();
///
/// println!("Snapshot ID: {}", result.snapshot_id);
/// ```
pub fn snapshot_commit(
    root_ettle_id: &str,
    policy_ref: &str,
    profile_ref: &str,
    options: SnapshotOptions,
    conn: &mut Connection,
    cas: &FsStore,
) -> Result<SnapshotCommitResult> {
    log_op_start!("snapshot_commit", root_ettle_id = root_ettle_id);

    let start = std::time::Instant::now();

    let result = snapshot_commit_impl(root_ettle_id, policy_ref, profile_ref, options, conn, cas)
        .map_err(|e| {
        let duration_ms = start.elapsed().as_millis() as u64;
        log_op_error!("snapshot_commit", e.clone(), duration_ms = duration_ms);
        e
    })?;

    let duration_ms = start.elapsed().as_millis() as u64;
    log_op_end!(
        "snapshot_commit",
        duration_ms = duration_ms,
        snapshot_id = &result.snapshot_id
    );

    Ok(result)
}

/// Internal implementation (separated for error handling).
fn snapshot_commit_impl(
    root_ettle_id: &str,
    policy_ref: &str,
    profile_ref: &str,
    options: SnapshotOptions,
    conn: &mut Connection,
    cas: &FsStore,
) -> Result<SnapshotCommitResult> {
    // 1. Hydrate tree from database
    let store = ettlex_store::repo::hydration::load_tree(conn).map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("snapshot_commit")
            .with_message(format!("Failed to load tree: {}", e))
    })?;

    // 2. Compute EPT for root ettle
    let ept = compute_ept(&store, root_ettle_id, None)?;

    // 3. Get store schema version from migrations
    let store_schema_version = get_store_schema_version(conn)?;

    // 4. Get seed digest if exists (optional)
    let seed_digest = get_seed_digest(conn)?;

    // 5. Generate snapshot manifest
    let manifest = generate_manifest(
        ept,
        policy_ref.to_string(),
        profile_ref.to_string(),
        root_ettle_id.to_string(),
        store_schema_version,
        seed_digest,
    )?;

    // 6. Persist to CAS + ledger
    let persist_options = ettlex_store::snapshot::persist::SnapshotOptions {
        expected_head: options.expected_head,
        dry_run: options.dry_run,
    };

    let persist_result = persist_commit_snapshot(conn, cas, manifest, persist_options)?;

    Ok(SnapshotCommitResult {
        snapshot_id: persist_result.snapshot_id,
        manifest_digest: persist_result.manifest_digest,
        semantic_manifest_digest: persist_result.semantic_manifest_digest,
        was_duplicate: persist_result.was_duplicate,
    })
}

/// Get the current store schema version from schema_version table.
fn get_store_schema_version(conn: &Connection) -> Result<String> {
    let version: String = conn
        .query_row(
            "SELECT migration_id FROM schema_version ORDER BY id DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("get_store_schema_version")
                .with_message(format!("Failed to query schema_version: {}", e))
        })?;

    Ok(version)
}

/// Get the seed digest from metadata (if exists).
fn get_seed_digest(conn: &Connection) -> Result<Option<String>> {
    // Check if metadata table exists first
    let table_exists: bool = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='metadata'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if !table_exists {
        // Metadata table doesn't exist - this is fine, seed_digest is optional
        return Ok(None);
    }

    let digest: Option<String> = conn
        .query_row(
            "SELECT value FROM metadata WHERE key = 'seed_digest'",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("get_seed_digest")
                .with_message(format!("Failed to query metadata: {}", e))
        })?;

    Ok(digest)
}
