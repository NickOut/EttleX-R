//! Provenance event tracking for seed imports
//!
//! Records events in the provenance_events table

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use rusqlite::{Connection, Transaction};

/// Provenance event kind
#[derive(Debug, Clone, Copy)]
pub enum ProvenanceKind {
    /// Seed import started
    ImportStarted,
    /// Ettle created from seed
    EttleApplied,
    /// Seed import completed
    ImportCompleted,
}

impl ProvenanceKind {
    fn as_str(&self) -> &'static str {
        match self {
            ProvenanceKind::ImportStarted => "seed_import_started",
            ProvenanceKind::EttleApplied => "seed_ettle_applied",
            ProvenanceKind::ImportCompleted => "seed_import_completed",
        }
    }
}

/// Emit a provenance event
pub fn emit_event(
    conn: &Connection,
    kind: ProvenanceKind,
    correlation_id: &str,
    metadata: Option<serde_json::Value>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let metadata_str = metadata
        .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());

    conn.execute(
        "INSERT INTO provenance_events (kind, correlation_id, timestamp, metadata) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![kind.as_str(), correlation_id, now, metadata_str],
    )
    .map_err(from_rusqlite)?;

    Ok(())
}

/// Emit a provenance event within a transaction
pub fn emit_event_tx(
    tx: &Transaction,
    kind: ProvenanceKind,
    correlation_id: &str,
    metadata: Option<serde_json::Value>,
) -> Result<()> {
    let now = chrono::Utc::now().timestamp();
    let metadata_str = metadata
        .map(|m| serde_json::to_string(&m).unwrap_or_else(|_| "{}".to_string()))
        .unwrap_or_else(|| "{}".to_string());

    tx.execute(
        "INSERT INTO provenance_events (kind, correlation_id, timestamp, metadata) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![kind.as_str(), correlation_id, now, metadata_str],
    )
    .map_err(from_rusqlite)?;

    Ok(())
}

/// Emit "seed import started" event
pub fn emit_started(tx: &Transaction, seed_digest: &str) -> Result<()> {
    emit_event_tx(
        tx,
        ProvenanceKind::ImportStarted,
        seed_digest,
        Some(serde_json::json!({
            "seed_digest": seed_digest,
        })),
    )
}

/// Emit "ettle applied" event
pub fn emit_applied(tx: &Transaction, seed_digest: &str, ettle_id: &str) -> Result<()> {
    emit_event_tx(
        tx,
        ProvenanceKind::EttleApplied,
        seed_digest,
        Some(serde_json::json!({
            "ettle_id": ettle_id,
        })),
    )
}

/// Emit "seed import completed" event
pub fn emit_completed(tx: &Transaction, seed_digest: &str) -> Result<()> {
    emit_event_tx(
        tx,
        ProvenanceKind::ImportCompleted,
        seed_digest,
        Some(serde_json::json!({
            "seed_digest": seed_digest,
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;
    use uuid::Uuid;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_migrations(&mut conn).unwrap();
        conn
    }

    #[test]
    fn test_emit_event() {
        let conn = setup_test_db();
        let correlation_id = Uuid::now_v7().to_string();

        emit_event(&conn, ProvenanceKind::ImportStarted, &correlation_id, None).unwrap();

        // Verify event was recorded
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provenance_events WHERE correlation_id = ?",
                [&correlation_id],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 1);
    }

    #[test]
    fn test_emit_started_applied_completed() {
        let mut conn = setup_test_db();
        let seed_digest = "test_digest";

        let tx = conn.transaction().unwrap();

        emit_started(&tx, seed_digest).unwrap();
        emit_applied(&tx, seed_digest, "ettle-1").unwrap();
        emit_applied(&tx, seed_digest, "ettle-2").unwrap();
        emit_completed(&tx, seed_digest).unwrap();

        tx.commit().unwrap();

        // Verify 4 events were recorded
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provenance_events WHERE correlation_id = ?",
                [seed_digest],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(count, 4);
    }
}
