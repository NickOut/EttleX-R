//! SQLite repository implementation
//!
//! Persists Ettles and EPs from Phase 0.5 Store to SQLite

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use ettlex_core::model::{Ep, Ettle};
use rusqlite::{Connection, OptionalExtension, Transaction};

/// SQLite repository for Ettles and EPs
pub struct SqliteRepo;

impl SqliteRepo {
    /// Persist an Ettle to the database
    ///
    /// Takes an Ettle from the Store and saves it to the ettles table
    pub fn persist_ettle(conn: &Connection, ettle: &Ettle) -> Result<()> {
        conn.execute(
            "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                parent_id = excluded.parent_id,
                deleted = excluded.deleted,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata",
            rusqlite::params![
                ettle.id,
                ettle.title,
                ettle.parent_id,
                if ettle.deleted { 1 } else { 0 },
                ettle.created_at.timestamp(),
                ettle.updated_at.timestamp(),
                serde_json::to_string(&ettle.metadata).unwrap_or_else(|_| "{}".to_string()),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist an Ettle within a transaction
    pub fn persist_ettle_tx(tx: &Transaction, ettle: &Ettle) -> Result<()> {
        tx.execute(
            "INSERT INTO ettles (id, title, parent_id, deleted, created_at, updated_at, metadata)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                title = excluded.title,
                parent_id = excluded.parent_id,
                deleted = excluded.deleted,
                updated_at = excluded.updated_at,
                metadata = excluded.metadata",
            rusqlite::params![
                ettle.id,
                ettle.title,
                ettle.parent_id,
                if ettle.deleted { 1 } else { 0 },
                ettle.created_at.timestamp(),
                ettle.updated_at.timestamp(),
                serde_json::to_string(&ettle.metadata).unwrap_or_else(|_| "{}".to_string()),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist an EP to the database
    ///
    /// Takes an EP from the Store and saves it to the eps table
    pub fn persist_ep(conn: &Connection, ep: &Ep) -> Result<()> {
        // For Phase 1, we store content inline (not CAS)
        // Phase 2 will add CAS integration
        let content_inline = serde_json::json!({
            "why": ep.why,
            "what": ep.what,
            "how": ep.how,
        })
        .to_string();

        conn.execute(
            "INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                child_ettle_id = excluded.child_ettle_id,
                content_inline = excluded.content_inline,
                deleted = excluded.deleted,
                updated_at = excluded.updated_at",
            rusqlite::params![
                ep.id,
                ep.ettle_id,
                ep.ordinal,
                if ep.normative { 1 } else { 0 },
                ep.child_ettle_id,
                None::<String>, // content_digest (will use CAS in future)
                content_inline,
                if ep.deleted { 1 } else { 0 },
                ep.created_at.timestamp(),
                ep.updated_at.timestamp(),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist an EP within a transaction
    pub fn persist_ep_tx(tx: &Transaction, ep: &Ep) -> Result<()> {
        let content_inline = serde_json::json!({
            "why": ep.why,
            "what": ep.what,
            "how": ep.how,
        })
        .to_string();

        tx.execute(
            "INSERT INTO eps (id, ettle_id, ordinal, normative, child_ettle_id, content_digest, content_inline, deleted, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                child_ettle_id = excluded.child_ettle_id,
                content_inline = excluded.content_inline,
                deleted = excluded.deleted,
                updated_at = excluded.updated_at",
            rusqlite::params![
                ep.id,
                ep.ettle_id,
                ep.ordinal,
                if ep.normative { 1 } else { 0 },
                ep.child_ettle_id,
                None::<String>,
                content_inline,
                if ep.deleted { 1 } else { 0 },
                ep.created_at.timestamp(),
                ep.updated_at.timestamp(),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Get an Ettle from the database by ID
    pub fn get_ettle(conn: &Connection, ettle_id: &str) -> Result<Option<Ettle>> {
        let mut stmt = conn
            .prepare("SELECT id, title, parent_id, deleted, created_at, updated_at, metadata FROM ettles WHERE id = ?")
            .map_err(from_rusqlite)?;

        let result = stmt
            .query_row([ettle_id], |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let parent_id: Option<String> = row.get(2)?;
                let deleted: i32 = row.get(3)?;
                let created_at: i64 = row.get(4)?;
                let updated_at: i64 = row.get(5)?;
                let metadata_json: String = row.get(6)?;

                let mut ettle = Ettle::new(id, title);
                ettle.parent_id = parent_id;
                ettle.deleted = deleted != 0;
                ettle.created_at = chrono::DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(chrono::Utc::now);
                ettle.updated_at = chrono::DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or_else(chrono::Utc::now);
                ettle.metadata = serde_json::from_str(&metadata_json).unwrap_or_default();

                Ok(ettle)
            })
            .optional()
            .map_err(from_rusqlite)?;

        Ok(result)
    }

    /// Get an EP from the database by ID
    pub fn get_ep(conn: &Connection, ep_id: &str) -> Result<Option<Ep>> {
        let mut stmt = conn
            .prepare("SELECT id, ettle_id, ordinal, normative, child_ettle_id, content_inline, deleted, created_at, updated_at FROM eps WHERE id = ?")
            .map_err(from_rusqlite)?;

        let result = stmt
            .query_row([ep_id], |row| {
                let id: String = row.get(0)?;
                let ettle_id: String = row.get(1)?;
                let ordinal: u32 = row.get(2)?;
                let normative: i32 = row.get(3)?;
                let child_ettle_id: Option<String> = row.get(4)?;
                let content_inline: String = row.get(5)?;
                let deleted: i32 = row.get(6)?;
                let created_at: i64 = row.get(7)?;
                let updated_at: i64 = row.get(8)?;

                // Parse content
                let content: serde_json::Value =
                    serde_json::from_str(&content_inline).unwrap_or_default();
                let why = content["why"].as_str().unwrap_or_default().to_string();
                let what = content["what"].as_str().unwrap_or_default().to_string();
                let how = content["how"].as_str().unwrap_or_default().to_string();

                let mut ep = Ep::new(id, ettle_id, ordinal, normative != 0, why, what, how);
                ep.child_ettle_id = child_ettle_id;
                ep.deleted = deleted != 0;
                ep.created_at = chrono::DateTime::from_timestamp(created_at, 0)
                    .unwrap_or_else(chrono::Utc::now);
                ep.updated_at = chrono::DateTime::from_timestamp(updated_at, 0)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(ep)
            })
            .optional()
            .map_err(from_rusqlite)?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrations;

    fn setup_test_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrations::apply_migrations(&mut conn).unwrap();
        conn
    }

    #[test]
    fn test_persist_and_get_ettle() {
        let conn = setup_test_db();
        let ettle = Ettle::new("test-ettle-1".to_string(), "Test Ettle".to_string());

        SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

        let retrieved = SqliteRepo::get_ettle(&conn, "test-ettle-1")
            .unwrap()
            .expect("Ettle should exist");

        assert_eq!(retrieved.id, "test-ettle-1");
        assert_eq!(retrieved.title, "Test Ettle");
        assert!(!retrieved.deleted);
    }

    #[test]
    fn test_persist_and_get_ep() {
        let conn = setup_test_db();

        // Create parent Ettle first (foreign key requirement)
        let ettle = Ettle::new("test-ettle-1".to_string(), "Test Ettle".to_string());
        SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

        let ep = Ep::new(
            "test-ep-1".to_string(),
            "test-ettle-1".to_string(),
            0,
            true,
            "Why content".to_string(),
            "What content".to_string(),
            "How content".to_string(),
        );

        SqliteRepo::persist_ep(&conn, &ep).unwrap();

        let retrieved = SqliteRepo::get_ep(&conn, "test-ep-1")
            .unwrap()
            .expect("EP should exist");

        assert_eq!(retrieved.id, "test-ep-1");
        assert_eq!(retrieved.ettle_id, "test-ettle-1");
        assert_eq!(retrieved.ordinal, 0);
        assert!(retrieved.normative);
        assert_eq!(retrieved.why, "Why content");
        assert_eq!(retrieved.what, "What content");
        assert_eq!(retrieved.how, "How content");
    }

    #[test]
    fn test_persist_ettle_idempotent() {
        let conn = setup_test_db();
        let mut ettle = Ettle::new("test-ettle-2".to_string(), "Original Title".to_string());

        SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

        // Update title and persist again
        ettle.title = "Updated Title".to_string();
        SqliteRepo::persist_ettle(&conn, &ettle).unwrap();

        let retrieved = SqliteRepo::get_ettle(&conn, "test-ettle-2")
            .unwrap()
            .expect("Ettle should exist");

        assert_eq!(retrieved.title, "Updated Title");
    }
}
