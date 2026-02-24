//! SQLite repository implementation
//!
//! Persists Ettles and EPs from Phase 0.5 Store to SQLite

#![allow(clippy::result_large_err)]

use crate::errors::{from_rusqlite, Result};
use ettlex_core::model::{
    Constraint, Decision, DecisionEvidenceItem, DecisionLink, Ep, EpConstraintRef, Ettle,
};
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

    /// Persist an EP-Constraint attachment record
    pub fn persist_ep_constraint_ref(
        conn: &Connection,
        ref_record: &EpConstraintRef,
    ) -> Result<()> {
        conn.execute(
            "INSERT INTO ep_constraint_refs (ep_id, constraint_id, ordinal, created_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(ep_id, constraint_id) DO UPDATE SET
                ordinal = excluded.ordinal",
            rusqlite::params![
                ref_record.ep_id,
                ref_record.constraint_id,
                ref_record.ordinal,
                ref_record.created_at.timestamp_millis(),
            ],
        )
        .map_err(from_rusqlite)?;

        Ok(())
    }

    /// List EP-Constraint attachment records for a specific EP
    pub fn list_ep_constraint_refs(conn: &Connection, ep_id: &str) -> Result<Vec<EpConstraintRef>> {
        let mut stmt = conn
            .prepare(
                "SELECT ep_id, constraint_id, ordinal, created_at
                 FROM ep_constraint_refs
                 WHERE ep_id = ?1
                 ORDER BY ordinal",
            )
            .map_err(from_rusqlite)?;

        let refs = stmt
            .query_map([ep_id], |row| {
                let ep_id: String = row.get(0)?;
                let constraint_id: String = row.get(1)?;
                let ordinal: i32 = row.get(2)?;
                let created_at_ms: i64 = row.get(3)?;

                let mut ref_record = EpConstraintRef::new(ep_id, constraint_id, ordinal);
                ref_record.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(ref_record)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(refs)
    }

    /// List all EP-Constraint attachment records
    pub fn list_all_ep_constraint_refs(conn: &Connection) -> Result<Vec<EpConstraintRef>> {
        let mut stmt = conn
            .prepare(
                "SELECT ep_id, constraint_id, ordinal, created_at
                 FROM ep_constraint_refs
                 ORDER BY ep_id, ordinal",
            )
            .map_err(from_rusqlite)?;

        let refs = stmt
            .query_map([], |row| {
                let ep_id: String = row.get(0)?;
                let constraint_id: String = row.get(1)?;
                let ordinal: i32 = row.get(2)?;
                let created_at_ms: i64 = row.get(3)?;

                let mut ref_record = EpConstraintRef::new(ep_id, constraint_id, ordinal);
                ref_record.created_at = chrono::DateTime::from_timestamp_millis(created_at_ms)
                    .unwrap_or_else(chrono::Utc::now);

                Ok(ref_record)
            })
            .map_err(from_rusqlite)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(from_rusqlite)?;

        Ok(refs)
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
