//! Read-only snapshot query operations.
//!
//! Provides functions for resolving snapshot references to manifest bytes
//! without mutating any state, plus structured row queries for the snapshot ledger.

#![allow(clippy::result_large_err)]

use crate::cas::FsStore;
use crate::errors::{from_rusqlite, Result};
use ettlex_core::errors::{ExError, ExErrorKind};
use rusqlite::{Connection, OptionalExtension};

/// A raw row from the `snapshots` ledger table.
#[derive(Debug, Clone)]
pub struct SnapshotRow {
    /// Unique snapshot identifier (UUIDv7)
    pub snapshot_id: String,
    /// Root ettle for this snapshot
    pub root_ettle_id: String,
    /// Full manifest digest (includes `created_at`)
    pub manifest_digest: String,
    /// Semantic manifest digest (excludes `created_at`, for idempotency)
    pub semantic_manifest_digest: String,
    /// Creation timestamp, milliseconds since epoch
    pub created_at: i64,
    /// Parent snapshot ID (for linear history)
    pub parent_snapshot_id: Option<String>,
    /// Policy reference string
    pub policy_ref: String,
    /// Profile reference string
    pub profile_ref: String,
    /// Status (`committed`, `draft`, etc.)
    pub status: String,
}

/// Fetch the manifest digest for a snapshot by its snapshot ID.
///
/// Performs a read-only SELECT on the `snapshots` table.
///
/// # Errors
///
/// - `NotFound` — no row with `snapshot_id = ?` exists in the `snapshots` table
/// - `Persistence` — SQLite query failed
pub fn fetch_snapshot_manifest_digest(conn: &Connection, snapshot_id: &str) -> Result<String> {
    let result = conn
        .query_row(
            "SELECT manifest_digest FROM snapshots WHERE snapshot_id = ?1",
            [snapshot_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("fetch_snapshot_manifest_digest")
                .with_message(e.to_string())
        })?;

    match result {
        Some(digest) => Ok(digest),
        None => Err(ExError::new(ExErrorKind::NotFound)
            .with_op("fetch_snapshot_manifest_digest")
            .with_entity_id(snapshot_id)
            .with_message("snapshot not found")),
    }
}

/// Fetch manifest bytes from the CAS store by digest.
///
/// Re-maps the CAS "not found" error to `MissingBlob` so callers can
/// distinguish between a missing snapshot row and a missing CAS blob.
///
/// # Errors
///
/// - `MissingBlob` — no CAS blob exists for the given digest
/// - `Io` — CAS read failed for another reason
pub fn fetch_manifest_bytes_by_digest(cas: &FsStore, manifest_digest: &str) -> Result<Vec<u8>> {
    cas.read(manifest_digest).map_err(|e| {
        if e.kind() == ExErrorKind::NotFound {
            ExError::new(ExErrorKind::MissingBlob)
                .with_op("fetch_manifest_bytes_by_digest")
                .with_entity_id(manifest_digest)
                .with_message("manifest blob not found in CAS")
        } else {
            e
        }
    })
}

/// Fetch a full `SnapshotRow` by snapshot ID.
///
/// # Errors
///
/// - `NotFound` — no row with the given `snapshot_id` exists
/// - `Persistence` — SQLite read failed
pub fn fetch_snapshot_row(conn: &Connection, snapshot_id: &str) -> Result<SnapshotRow> {
    conn.query_row(
        "SELECT snapshot_id, root_ettle_id, manifest_digest, semantic_manifest_digest,
                created_at, parent_snapshot_id, policy_ref, profile_ref, status
         FROM snapshots WHERE snapshot_id = ?1",
        [snapshot_id],
        row_to_snapshot_row,
    )
    .optional()
    .map_err(from_rusqlite)?
    .ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("fetch_snapshot_row")
            .with_entity_id(snapshot_id)
            .with_message("snapshot not found")
    })
}

/// List all snapshot rows, optionally filtered by root ettle ID.
///
/// Results are ordered by `created_at`, `snapshot_id` ascending.
pub fn list_snapshot_rows(conn: &Connection, ettle_id: Option<&str>) -> Result<Vec<SnapshotRow>> {
    let rows = match ettle_id {
        None => {
            let mut stmt = conn
                .prepare(
                    "SELECT snapshot_id, root_ettle_id, manifest_digest,
                            semantic_manifest_digest, created_at, parent_snapshot_id,
                            policy_ref, profile_ref, status
                     FROM snapshots
                     ORDER BY created_at, snapshot_id",
                )
                .map_err(from_rusqlite)?;
            let result: std::result::Result<Vec<_>, _> = stmt
                .query_map([], row_to_snapshot_row)
                .map_err(from_rusqlite)?
                .collect();
            result.map_err(from_rusqlite)?
        }
        Some(eid) => {
            let mut stmt = conn
                .prepare(
                    "SELECT snapshot_id, root_ettle_id, manifest_digest,
                            semantic_manifest_digest, created_at, parent_snapshot_id,
                            policy_ref, profile_ref, status
                     FROM snapshots
                     WHERE root_ettle_id = ?1
                     ORDER BY created_at, snapshot_id",
                )
                .map_err(from_rusqlite)?;
            let result: std::result::Result<Vec<_>, _> = stmt
                .query_map([eid], row_to_snapshot_row)
                .map_err(from_rusqlite)?
                .collect();
            result.map_err(from_rusqlite)?
        }
    };
    Ok(rows)
}

/// Fetch both digest values for a snapshot: `(manifest_digest, semantic_manifest_digest)`.
pub fn fetch_snapshot_digests(conn: &Connection, snapshot_id: &str) -> Result<(String, String)> {
    conn.query_row(
        "SELECT manifest_digest, semantic_manifest_digest
         FROM snapshots WHERE snapshot_id = ?1",
        [snapshot_id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    )
    .optional()
    .map_err(from_rusqlite)?
    .ok_or_else(|| {
        ExError::new(ExErrorKind::NotFound)
            .with_op("fetch_snapshot_digests")
            .with_entity_id(snapshot_id)
            .with_message("snapshot not found")
    })
}

/// Fetch the most recently committed snapshot row (by `created_at DESC`).
///
/// Returns `None` if no snapshots exist yet.
pub fn fetch_head_snapshot(conn: &Connection) -> Result<Option<SnapshotRow>> {
    conn.query_row(
        "SELECT snapshot_id, root_ettle_id, manifest_digest, semantic_manifest_digest,
                created_at, parent_snapshot_id, policy_ref, profile_ref, status
         FROM snapshots
         ORDER BY created_at DESC, snapshot_id DESC
         LIMIT 1",
        [],
        row_to_snapshot_row,
    )
    .optional()
    .map_err(from_rusqlite)
}

fn row_to_snapshot_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<SnapshotRow> {
    Ok(SnapshotRow {
        snapshot_id: row.get(0)?,
        root_ettle_id: row.get(1)?,
        manifest_digest: row.get(2)?,
        semantic_manifest_digest: row.get(3)?,
        created_at: row.get(4)?,
        parent_snapshot_id: row.get(5)?,
        policy_ref: row.get(6)?,
        profile_ref: row.get(7)?,
        status: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::migrations::apply_migrations(&mut conn).unwrap();
        conn
    }

    fn insert_snapshot(conn: &Connection, id: &str, ettle_id: &str) {
        conn.execute(
            "INSERT INTO snapshots
             (snapshot_id, root_ettle_id, manifest_digest, semantic_manifest_digest,
              created_at, parent_snapshot_id, policy_ref, profile_ref, status)
             VALUES (?1, ?2, 'md', 'smd', 0, NULL, 'pol', 'prof', 'committed')",
            rusqlite::params![id, ettle_id],
        )
        .unwrap();
    }

    #[test]
    fn test_fetch_snapshot_row_found() {
        let conn = setup();
        insert_snapshot(&conn, "snap:1", "ettle:root");
        let row = fetch_snapshot_row(&conn, "snap:1").unwrap();
        assert_eq!(row.snapshot_id, "snap:1");
        assert_eq!(row.root_ettle_id, "ettle:root");
        assert_eq!(row.policy_ref, "pol");
        assert_eq!(row.status, "committed");
    }

    #[test]
    fn test_fetch_snapshot_row_not_found() {
        let conn = setup();
        let err = fetch_snapshot_row(&conn, "nonexistent").unwrap_err();
        assert_eq!(err.kind(), ettlex_core::errors::ExErrorKind::NotFound);
    }

    #[test]
    fn test_list_snapshot_rows_no_filter() {
        let conn = setup();
        insert_snapshot(&conn, "snap:a", "ettle:r1");
        insert_snapshot(&conn, "snap:b", "ettle:r2");
        let rows = list_snapshot_rows(&conn, None).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_list_snapshot_rows_with_ettle_filter() {
        let conn = setup();
        insert_snapshot(&conn, "snap:a", "ettle:r1");
        insert_snapshot(&conn, "snap:b", "ettle:r2");
        let rows = list_snapshot_rows(&conn, Some("ettle:r1")).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].snapshot_id, "snap:a");
    }

    #[test]
    fn test_fetch_snapshot_digests_found() {
        let conn = setup();
        insert_snapshot(&conn, "snap:d", "ettle:r");
        let (md, smd) = fetch_snapshot_digests(&conn, "snap:d").unwrap();
        assert_eq!(md, "md");
        assert_eq!(smd, "smd");
    }

    #[test]
    fn test_fetch_snapshot_digests_not_found() {
        let conn = setup();
        let err = fetch_snapshot_digests(&conn, "nonexistent").unwrap_err();
        assert_eq!(err.kind(), ettlex_core::errors::ExErrorKind::NotFound);
    }

    #[test]
    fn test_fetch_head_snapshot_empty() {
        let conn = setup();
        let result = fetch_head_snapshot(&conn).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_fetch_head_snapshot_returns_latest() {
        let conn = setup();
        // Insert two snapshots with different timestamps
        conn.execute(
            "INSERT INTO snapshots
             (snapshot_id, root_ettle_id, manifest_digest, semantic_manifest_digest,
              created_at, parent_snapshot_id, policy_ref, profile_ref, status)
             VALUES ('snap:old', 'ettle:r', 'md1', 'smd1', 100, NULL, 'pol', 'prof', 'committed')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO snapshots
             (snapshot_id, root_ettle_id, manifest_digest, semantic_manifest_digest,
              created_at, parent_snapshot_id, policy_ref, profile_ref, status)
             VALUES ('snap:new', 'ettle:r', 'md2', 'smd2', 200, 'snap:old', 'pol', 'prof', 'committed')",
            [],
        )
        .unwrap();
        let head = fetch_head_snapshot(&conn).unwrap().unwrap();
        assert_eq!(head.snapshot_id, "snap:new");
    }
}
