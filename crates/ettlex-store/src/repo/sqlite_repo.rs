//! SQLite repository implementation
//!
//! Persists Ettles and EPs from Phase 0.5 Store to SQLite

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use crate::model::{
    EttleCursor, EttleListItem, EttleListOpts, EttleListPage, EttleRecord, GroupMemberRecord,
    GroupRecord, RelationListOpts, RelationRecord, RelationTypeEntry,
};
use base64::Engine as _;
use ettlex_core::errors::{ExError, ExErrorKind};
use ettlex_core::model::{Constraint, Decision, DecisionEvidenceItem, DecisionLink, Ettle};
use rusqlite::{Connection, OptionalExtension, Transaction};

/// SQLite repository for Ettles and relations.
pub struct SqliteRepo;

impl SqliteRepo {
    // -------------------------------------------------------------------------
    // Ettle CRUD functions
    // -------------------------------------------------------------------------

    /// Insert a new Ettle using the v2 schema columns.
    #[allow(clippy::too_many_arguments)]
    pub fn insert_ettle(
        conn: &Connection,
        id: &str,
        title: &str,
        why: &str,
        what: &str,
        how: &str,
        reasoning_link_id: Option<&str>,
        reasoning_link_type: Option<&str>,
        created_at: &str,
        updated_at: &str,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO ettles (id, title, why, what, how, reasoning_link_id, reasoning_link_type, created_at, updated_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                id,
                title,
                why,
                what,
                how,
                reasoning_link_id,
                reasoning_link_type,
                created_at,
                updated_at,
            ],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Get an Ettle record (v2) by ID.
    /// strings without a type-mismatch error.
    pub fn get_ettle_record(conn: &Connection, ettle_id: &str) -> Result<Option<EttleRecord>> {
        let result = conn
            .query_row(
                "SELECT id, title, why, what, how, reasoning_link_id, reasoning_link_type, \
                 created_at, updated_at, tombstoned_at \
                 FROM ettles WHERE id = ?1",
                [ettle_id],
                |row| {
                    Ok(EttleRecord {
                        id: row.get(0)?,
                        title: row.get(1)?,
                        why: row.get(2)?,
                        what: row.get(3)?,
                        how: row.get(4)?,
                        reasoning_link_id: row.get(5)?,
                        reasoning_link_type: row.get(6)?,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                        tombstoned_at: row.get(9)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    /// List Ettles using cursor-based pagination on (created_at, id).
    pub fn list_ettles(conn: &Connection, opts: &EttleListOpts) -> Result<EttleListPage> {
        // Fetch limit+1 rows so we can detect if there's a next page
        let fetch_limit = opts.limit as i64 + 1;

        fn map_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<EttleListItem> {
            Ok(EttleListItem {
                id: row.get(0)?,
                title: row.get(1)?,
                tombstoned_at: row.get(2)?,
            })
        }

        let rows: Vec<EttleListItem> = match (&opts.cursor, opts.include_tombstoned) {
            (Some(c), true) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, title, tombstoned_at FROM ettles \
                         WHERE (created_at > ?1 OR (created_at = ?1 AND id > ?2)) \
                         ORDER BY created_at, id LIMIT ?3",
                    )
                    .map_err(from_rusqlite)?;
                let rows = stmt
                    .query_map(rusqlite::params![c.created_at, c.id, fetch_limit], map_row)
                    .map_err(from_rusqlite)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(from_rusqlite)?;
                rows
            }
            (Some(c), false) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, title, tombstoned_at FROM ettles \
                         WHERE tombstoned_at IS NULL \
                         AND (created_at > ?1 OR (created_at = ?1 AND id > ?2)) \
                         ORDER BY created_at, id LIMIT ?3",
                    )
                    .map_err(from_rusqlite)?;
                let rows = stmt
                    .query_map(rusqlite::params![c.created_at, c.id, fetch_limit], map_row)
                    .map_err(from_rusqlite)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(from_rusqlite)?;
                rows
            }
            (None, true) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, title, tombstoned_at FROM ettles \
                         ORDER BY created_at, id LIMIT ?1",
                    )
                    .map_err(from_rusqlite)?;
                let rows = stmt
                    .query_map([fetch_limit], map_row)
                    .map_err(from_rusqlite)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(from_rusqlite)?;
                rows
            }
            (None, false) => {
                let mut stmt = conn
                    .prepare(
                        "SELECT id, title, tombstoned_at FROM ettles \
                         WHERE tombstoned_at IS NULL \
                         ORDER BY created_at, id LIMIT ?1",
                    )
                    .map_err(from_rusqlite)?;
                let rows = stmt
                    .query_map([fetch_limit], map_row)
                    .map_err(from_rusqlite)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(from_rusqlite)?;
                rows
            }
        };

        // Detect next page
        let has_more = rows.len() > opts.limit as usize;
        let items: Vec<EttleListItem> = rows.into_iter().take(opts.limit as usize).collect();

        let next_cursor = if has_more {
            // Cursor is based on the last item returned
            if let Some(last) = items.last() {
                // Fetch created_at for the last item to build the cursor.
                // CAST to TEXT so that rows seeded with INTEGER epoch values are
                // handled without a rusqlite type mismatch.
                let created_at: String = conn
                    .query_row(
                        "SELECT CAST(created_at AS TEXT) FROM ettles WHERE id = ?1",
                        [&last.id],
                        |r| r.get(0),
                    )
                    .map_err(from_rusqlite)?;
                let raw = format!("{},{}", created_at, last.id);
                Some(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw.as_bytes()))
            } else {
                None
            }
        } else {
            None
        };

        Ok(EttleListPage { items, next_cursor })
    }

    /// Decode a base64 cursor string into an `EttleCursor`.
    pub fn decode_ettle_cursor(encoded: &str) -> Result<EttleCursor> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded.as_bytes())
            .map_err(|e| {
                ExError::new(ExErrorKind::InvalidInput)
                    .with_op("decode_ettle_cursor")
                    .with_message(format!("invalid cursor base64: {}", e))
            })?;
        let s = String::from_utf8(bytes).map_err(|e| {
            ExError::new(ExErrorKind::InvalidInput)
                .with_op("decode_ettle_cursor")
                .with_message(format!("cursor not valid UTF-8: {}", e))
        })?;
        // Split on the LAST comma to handle created_at values that might contain commas
        let comma_pos = s.rfind(',').ok_or_else(|| {
            ExError::new(ExErrorKind::InvalidInput)
                .with_op("decode_ettle_cursor")
                .with_message("cursor missing comma separator")
        })?;
        let created_at = s[..comma_pos].to_string();
        let id = s[comma_pos + 1..].to_string();
        Ok(EttleCursor { created_at, id })
    }

    /// Update an existing Ettle's content fields.
    ///
    /// Only `Some` fields are written; `None` means "preserve existing value".
    /// For `reasoning_link_id` / `reasoning_link_type`, `Some(None)` clears the field.
    ///
    /// This implementation fetches the current record, applies the patches, and writes
    /// all fields in a single UPDATE to avoid dynamic SQL generation.
    #[allow(clippy::too_many_arguments)]
    pub fn update_ettle(
        conn: &Connection,
        id: &str,
        title: Option<&str>,
        why: Option<&str>,
        what: Option<&str>,
        how: Option<&str>,
        reasoning_link_id: Option<Option<&str>>,
        reasoning_link_type: Option<Option<&str>>,
        updated_at: &str,
    ) -> Result<()> {
        // Read current record to apply partial updates
        let current = SqliteRepo::get_ettle_record(conn, id)?.ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_op("update_ettle")
                .with_entity_id(id)
                .with_message(format!("Ettle not found: {}", id))
        })?;

        let new_title = title.unwrap_or(&current.title).to_string();
        let new_why = why.unwrap_or(&current.why).to_string();
        let new_what = what.unwrap_or(&current.what).to_string();
        let new_how = how.unwrap_or(&current.how).to_string();
        let new_link_id: Option<String> = match reasoning_link_id {
            Some(Some(v)) => Some(v.to_string()),
            Some(None) => None,                        // Clear
            None => current.reasoning_link_id.clone(), // Preserve
        };
        let new_link_type: Option<String> = match reasoning_link_type {
            Some(Some(v)) => Some(v.to_string()),
            Some(None) => None,                          // Clear
            None => current.reasoning_link_type.clone(), // Preserve
        };

        conn.execute(
            "UPDATE ettles SET title = ?1, why = ?2, what = ?3, how = ?4, \
             reasoning_link_id = ?5, reasoning_link_type = ?6, updated_at = ?7 \
             WHERE id = ?8",
            rusqlite::params![
                new_title,
                new_why,
                new_what,
                new_how,
                new_link_id,
                new_link_type,
                updated_at,
                id,
            ],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Set `tombstoned_at` on an Ettle.
    pub fn tombstone_ettle(conn: &Connection, id: &str, tombstoned_at: &str) -> Result<()> {
        conn.execute(
            "UPDATE ettles SET tombstoned_at = ?1 WHERE id = ?2",
            rusqlite::params![tombstoned_at, id],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Count active (non-tombstoned) Ettles that have `reasoning_link_id = ettle_id`.
    pub fn get_active_ettle_dependants_count(conn: &Connection, ettle_id: &str) -> Result<u64> {
        let count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ettles WHERE reasoning_link_id = ?1 AND tombstoned_at IS NULL",
                [ettle_id],
                |r| r.get(0),
            )
            .map_err(from_rusqlite)?;
        Ok(count)
    }

    /// Persist a Constraint to the database
    ///
    /// Takes a Constraint from the Store and saves it to the constraints table
    pub fn persist_constraint(conn: &Connection, constraint: &Constraint) -> Result<()> {
        let deleted_at_ms = constraint.deleted_at.map(|dt| dt.timestamp_millis());

        conn.execute(
            "INSERT INTO constraints (constraint_id, family, kind, scope, payload_json, payload_digest, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(constraint_id) DO UPDATE SET
                family = excluded.family,
                kind = excluded.kind,
                scope = excluded.scope,
                payload_json = excluded.payload_json,
                payload_digest = excluded.payload_digest,
                updated_at = excluded.updated_at,
                deleted_at = excluded.deleted_at",
            rusqlite::params![
                constraint.constraint_id,
                constraint.family,
                constraint.kind,
                constraint.scope,
                constraint.payload_json.to_string(),
                constraint.payload_digest,
                constraint.created_at.timestamp_millis(),
                constraint.updated_at.timestamp_millis(),
                deleted_at_ms,
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist a Constraint within a transaction
    pub fn persist_constraint_tx(tx: &Transaction, constraint: &Constraint) -> Result<()> {
        let deleted_at_ms = constraint.deleted_at.map(|dt| dt.timestamp_millis());

        tx.execute(
            "INSERT INTO constraints (constraint_id, family, kind, scope, payload_json, payload_digest, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(constraint_id) DO UPDATE SET
                family = excluded.family,
                kind = excluded.kind,
                scope = excluded.scope,
                payload_json = excluded.payload_json,
                payload_digest = excluded.payload_digest,
                updated_at = excluded.updated_at,
                deleted_at = excluded.deleted_at",
            rusqlite::params![
                constraint.constraint_id,
                constraint.family,
                constraint.kind,
                constraint.scope,
                constraint.payload_json.to_string(),
                constraint.payload_digest,
                constraint.created_at.timestamp_millis(),
                constraint.updated_at.timestamp_millis(),
                deleted_at_ms,
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Get a Constraint by ID
    pub fn get_constraint(conn: &Connection, constraint_id: &str) -> Result<Option<Constraint>> {
        let result = conn
            .query_row(
                "SELECT constraint_id, family, kind, scope, payload_json, payload_digest, created_at, updated_at, deleted_at
                 FROM constraints
                 WHERE constraint_id = ?1",
                [constraint_id],
                |row| {
                    let constraint_id: String = row.get(0)?;
                    let family: String = row.get(1)?;
                    let kind: String = row.get(2)?;
                    let scope: String = row.get(3)?;
                    let payload_json_str: String = row.get(4)?;
                    let payload_digest: String = row.get(5)?;
                    let created_at_ms: i64 = row.get(6)?;
                    let updated_at_ms: i64 = row.get(7)?;
                    let deleted_at_ms: Option<i64> = row.get(8)?;

                    let payload_json: serde_json::Value =
                        serde_json::from_str(&payload_json_str).unwrap_or(serde_json::json!({}));

                    let mut constraint = Constraint::new(
                        constraint_id,
                        family,
                        kind,
                        scope,
                        payload_json,
                    );

                    // Override computed digest with stored one (for historical records)
                    constraint.payload_digest = payload_digest;

                    // Set timestamps from DB
                    constraint.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                        .unwrap_or_else(chrono::Utc::now);
                    constraint.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                        .unwrap_or_else(chrono::Utc::now);
                    constraint.deleted_at =
                        deleted_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                    Ok(constraint)
                },
            )
            .optional()
            .map_err(from_rusqlite)?;

        Ok(result)
    }

    /// List all Constraints
    pub fn list_constraints(conn: &Connection) -> Result<Vec<Constraint>> {
        let mut stmt = conn
            .prepare(
                "SELECT constraint_id, family, kind, scope, payload_json, payload_digest, created_at, updated_at, deleted_at
                 FROM constraints
                 ORDER BY constraint_id",
            )
            .map_err(from_rusqlite)?;

        let constraints = stmt
            .query_map([], |row| {
                let constraint_id: String = row.get(0)?;
                let family: String = row.get(1)?;
                let kind: String = row.get(2)?;
                let scope: String = row.get(3)?;
                let payload_json_str: String = row.get(4)?;
                let payload_digest: String = row.get(5)?;
                let created_at_ms: i64 = row.get(6)?;
                let updated_at_ms: i64 = row.get(7)?;
                let deleted_at_ms: Option<i64> = row.get(8)?;

                let payload_json: serde_json::Value =
                    serde_json::from_str(&payload_json_str).unwrap_or(serde_json::json!({}));

                let mut constraint =
                    Constraint::new(constraint_id, family, kind, scope, payload_json);

                constraint.payload_digest = payload_digest;
                constraint.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                constraint.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                constraint.deleted_at =
                    deleted_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(constraint)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(constraints)
    }

    /// Persist a Decision to the database
    ///
    /// Takes a Decision from the Store and saves it to the decisions table
    pub fn persist_decision(conn: &Connection, decision: &Decision) -> Result<()> {
        let tombstoned_at_ms = decision.tombstoned_at.map(|dt| dt.timestamp_millis());

        conn.execute(
            "INSERT INTO decisions (decision_id, title, status, decision_text, rationale, alternatives_text, consequences_text, evidence_kind, evidence_excerpt, evidence_capture_id, evidence_file_path, evidence_hash, created_at, updated_at, tombstoned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(decision_id) DO UPDATE SET
                title = excluded.title,
                status = excluded.status,
                decision_text = excluded.decision_text,
                rationale = excluded.rationale,
                alternatives_text = excluded.alternatives_text,
                consequences_text = excluded.consequences_text,
                evidence_kind = excluded.evidence_kind,
                evidence_excerpt = excluded.evidence_excerpt,
                evidence_capture_id = excluded.evidence_capture_id,
                evidence_file_path = excluded.evidence_file_path,
                evidence_hash = excluded.evidence_hash,
                updated_at = excluded.updated_at,
                tombstoned_at = excluded.tombstoned_at",
            rusqlite::params![
                decision.decision_id,
                decision.title,
                decision.status,
                decision.decision_text,
                decision.rationale,
                decision.alternatives_text,
                decision.consequences_text,
                decision.evidence_kind,
                decision.evidence_excerpt,
                decision.evidence_capture_id,
                decision.evidence_file_path,
                decision.evidence_hash,
                decision.created_at.timestamp_millis(),
                decision.updated_at.timestamp_millis(),
                tombstoned_at_ms,
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist a Decision within a transaction
    pub fn persist_decision_tx(tx: &Transaction, decision: &Decision) -> Result<()> {
        let tombstoned_at_ms = decision.tombstoned_at.map(|dt| dt.timestamp_millis());

        tx.execute(
            "INSERT INTO decisions (decision_id, title, status, decision_text, rationale, alternatives_text, consequences_text, evidence_kind, evidence_excerpt, evidence_capture_id, evidence_file_path, evidence_hash, created_at, updated_at, tombstoned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
             ON CONFLICT(decision_id) DO UPDATE SET
                title = excluded.title,
                status = excluded.status,
                decision_text = excluded.decision_text,
                rationale = excluded.rationale,
                alternatives_text = excluded.alternatives_text,
                consequences_text = excluded.consequences_text,
                evidence_kind = excluded.evidence_kind,
                evidence_excerpt = excluded.evidence_excerpt,
                evidence_capture_id = excluded.evidence_capture_id,
                evidence_file_path = excluded.evidence_file_path,
                evidence_hash = excluded.evidence_hash,
                updated_at = excluded.updated_at,
                tombstoned_at = excluded.tombstoned_at",
            rusqlite::params![
                decision.decision_id,
                decision.title,
                decision.status,
                decision.decision_text,
                decision.rationale,
                decision.alternatives_text,
                decision.consequences_text,
                decision.evidence_kind,
                decision.evidence_excerpt,
                decision.evidence_capture_id,
                decision.evidence_file_path,
                decision.evidence_hash,
                decision.created_at.timestamp_millis(),
                decision.updated_at.timestamp_millis(),
                tombstoned_at_ms,
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist a Decision Evidence Item to the database
    pub fn persist_evidence_item(conn: &Connection, item: &DecisionEvidenceItem) -> Result<()> {
        conn.execute(
            "INSERT INTO decision_evidence_items (evidence_capture_id, source, content, content_hash, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(evidence_capture_id) DO UPDATE SET
                source = excluded.source,
                content = excluded.content,
                content_hash = excluded.content_hash",
            rusqlite::params![
                item.evidence_capture_id,
                item.source,
                item.content,
                item.content_hash,
                item.created_at.timestamp_millis(),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Persist a Decision Link to the database
    pub fn persist_decision_link(conn: &Connection, link: &DecisionLink) -> Result<()> {
        let tombstoned_at_ms = link.tombstoned_at.map(|dt| dt.timestamp_millis());

        conn.execute(
            "INSERT INTO decision_links (decision_id, target_kind, target_id, relation_kind, ordinal, created_at, tombstoned_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(decision_id, target_kind, target_id, relation_kind) DO UPDATE SET
                ordinal = excluded.ordinal,
                tombstoned_at = excluded.tombstoned_at",
            rusqlite::params![
                link.decision_id,
                link.target_kind,
                link.target_id,
                link.relation_kind,
                link.ordinal,
                link.created_at.timestamp_millis(),
                tombstoned_at_ms,
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// Get a Decision by ID
    pub fn get_decision(conn: &Connection, decision_id: &str) -> Result<Option<Decision>> {
        let result = conn
            .query_row(
                "SELECT decision_id, title, status, decision_text, rationale, alternatives_text, consequences_text, evidence_kind, evidence_excerpt, evidence_capture_id, evidence_file_path, evidence_hash, created_at, updated_at, tombstoned_at
                 FROM decisions
                 WHERE decision_id = ?1",
                [decision_id],
                |row| {
                    let decision_id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let status: String = row.get(2)?;
                    let decision_text: String = row.get(3)?;
                    let rationale: String = row.get(4)?;
                    let alternatives_text: Option<String> = row.get(5)?;
                    let consequences_text: Option<String> = row.get(6)?;
                    let evidence_kind: String = row.get(7)?;
                    let evidence_excerpt: Option<String> = row.get(8)?;
                    let evidence_capture_id: Option<String> = row.get(9)?;
                    let evidence_file_path: Option<String> = row.get(10)?;
                    let evidence_hash: String = row.get(11)?;
                    let created_at_ms: i64 = row.get(12)?;
                    let updated_at_ms: i64 = row.get(13)?;
                    let tombstoned_at_ms: Option<i64> = row.get(14)?;

                    let mut decision = Decision::new(
                        decision_id,
                        title,
                        status,
                        decision_text,
                        rationale,
                        alternatives_text,
                        consequences_text,
                        evidence_kind,
                        evidence_excerpt,
                        evidence_capture_id,
                        evidence_file_path,
                    );

                    decision.evidence_hash = evidence_hash;
                    decision.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                        .unwrap_or_else(chrono::Utc::now);
                    decision.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                        .unwrap_or_else(chrono::Utc::now);
                    decision.tombstoned_at =
                        tombstoned_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                    Ok(decision)
                },
            )
            .optional()
            .map_err(from_rusqlite)?;

        Ok(result)
    }

    /// List all Decisions
    pub fn list_decisions(conn: &Connection) -> Result<Vec<Decision>> {
        let mut stmt = conn
            .prepare(
                "SELECT decision_id, title, status, decision_text, rationale, alternatives_text, consequences_text, evidence_kind, evidence_excerpt, evidence_capture_id, evidence_file_path, evidence_hash, created_at, updated_at, tombstoned_at
                 FROM decisions
                 ORDER BY created_at, decision_id",
            )
            .map_err(from_rusqlite)?;

        let decisions = stmt
            .query_map([], |row| {
                let decision_id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let status: String = row.get(2)?;
                let decision_text: String = row.get(3)?;
                let rationale: String = row.get(4)?;
                let alternatives_text: Option<String> = row.get(5)?;
                let consequences_text: Option<String> = row.get(6)?;
                let evidence_kind: String = row.get(7)?;
                let evidence_excerpt: Option<String> = row.get(8)?;
                let evidence_capture_id: Option<String> = row.get(9)?;
                let evidence_file_path: Option<String> = row.get(10)?;
                let evidence_hash: String = row.get(11)?;
                let created_at_ms: i64 = row.get(12)?;
                let updated_at_ms: i64 = row.get(13)?;
                let tombstoned_at_ms: Option<i64> = row.get(14)?;

                let mut decision = Decision::new(
                    decision_id,
                    title,
                    status,
                    decision_text,
                    rationale,
                    alternatives_text,
                    consequences_text,
                    evidence_kind,
                    evidence_excerpt,
                    evidence_capture_id,
                    evidence_file_path,
                );

                decision.evidence_hash = evidence_hash;
                decision.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                decision.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                decision.tombstoned_at =
                    tombstoned_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(decision)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(decisions)
    }

    /// List Decision Links for a specific target
    pub fn list_decision_links(
        conn: &Connection,
        target_kind: &str,
        target_id: &str,
    ) -> Result<Vec<DecisionLink>> {
        let mut stmt = conn
            .prepare(
                "SELECT decision_id, target_kind, target_id, relation_kind, ordinal, created_at, tombstoned_at
                 FROM decision_links
                 WHERE target_kind = ?1 AND target_id = ?2
                 ORDER BY ordinal, relation_kind, decision_id",
            )
            .map_err(from_rusqlite)?;

        let links = stmt
            .query_map([target_kind, target_id], |row| {
                let decision_id: String = row.get(0)?;
                let target_kind: String = row.get(1)?;
                let target_id: String = row.get(2)?;
                let relation_kind: String = row.get(3)?;
                let ordinal: i32 = row.get(4)?;
                let created_at_ms: i64 = row.get(5)?;
                let tombstoned_at_ms: Option<i64> = row.get(6)?;

                let mut link =
                    DecisionLink::new(decision_id, target_kind, target_id, relation_kind, ordinal);
                link.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                link.tombstoned_at =
                    tombstoned_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(link)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(links)
    }

    /// List all Decision Links
    pub fn list_all_decision_links(conn: &Connection) -> Result<Vec<DecisionLink>> {
        let mut stmt = conn
            .prepare(
                "SELECT decision_id, target_kind, target_id, relation_kind, ordinal, created_at, tombstoned_at
                 FROM decision_links
                 ORDER BY decision_id, target_kind, target_id, relation_kind",
            )
            .map_err(from_rusqlite)?;

        let links = stmt
            .query_map([], |row| {
                let decision_id: String = row.get(0)?;
                let target_kind: String = row.get(1)?;
                let target_id: String = row.get(2)?;
                let relation_kind: String = row.get(3)?;
                let ordinal: i32 = row.get(4)?;
                let created_at_ms: i64 = row.get(5)?;
                let tombstoned_at_ms: Option<i64> = row.get(6)?;

                let mut link =
                    DecisionLink::new(decision_id, target_kind, target_id, relation_kind, ordinal);
                link.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                link.tombstoned_at =
                    tombstoned_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(link)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(links)
    }

    /// List all Decision Evidence Items
    pub fn list_all_evidence_items(conn: &Connection) -> Result<Vec<DecisionEvidenceItem>> {
        let mut stmt = conn
            .prepare(
                "SELECT evidence_capture_id, source, content, content_hash, created_at
                 FROM decision_evidence_items
                 ORDER BY evidence_capture_id",
            )
            .map_err(from_rusqlite)?;

        let items = stmt
            .query_map([], |row| {
                let evidence_capture_id: String = row.get(0)?;
                let source: String = row.get(1)?;
                let content: String = row.get(2)?;
                let content_hash: String = row.get(3)?;
                let created_at_ms: i64 = row.get(4)?;

                let mut item = DecisionEvidenceItem::new(evidence_capture_id, source, content);
                item.content_hash = content_hash;
                item.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(item)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(items)
    }

    /// Get an Ettle from the database by ID (current schema: id, title, created_at, updated_at).
    pub fn get_ettle(conn: &Connection, ettle_id: &str) -> Result<Option<Ettle>> {
        let result = conn
            .query_row(
                "SELECT id, title, created_at, updated_at FROM ettles WHERE id = ?1",
                [ettle_id],
                |row| {
                    let id: String = row.get(0)?;
                    let title: String = row.get(1)?;
                    let created_at_str: String = row.get(2)?;
                    let updated_at_str: String = row.get(3)?;

                    let mut ettle = Ettle::new(id, title);
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&created_at_str) {
                        ettle.created_at = ts.with_timezone(&chrono::Utc);
                    }
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&updated_at_str) {
                        ettle.updated_at = ts.with_timezone(&chrono::Utc);
                    }

                    Ok(ettle)
                },
            )
            .optional()
            .map_err(from_rusqlite)?;

        Ok(result)
    }

    /// List Ettles with optional prefix filter and cursor-based pagination.
    ///
    /// Returns up to `limit` Ettles whose `id` is lexicographically greater than
    /// `after_id` (exclusive), optionally filtered to IDs that start with `prefix_filter`.
    /// Results are ordered by `id` ascending.
    pub fn list_ettles_paginated(
        conn: &Connection,
        prefix_filter: Option<&str>,
        after_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Ettle>> {
        // Build query dynamically based on optional filters
        let mut conditions: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(after) = after_id {
            conditions.push("id > ?".to_string());
            params.push(Box::new(after.to_string()));
        }

        if let Some(prefix) = prefix_filter {
            conditions.push("id LIKE ? ESCAPE '\\'".to_string());
            // Escape special LIKE characters and append %
            let escaped = prefix
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            params.push(Box::new(format!("{}%", escaped)));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT id, title, created_at, updated_at
             FROM ettles
             {}
             ORDER BY id
             LIMIT {}",
            where_clause, limit
        );

        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let ettles = stmt
            .query_map(param_refs.as_slice(), |row| {
                let id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let created_at_str: String = row.get(2)?;
                let updated_at_str: String = row.get(3)?;

                let mut ettle = Ettle::new(id, title);
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&created_at_str) {
                    ettle.created_at = ts.with_timezone(&chrono::Utc);
                }
                if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(&updated_at_str) {
                    ettle.updated_at = ts.with_timezone(&chrono::Utc);
                }

                Ok(ettle)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(ettles)
    }

    /// List Constraints by family, with optional tombstone filter.
    ///
    /// If `include_tombstoned` is false, only constraints where `deleted_at IS NULL`
    /// are returned.
    pub fn list_constraints_by_family(
        conn: &Connection,
        family: &str,
        include_tombstoned: bool,
    ) -> Result<Vec<Constraint>> {
        let sql = if include_tombstoned {
            "SELECT constraint_id, family, kind, scope, payload_json, payload_digest,
                    created_at, updated_at, deleted_at
             FROM constraints
             WHERE family = ?1
             ORDER BY constraint_id"
                .to_string()
        } else {
            "SELECT constraint_id, family, kind, scope, payload_json, payload_digest,
                    created_at, updated_at, deleted_at
             FROM constraints
             WHERE family = ?1 AND deleted_at IS NULL
             ORDER BY constraint_id"
                .to_string()
        };

        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        Self::query_constraints(&mut stmt, [family])
    }

    /// List Decisions linked to a target entity, with optional tombstone filter.
    pub fn list_decisions_by_target(
        conn: &Connection,
        target_kind: &str,
        target_id: &str,
        include_tombstoned: bool,
    ) -> Result<Vec<Decision>> {
        let sql = if include_tombstoned {
            "SELECT d.decision_id, d.title, d.status, d.decision_text, d.rationale,
                    d.alternatives_text, d.consequences_text, d.evidence_kind,
                    d.evidence_excerpt, d.evidence_capture_id, d.evidence_file_path,
                    d.evidence_hash, d.created_at, d.updated_at, d.tombstoned_at
             FROM decisions d
             JOIN decision_links l ON l.decision_id = d.decision_id
             WHERE l.target_kind = ?1 AND l.target_id = ?2 AND l.tombstoned_at IS NULL
             GROUP BY d.decision_id
             ORDER BY d.created_at, d.decision_id"
                .to_string()
        } else {
            "SELECT d.decision_id, d.title, d.status, d.decision_text, d.rationale,
                    d.alternatives_text, d.consequences_text, d.evidence_kind,
                    d.evidence_excerpt, d.evidence_capture_id, d.evidence_file_path,
                    d.evidence_hash, d.created_at, d.updated_at, d.tombstoned_at
             FROM decisions d
             JOIN decision_links l ON l.decision_id = d.decision_id
             WHERE l.target_kind = ?1 AND l.target_id = ?2
               AND l.tombstoned_at IS NULL AND d.tombstoned_at IS NULL
             GROUP BY d.decision_id
             ORDER BY d.created_at, d.decision_id"
                .to_string()
        };

        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        Self::query_decisions(&mut stmt, rusqlite::params![target_kind, target_id])
    }

    /// List Decisions with cursor-based pagination.
    ///
    /// `after_key` is `(created_at_ms, decision_id)` exclusive lower bound.
    pub fn list_decisions_paginated(
        conn: &Connection,
        after_key: Option<(i64, &str)>,
        limit: usize,
    ) -> Result<Vec<Decision>> {
        let sql = match after_key {
            None => format!(
                "SELECT decision_id, title, status, decision_text, rationale,
                        alternatives_text, consequences_text, evidence_kind,
                        evidence_excerpt, evidence_capture_id, evidence_file_path,
                        evidence_hash, created_at, updated_at, tombstoned_at
                 FROM decisions
                 ORDER BY created_at, decision_id
                 LIMIT {}",
                limit
            ),
            Some(_) => format!(
                "SELECT decision_id, title, status, decision_text, rationale,
                        alternatives_text, consequences_text, evidence_kind,
                        evidence_excerpt, evidence_capture_id, evidence_file_path,
                        evidence_hash, created_at, updated_at, tombstoned_at
                 FROM decisions
                 WHERE (created_at > ?1) OR (created_at = ?1 AND decision_id > ?2)
                 ORDER BY created_at, decision_id
                 LIMIT {}",
                limit
            ),
        };

        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;

        match after_key {
            None => Self::query_decisions(&mut stmt, []),
            Some((ts, id)) => Self::query_decisions(&mut stmt, rusqlite::params![ts, id]),
        }
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    fn query_constraints<P: rusqlite::Params>(
        stmt: &mut rusqlite::Statement<'_>,
        params: P,
    ) -> Result<Vec<Constraint>> {
        let constraints = stmt
            .query_map(params, |row| {
                let constraint_id: String = row.get(0)?;
                let family: String = row.get(1)?;
                let kind: String = row.get(2)?;
                let scope: String = row.get(3)?;
                let payload_json_str: String = row.get(4)?;
                let payload_digest: String = row.get(5)?;
                let created_at_ms: i64 = row.get(6)?;
                let updated_at_ms: i64 = row.get(7)?;
                let deleted_at_ms: Option<i64> = row.get(8)?;

                let payload_json: serde_json::Value =
                    serde_json::from_str(&payload_json_str).unwrap_or(serde_json::json!({}));

                let mut constraint =
                    Constraint::new(constraint_id, family, kind, scope, payload_json);
                constraint.payload_digest = payload_digest;
                constraint.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                constraint.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                constraint.deleted_at =
                    deleted_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(constraint)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(constraints)
    }

    fn query_decisions<P: rusqlite::Params>(
        stmt: &mut rusqlite::Statement<'_>,
        params: P,
    ) -> Result<Vec<Decision>> {
        let decisions = stmt
            .query_map(params, |row| {
                let decision_id: String = row.get(0)?;
                let title: String = row.get(1)?;
                let status: String = row.get(2)?;
                let decision_text: String = row.get(3)?;
                let rationale: String = row.get(4)?;
                let alternatives_text: Option<String> = row.get(5)?;
                let consequences_text: Option<String> = row.get(6)?;
                let evidence_kind: String = row.get(7)?;
                let evidence_excerpt: Option<String> = row.get(8)?;
                let evidence_capture_id: Option<String> = row.get(9)?;
                let evidence_file_path: Option<String> = row.get(10)?;
                let evidence_hash: String = row.get(11)?;
                let created_at_ms: i64 = row.get(12)?;
                let updated_at_ms: i64 = row.get(13)?;
                let tombstoned_at_ms: Option<i64> = row.get(14)?;

                let mut decision = Decision::new(
                    decision_id,
                    title,
                    status,
                    decision_text,
                    rationale,
                    alternatives_text,
                    consequences_text,
                    evidence_kind,
                    evidence_excerpt,
                    evidence_capture_id,
                    evidence_file_path,
                );
                decision.evidence_hash = evidence_hash;
                decision.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                decision.updated_at = chrono::DateTime::from_timestamp_millis(updated_at_ms)
                    .unwrap_or_else(chrono::Utc::now);
                decision.tombstoned_at =
                    tombstoned_at_ms.and_then(chrono::DateTime::from_timestamp_millis);

                Ok(decision)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(decisions)
    }

    // =========================================================================
    // Relation Type Registry (Slice 02)
    // =========================================================================

    /// Get a relation type registry entry by relation_type.
    pub fn get_relation_type_entry(
        conn: &Connection,
        relation_type: &str,
    ) -> Result<Option<RelationTypeEntry>> {
        let result = conn
            .query_row(
                "SELECT relation_type, properties_json, created_at, tombstoned_at \
                 FROM relation_type_registry WHERE relation_type = ?1",
                [relation_type],
                |row| {
                    Ok(RelationTypeEntry {
                        relation_type: row.get(0)?,
                        properties_json: row.get(1)?,
                        created_at: row.get(2)?,
                        tombstoned_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    // =========================================================================
    // Relations (Slice 02)
    // =========================================================================

    /// Insert a new relation row.
    pub fn insert_relation(conn: &Connection, record: &RelationRecord) -> Result<()> {
        conn.execute(
            "INSERT INTO relations (id, source_ettle_id, target_ettle_id, relation_type, \
             properties_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                record.id,
                record.source_ettle_id,
                record.target_ettle_id,
                record.relation_type,
                record.properties_json,
                record.created_at,
            ],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Get a relation by ID.
    pub fn get_relation(conn: &Connection, id: &str) -> Result<Option<RelationRecord>> {
        let result = conn
            .query_row(
                "SELECT id, source_ettle_id, target_ettle_id, relation_type, \
                 properties_json, created_at, tombstoned_at \
                 FROM relations WHERE id = ?1",
                [id],
                |row| {
                    Ok(RelationRecord {
                        id: row.get(0)?,
                        source_ettle_id: row.get(1)?,
                        target_ettle_id: row.get(2)?,
                        relation_type: row.get(3)?,
                        properties_json: row.get(4)?,
                        created_at: row.get(5)?,
                        tombstoned_at: row.get(6)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    /// List relations with optional filters, sorted by (created_at ASC, id ASC).
    pub fn list_relations(
        conn: &Connection,
        opts: &RelationListOpts,
    ) -> Result<Vec<RelationRecord>> {
        let mut clauses: Vec<String> = Vec::new();
        if !opts.include_tombstoned {
            clauses.push("tombstoned_at IS NULL".to_string());
        }
        if let Some(src) = &opts.source_ettle_id {
            clauses.push(format!("source_ettle_id = '{}'", src.replace('\'', "''")));
        }
        if let Some(tgt) = &opts.target_ettle_id {
            clauses.push(format!("target_ettle_id = '{}'", tgt.replace('\'', "''")));
        }
        if let Some(rt) = &opts.relation_type {
            clauses.push(format!("relation_type = '{}'", rt.replace('\'', "''")));
        }

        let where_clause = if clauses.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", clauses.join(" AND "))
        };

        let sql = format!(
            "SELECT id, source_ettle_id, target_ettle_id, relation_type, \
             properties_json, created_at, tombstoned_at \
             FROM relations {} ORDER BY created_at ASC, id ASC",
            where_clause
        );

        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(RelationRecord {
                    id: row.get(0)?,
                    source_ettle_id: row.get(1)?,
                    target_ettle_id: row.get(2)?,
                    relation_type: row.get(3)?,
                    properties_json: row.get(4)?,
                    created_at: row.get(5)?,
                    tombstoned_at: row.get(6)?,
                })
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(rows)
    }

    /// Update a relation's properties_json. Returns false if not found.
    pub fn update_relation_properties(
        conn: &Connection,
        id: &str,
        properties_json: &str,
    ) -> Result<bool> {
        let rows_changed = conn
            .execute(
                "UPDATE relations SET properties_json = ?1 WHERE id = ?2",
                rusqlite::params![properties_json, id],
            )
            .map_err(from_rusqlite)?;
        Ok(rows_changed > 0)
    }

    /// Tombstone a relation. Returns false if not found.
    pub fn tombstone_relation(conn: &Connection, id: &str, tombstoned_at: &str) -> Result<bool> {
        let rows_changed = conn
            .execute(
                "UPDATE relations SET tombstoned_at = ?1 WHERE id = ?2",
                rusqlite::params![tombstoned_at, id],
            )
            .map_err(from_rusqlite)?;
        Ok(rows_changed > 0)
    }

    /// Count active outgoing constraint relations from a source ettle.
    pub fn count_active_outgoing_constraint_relations(
        conn: &Connection,
        source_ettle_id: &str,
    ) -> Result<u64> {
        let count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM relations \
                 WHERE source_ettle_id = ?1 AND relation_type = 'constraint' \
                 AND tombstoned_at IS NULL",
                [source_ettle_id],
                |r| r.get(0),
            )
            .map_err(from_rusqlite)?;
        Ok(count)
    }

    /// Get target_ettle_ids of active outgoing relations of a given type from source.
    pub fn get_active_outgoing_relations_of_type(
        conn: &Connection,
        source_ettle_id: &str,
        relation_type: &str,
    ) -> Result<Vec<String>> {
        let mut stmt = conn
            .prepare(
                "SELECT target_ettle_id FROM relations \
                 WHERE source_ettle_id = ?1 AND relation_type = ?2 AND tombstoned_at IS NULL",
            )
            .map_err(from_rusqlite)?;
        let rows = stmt
            .query_map(rusqlite::params![source_ettle_id, relation_type], |row| {
                row.get(0)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<String>, _>>()
            .map_err(from_rusqlite)?;
        Ok(rows)
    }

    // =========================================================================
    // Groups (Slice 02)
    // =========================================================================

    /// Insert a new group row.
    pub fn insert_group(conn: &Connection, record: &GroupRecord) -> Result<()> {
        conn.execute(
            "INSERT INTO groups (id, name, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![record.id, record.name, record.created_at],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Get a group by ID.
    pub fn get_group(conn: &Connection, id: &str) -> Result<Option<GroupRecord>> {
        let result = conn
            .query_row(
                "SELECT id, name, created_at, tombstoned_at FROM groups WHERE id = ?1",
                [id],
                |row| {
                    Ok(GroupRecord {
                        id: row.get(0)?,
                        name: row.get(1)?,
                        created_at: row.get(2)?,
                        tombstoned_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    /// List groups sorted by created_at ASC.
    pub fn list_groups(conn: &Connection, include_tombstoned: bool) -> Result<Vec<GroupRecord>> {
        let sql = if include_tombstoned {
            "SELECT id, name, created_at, tombstoned_at FROM groups ORDER BY created_at ASC, id ASC"
        } else {
            "SELECT id, name, created_at, tombstoned_at FROM groups \
             WHERE tombstoned_at IS NULL ORDER BY created_at ASC, id ASC"
        };
        let mut stmt = conn.prepare(sql).map_err(from_rusqlite)?;
        let rows = stmt
            .query_map([], |row| {
                Ok(GroupRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    tombstoned_at: row.get(3)?,
                })
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;
        Ok(rows)
    }

    /// Tombstone a group. Returns false if not found.
    pub fn tombstone_group(conn: &Connection, id: &str, tombstoned_at: &str) -> Result<bool> {
        let rows_changed = conn
            .execute(
                "UPDATE groups SET tombstoned_at = ?1 WHERE id = ?2",
                rusqlite::params![tombstoned_at, id],
            )
            .map_err(from_rusqlite)?;
        Ok(rows_changed > 0)
    }

    /// Count active (non-tombstoned) members in a group.
    pub fn count_active_group_members(conn: &Connection, group_id: &str) -> Result<u64> {
        let count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM group_members WHERE group_id = ?1 AND tombstoned_at IS NULL",
                [group_id],
                |r| r.get(0),
            )
            .map_err(from_rusqlite)?;
        Ok(count)
    }

    // =========================================================================
    // Group Members (Slice 02)
    // =========================================================================

    /// Insert a new group member row.
    pub fn insert_group_member(conn: &Connection, record: &GroupMemberRecord) -> Result<()> {
        conn.execute(
            "INSERT INTO group_members (id, group_id, ettle_id, created_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                record.id,
                record.group_id,
                record.ettle_id,
                record.created_at
            ],
        )
        .map_err(from_rusqlite)?;
        Ok(())
    }

    /// Get the active group member record for a (group_id, ettle_id) pair.
    pub fn get_active_group_member(
        conn: &Connection,
        group_id: &str,
        ettle_id: &str,
    ) -> Result<Option<GroupMemberRecord>> {
        let result = conn
            .query_row(
                "SELECT id, group_id, ettle_id, created_at, tombstoned_at \
                 FROM group_members \
                 WHERE group_id = ?1 AND ettle_id = ?2 AND tombstoned_at IS NULL",
                rusqlite::params![group_id, ettle_id],
                |row| {
                    Ok(GroupMemberRecord {
                        id: row.get(0)?,
                        group_id: row.get(1)?,
                        ettle_id: row.get(2)?,
                        created_at: row.get(3)?,
                        tombstoned_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    /// Get a group member record by its ID (regardless of tombstone status).
    pub fn get_group_member_by_id(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<GroupMemberRecord>> {
        let result = conn
            .query_row(
                "SELECT id, group_id, ettle_id, created_at, tombstoned_at \
                 FROM group_members WHERE id = ?1",
                [id],
                |row| {
                    Ok(GroupMemberRecord {
                        id: row.get(0)?,
                        group_id: row.get(1)?,
                        ettle_id: row.get(2)?,
                        created_at: row.get(3)?,
                        tombstoned_at: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)?;
        Ok(result)
    }

    /// List members of a group sorted by created_at ASC.
    pub fn list_group_members(
        conn: &Connection,
        group_id: &str,
        include_tombstoned: bool,
    ) -> Result<Vec<GroupMemberRecord>> {
        let sql = if include_tombstoned {
            "SELECT id, group_id, ettle_id, created_at, tombstoned_at \
             FROM group_members WHERE group_id = ?1 ORDER BY created_at ASC, id ASC"
        } else {
            "SELECT id, group_id, ettle_id, created_at, tombstoned_at \
             FROM group_members WHERE group_id = ?1 AND tombstoned_at IS NULL \
             ORDER BY created_at ASC, id ASC"
        };
        let mut stmt = conn.prepare(sql).map_err(from_rusqlite)?;
        let rows = stmt
            .query_map([group_id], |row| {
                Ok(GroupMemberRecord {
                    id: row.get(0)?,
                    group_id: row.get(1)?,
                    ettle_id: row.get(2)?,
                    created_at: row.get(3)?,
                    tombstoned_at: row.get(4)?,
                })
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;
        Ok(rows)
    }

    /// Tombstone a group member by ID. Returns false if not found.
    pub fn tombstone_group_member(
        conn: &Connection,
        member_id: &str,
        tombstoned_at: &str,
    ) -> Result<bool> {
        let rows_changed = conn
            .execute(
                "UPDATE group_members SET tombstoned_at = ?1 WHERE id = ?2",
                rusqlite::params![tombstoned_at, member_id],
            )
            .map_err(from_rusqlite)?;
        Ok(rows_changed > 0)
    }

    /// Get all active groups that contain the given ettle_id.
    pub fn get_active_groups_for_ettle(
        conn: &Connection,
        ettle_id: &str,
    ) -> Result<Vec<GroupRecord>> {
        let mut stmt = conn
            .prepare(
                "SELECT g.id, g.name, g.created_at, g.tombstoned_at \
                 FROM groups g \
                 INNER JOIN group_members gm ON g.id = gm.group_id \
                 WHERE gm.ettle_id = ?1 AND gm.tombstoned_at IS NULL AND g.tombstoned_at IS NULL \
                 ORDER BY g.created_at ASC, g.id ASC",
            )
            .map_err(from_rusqlite)?;
        let rows = stmt
            .query_map([ettle_id], |row| {
                Ok(GroupRecord {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    created_at: row.get(2)?,
                    tombstoned_at: row.get(3)?,
                })
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;
        Ok(rows)
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
    fn test_get_ettle_not_found() {
        let conn = setup_test_db();
        let result = SqliteRepo::get_ettle(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }
}
