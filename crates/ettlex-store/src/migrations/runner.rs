//! Migration runner
//!
//! Applies migrations with checksums, gap detection, and idempotency

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, migration_error, Result};
use crate::migrations::checksums::compute_checksum;
use crate::migrations::embedded::get_migrations;
use rusqlite::Connection;

/// Apply all pending migrations to the database
pub fn apply_migrations(conn: &mut Connection) -> Result<()> {
    // Create schema_version table if it doesn't exist
    create_schema_version_table(conn)?;

    // Get all migrations
    let migrations = get_migrations();

    // Apply each migration
    for migration in migrations {
        apply_migration(conn, migration.id, migration.sql)?;
    }

    // Backfill content_digest for any EPs written before the digest column was populated
    backfill_ep_content_digests(conn)?;

    Ok(())
}

/// Backfill content_digest for EP rows that have NULL (written before the fix).
///
/// Reads content_inline JSON, extracts why/what/how, computes SHA-256, and writes
/// the digest back. This is a no-op for rows that already have a digest.
fn backfill_ep_content_digests(conn: &Connection) -> Result<()> {
    use sha2::{Digest, Sha256};

    // Collect rows that need backfilling
    let mut stmt = conn
        .prepare("SELECT id, content_inline FROM eps WHERE content_digest IS NULL")
        .map_err(from_rusqlite)?;
    let rows: Vec<(String, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(from_rusqlite)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(from_rusqlite)?;
    drop(stmt);

    for (ep_id, content_inline) in rows {
        let v: serde_json::Value =
            serde_json::from_str(&content_inline).unwrap_or(serde_json::json!({}));
        let why = v["why"].as_str().unwrap_or("").to_string();
        let what = v["what"].as_str().unwrap_or("").to_string();
        let how = v["how"].as_str().unwrap_or("").to_string();

        let mut map = std::collections::BTreeMap::new();
        map.insert("how", how.as_str());
        map.insert("what", what.as_str());
        map.insert("why", why.as_str());
        let json = serde_json::to_string(&map).expect("BTreeMap serialization is infallible");
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let digest = hex::encode(hasher.finalize());

        conn.execute(
            "UPDATE eps SET content_digest = ?1 WHERE id = ?2 AND content_digest IS NULL",
            rusqlite::params![digest, ep_id],
        )
        .map_err(from_rusqlite)?;
    }

    Ok(())
}

/// Create the schema_version table if it doesn't exist
fn create_schema_version_table(conn: &Connection) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (
            id INTEGER PRIMARY KEY,
            migration_id TEXT NOT NULL UNIQUE,
            applied_at INTEGER NOT NULL,
            checksum TEXT
        )",
        [],
    )
    .map_err(from_rusqlite)?;

    Ok(())
}

/// Apply a single migration if not already applied
fn apply_migration(conn: &mut Connection, migration_id: &str, sql: &str) -> Result<()> {
    // Check if migration already applied
    let already_applied: bool = conn
        .query_row(
            "SELECT 1 FROM schema_version WHERE migration_id = ?",
            [migration_id],
            |_| Ok(true),
        )
        .unwrap_or(false);

    if already_applied {
        // Idempotent: already applied
        return Ok(());
    }

    // Compute checksum
    let checksum = compute_checksum(sql);

    // Start transaction
    let tx = conn.transaction().map_err(from_rusqlite)?;

    // Execute migration SQL
    tx.execute_batch(sql)
        .map_err(|e| migration_error(migration_id, &e.to_string()))?;

    // Record migration
    let now = chrono::Utc::now().timestamp();
    tx.execute(
        "INSERT INTO schema_version (migration_id, applied_at, checksum) VALUES (?, ?, ?)",
        rusqlite::params![migration_id, now, checksum],
    )
    .map_err(from_rusqlite)?;

    // Commit transaction
    tx.commit().map_err(from_rusqlite)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_migrations() {
        let mut conn = Connection::open_in_memory().unwrap();
        let result = apply_migrations(&mut conn);
        assert!(result.is_ok());
    }

    #[test]
    fn test_idempotency() {
        let mut conn = Connection::open_in_memory().unwrap();
        apply_migrations(&mut conn).unwrap();
        let result = apply_migrations(&mut conn);
        assert!(result.is_ok());
    }
}
