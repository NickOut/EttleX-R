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
        &store,
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

/// Validate that an EP is a leaf (has no child_ettle_id).
///
/// ## Errors
///
/// - `ExErrorKind::NotFound`: EP not found
/// - `ExErrorKind::ConstraintViolation`: EP is not a leaf (has child_ettle_id)
fn validate_leaf_ep(store: &ettlex_core::ops::store::Store, ep_id: &str) -> Result<()> {
    use ettlex_core::errors::EttleXError;

    // Get EP (returns NotFound if missing or deleted)
    let ep = store.get_ep(ep_id).map_err(|e| match e {
        EttleXError::EpNotFound { .. } => ExError::new(ExErrorKind::NotFound)
            .with_op("validate_leaf_ep")
            .with_ep_id(ep_id)
            .with_message("EP not found"),
        EttleXError::EpDeleted { .. } => ExError::new(ExErrorKind::NotFound)
            .with_op("validate_leaf_ep")
            .with_ep_id(ep_id)
            .with_message("EP was deleted"),
        _ => ExError::from(e),
    })?;

    // Check if EP is a leaf
    if !ep.is_leaf() {
        return Err(ExError::new(ExErrorKind::ConstraintViolation)
            .with_op("validate_leaf_ep")
            .with_ep_id(ep_id)
            .with_message("EP is not a leaf (has child_ettle_id)"));
    }

    Ok(())
}

/// Commit a snapshot for a leaf EP (CANONICAL entry point).
///
/// This is the canonical way to commit a snapshot via action commands.
/// Use `apply_engine_command()` instead of calling this directly.
///
/// ## Arguments
///
/// - `leaf_ep_id`: Leaf EP identifier (must have no child_ettle_id)
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
/// - `ExErrorKind::NotFound`: EP not found or deleted
/// - `ExErrorKind::ConstraintViolation`: EP is not a leaf
/// - Other errors from snapshot_commit_impl
pub fn snapshot_commit_by_leaf(
    leaf_ep_id: &str,
    policy_ref: &str,
    profile_ref: &str,
    options: SnapshotOptions,
    conn: &mut Connection,
    cas: &FsStore,
) -> Result<SnapshotCommitResult> {
    // 1. Hydrate tree to validate leaf EP
    let store = ettlex_store::repo::hydration::load_tree(conn).map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("snapshot_commit_by_leaf")
            .with_message(format!("Failed to load tree: {}", e))
    })?;

    // 2. Validate that EP is a leaf
    validate_leaf_ep(&store, leaf_ep_id)?;

    // 3. Get the root ettle for this EP
    let ep = store.get_ep(leaf_ep_id).map_err(ExError::from)?;
    let root_ettle_id = &ep.ettle_id;

    // 4. Delegate to existing snapshot_commit implementation
    snapshot_commit(root_ettle_id, policy_ref, profile_ref, options, conn, cas)
}

/// Resolve root Ettle to exactly one leaf EP (legacy support).
///
/// This function walks the tree rooted at the given Ettle and finds all leaf EPs.
/// For deterministic resolution, exactly one leaf must exist.
///
/// ## Errors
///
/// - `ExErrorKind::NotFound`: Ettle not found or no leaf EPs exist
/// - `ExErrorKind::AmbiguousSelection`: Multiple leaf EPs exist (includes candidate IDs)
fn resolve_root_to_leaf(
    store: &ettlex_core::ops::store::Store,
    root_ettle_id: &str,
) -> Result<String> {
    use ettlex_core::errors::EttleXError;

    // Get root ettle (returns NotFound if missing or deleted)
    let ettle = store.get_ettle(root_ettle_id).map_err(|e| match e {
        EttleXError::EttleNotFound { .. } => ExError::new(ExErrorKind::NotFound)
            .with_op("resolve_root_to_leaf")
            .with_entity_id(root_ettle_id)
            .with_message("Root ettle not found"),
        EttleXError::EttleDeleted { .. } => ExError::new(ExErrorKind::NotFound)
            .with_op("resolve_root_to_leaf")
            .with_entity_id(root_ettle_id)
            .with_message("Root ettle was deleted"),
        _ => ExError::from(e),
    })?;

    // Find all leaf EPs in this ettle
    let leaf_eps: Vec<String> = ettle
        .ep_ids
        .iter()
        .filter_map(|ep_id| {
            store
                .get_ep(ep_id)
                .ok()
                .filter(|ep| ep.is_leaf())
                .map(|ep| ep.id.clone())
        })
        .collect();

    match leaf_eps.len() {
        0 => Err(ExError::new(ExErrorKind::NotFound)
            .with_op("resolve_root_to_leaf")
            .with_entity_id(root_ettle_id)
            .with_message("No leaf EPs found in root ettle")),
        1 => Ok(leaf_eps[0].clone()),
        _ => Err(ExError::new(ExErrorKind::AmbiguousSelection)
            .with_op("resolve_root_to_leaf")
            .with_entity_id(root_ettle_id)
            .with_message(format!("Multiple leaf EPs found: {:?}", leaf_eps))),
    }
}

/// Legacy entry point: resolve root to leaf, then commit.
///
/// This function provides backward compatibility for the old API that took
/// a root_ettle_id parameter. It resolves the root to exactly one leaf EP,
/// then delegates to `snapshot_commit_by_leaf()`.
///
/// ## Deterministic Resolution
///
/// - Succeeds if exactly one leaf EP exists in the root ettle
/// - Fails with `AmbiguousSelection` if multiple leaves exist
/// - Fails with `NotFound` if no leaves exist
///
/// ## Arguments
///
/// - `root_ettle_id`: Root ettle identifier
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
/// - `ExErrorKind::NotFound`: Root ettle not found or no leaf EPs exist
/// - `ExErrorKind::AmbiguousSelection`: Multiple leaf EPs exist
/// - Other errors from snapshot_commit_by_leaf
pub fn snapshot_commit_by_root_legacy(
    root_ettle_id: &str,
    policy_ref: &str,
    profile_ref: &str,
    options: SnapshotOptions,
    conn: &mut Connection,
    cas: &FsStore,
) -> Result<SnapshotCommitResult> {
    // 1. Hydrate tree to resolve leaf EP
    let store = ettlex_store::repo::hydration::load_tree(conn).map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("snapshot_commit_by_root_legacy")
            .with_message(format!("Failed to load tree: {}", e))
    })?;

    // 2. Resolve root to exactly one leaf
    let leaf_ep_id = resolve_root_to_leaf(&store, root_ettle_id)?;

    // 3. Delegate to leaf-scoped commit
    snapshot_commit_by_leaf(&leaf_ep_id, policy_ref, profile_ref, options, conn, cas)
}
