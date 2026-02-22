//! Snapshot persistence operations.
//!
//! Provides functions for persisting snapshot manifests to CAS and creating
//! ledger entries in the snapshots table.

#![allow(clippy::result_large_err)]

use crate::cas::FsStore;
use crate::errors::Result;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::snapshot::manifest::SnapshotManifest;
use rusqlite::{Connection, OptionalExtension, Transaction};

/// Options for snapshot commit operation.
#[derive(Debug, Clone)]
pub struct SnapshotOptions {
    /// Expected current head snapshot ID (for optimistic concurrency)
    pub expected_head: Option<String>,
    /// If true, compute digests but don't persist
    pub dry_run: bool,
}

/// Result of a snapshot commit operation.
#[derive(Debug, Clone)]
pub struct SnapshotCommitResult {
    /// Unique snapshot identifier (UUIDv7)
    pub snapshot_id: String,
    /// Full manifest digest (includes created_at)
    pub manifest_digest: String,
    /// Semantic digest (excludes created_at, for idempotency)
    pub semantic_manifest_digest: String,
    /// Whether this was a duplicate (idempotent return)
    pub was_duplicate: bool,
}

/// Persist a snapshot manifest to content-addressable storage.
///
/// Writes the manifest as JSON to CAS and returns the digest. This operation
/// is idempotent: writing the same content multiple times succeeds and returns
/// the same digest.
///
/// ## Arguments
///
/// - `store`: CAS store instance
/// - `manifest`: Snapshot manifest to persist
///
/// ## Returns
///
/// SHA256 digest of the persisted manifest (hex-encoded, 64 characters)
///
/// ## Errors
///
/// - `ExErrorKind::Persistence`: CAS write failed
/// - `ExErrorKind::Serialization`: JSON serialization failed
pub fn persist_manifest_to_cas(store: &FsStore, manifest: &SnapshotManifest) -> Result<String> {
    // Serialize manifest to JSON
    let json = serde_json::to_string_pretty(manifest).map_err(|e| {
        ExError::new(ExErrorKind::Serialization)
            .with_op("persist_manifest_to_cas")
            .with_message(format!("Failed to serialize manifest: {}", e))
    })?;

    // Write to CAS (idempotent)
    let digest = store.write(json.as_bytes(), "json").map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("persist_manifest_to_cas")
            .with_message(format!("Failed to write manifest to CAS: {}", e))
    })?;

    tracing::debug!(
        digest = %digest,
        size_bytes = json.len(),
        "Persisted manifest to CAS"
    );

    Ok(digest)
}

/// Create a snapshot ledger entry in the database.
///
/// Inserts a row into the snapshots table with all metadata from the manifest.
/// This function is meant to be called within a transaction.
///
/// ## Arguments
///
/// - `tx`: Database transaction
/// - `snapshot_id`: UUIDv7 identifier for this snapshot
/// - `manifest`: Snapshot manifest with metadata
/// - `parent_snapshot_id`: Optional parent snapshot for history tracking
///
/// ## Returns
///
/// Row ID of the inserted snapshot
///
/// ## Errors
///
/// - `ExErrorKind::Persistence`: Database insert failed
fn create_snapshot_ledger_entry(
    tx: &Transaction,
    snapshot_id: &str,
    manifest: &SnapshotManifest,
    parent_snapshot_id: Option<String>,
) -> Result<i64> {
    // Convert RFC3339 timestamp to Unix milliseconds
    let created_at_ms = chrono::DateTime::parse_from_rfc3339(&manifest.created_at)
        .map_err(|e| {
            ExError::new(ExErrorKind::Serialization)
                .with_op("create_snapshot_ledger_entry")
                .with_message(format!("Invalid timestamp in manifest: {}", e))
        })?
        .timestamp_millis();

    let row_id = tx
        .execute(
            r#"
            INSERT INTO snapshots (
                snapshot_id,
                root_ettle_id,
                manifest_digest,
                semantic_manifest_digest,
                created_at,
                parent_snapshot_id,
                policy_ref,
                profile_ref,
                status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
            rusqlite::params![
                snapshot_id,
                manifest.root_ettle_id,
                manifest.manifest_digest,
                manifest.semantic_manifest_digest,
                created_at_ms,
                parent_snapshot_id,
                manifest.policy_ref,
                manifest.profile_ref,
                "committed",
            ],
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("create_snapshot_ledger_entry")
                .with_message(format!("Failed to insert snapshot: {}", e))
        })?;

    tracing::debug!(
        snapshot_id = %snapshot_id,
        row_id = row_id,
        root_ettle_id = %manifest.root_ettle_id,
        "Created snapshot ledger entry"
    );

    Ok(row_id as i64)
}

/// Query for an existing snapshot by semantic digest.
///
/// Checks if a snapshot with the given semantic digest already exists.
/// Used for idempotency checks.
fn query_by_semantic_digest(
    tx: &Transaction,
    semantic_digest: &str,
) -> Result<Option<SnapshotCommitResult>> {
    let mut stmt = tx
        .prepare(
            r#"
            SELECT snapshot_id, manifest_digest, semantic_manifest_digest
            FROM snapshots
            WHERE semantic_manifest_digest = ?1
            LIMIT 1
            "#,
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("query_by_semantic_digest")
                .with_message(format!("Failed to prepare query: {}", e))
        })?;

    let result = stmt
        .query_row([semantic_digest], |row| {
            Ok(SnapshotCommitResult {
                snapshot_id: row.get(0)?,
                manifest_digest: row.get(1)?,
                semantic_manifest_digest: row.get(2)?,
                was_duplicate: true,
            })
        })
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("query_by_semantic_digest")
                .with_message(format!("Failed to query snapshot: {}", e))
        })?;

    Ok(result)
}

/// Query for the current head snapshot for a given root ettle.
///
/// Returns the most recent snapshot ID for the specified root ettle.
fn query_current_head(tx: &Transaction, root_ettle_id: &str) -> Result<Option<String>> {
    let mut stmt = tx
        .prepare(
            r#"
            SELECT snapshot_id
            FROM snapshots
            WHERE root_ettle_id = ?1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("query_current_head")
                .with_message(format!("Failed to prepare query: {}", e))
        })?;

    let result = stmt
        .query_row([root_ettle_id], |row| row.get(0))
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("query_current_head")
                .with_message(format!("Failed to query head: {}", e))
        })?;

    Ok(result)
}

/// Commit a snapshot atomically to both CAS and ledger.
///
/// This is the main entry point for snapshot persistence. It performs:
/// 1. Expected head validation (if provided)
/// 2. Idempotency check (return existing if semantic digest matches)
/// 3. Persist manifest to CAS
/// 4. Create ledger entry
/// 5. Commit transaction atomically
///
/// ## Arguments
///
/// - `conn`: Database connection
/// - `cas_store`: CAS store instance
/// - `manifest`: Snapshot manifest to commit
/// - `options`: Commit options (expected_head, dry_run)
///
/// ## Returns
///
/// `SnapshotCommitResult` with snapshot ID and digests
///
/// ## Errors
///
/// - `ExErrorKind::Concurrency`: Expected head mismatch
/// - `ExErrorKind::Persistence`: CAS or database error
/// - `ExErrorKind::Serialization`: Manifest serialization failed
///
/// ## Idempotency
///
/// If a snapshot with the same semantic digest already exists, returns the
/// existing snapshot ID without creating a duplicate.
pub fn commit_snapshot(
    conn: &mut Connection,
    cas_store: &FsStore,
    manifest: SnapshotManifest,
    options: SnapshotOptions,
) -> Result<SnapshotCommitResult> {
    // Dry-run mode: compute digests but don't persist
    if options.dry_run {
        return Ok(SnapshotCommitResult {
            snapshot_id: String::new(), // No ID in dry-run
            manifest_digest: manifest.manifest_digest.clone(),
            semantic_manifest_digest: manifest.semantic_manifest_digest.clone(),
            was_duplicate: false,
        });
    }

    let tx = conn.transaction().map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("commit_snapshot")
            .with_message(format!("Failed to start transaction: {}", e))
    })?;

    // 1. Validate expected head if provided
    let parent_snapshot_id = if let Some(expected) = &options.expected_head {
        let current = query_current_head(&tx, &manifest.root_ettle_id)?;
        if current.as_ref() != Some(expected) {
            return Err(ExError::new(ExErrorKind::Concurrency)
                .with_op("commit_snapshot")
                .with_entity_id(manifest.root_ettle_id.clone())
                .with_message(format!(
                    "Expected head '{}' but current is '{:?}'",
                    expected, current
                )));
        }
        Some(expected.clone())
    } else {
        // No expected head, but check if there's an existing head to link as parent
        query_current_head(&tx, &manifest.root_ettle_id)?
    };

    // 2. Check idempotency (semantic digest already exists?)
    if let Some(existing) = query_by_semantic_digest(&tx, &manifest.semantic_manifest_digest)? {
        tracing::debug!(
            snapshot_id = %existing.snapshot_id,
            semantic_digest = %manifest.semantic_manifest_digest,
            "Snapshot with same semantic digest already exists (idempotent)"
        );
        return Ok(existing);
    }

    // 3. Persist manifest to CAS (outside transaction, idempotent)
    // CAS computes digest of the actual JSON bytes written. We use this as the
    // official manifest_digest since it's what we can use to retrieve the manifest.
    let cas_manifest_digest = persist_manifest_to_cas(cas_store, &manifest)?;

    // 4. Generate snapshot ID (UUIDv7 for temporal ordering)
    let snapshot_id = uuid::Uuid::now_v7().to_string();

    // 5. Create modified manifest with CAS digest (for ledger storage)
    let mut manifest_for_ledger = manifest.clone();
    manifest_for_ledger.manifest_digest = cas_manifest_digest.clone();

    // 6. Create ledger entry (inside transaction)
    create_snapshot_ledger_entry(&tx, &snapshot_id, &manifest_for_ledger, parent_snapshot_id)?;

    // 7. Commit transaction
    tx.commit().map_err(|e| {
        ExError::new(ExErrorKind::Persistence)
            .with_op("commit_snapshot")
            .with_message(format!("Failed to commit transaction: {}", e))
    })?;

    Ok(SnapshotCommitResult {
        snapshot_id,
        manifest_digest: cas_manifest_digest,
        semantic_manifest_digest: manifest.semantic_manifest_digest,
        was_duplicate: false,
    })
}
