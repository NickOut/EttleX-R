//! Profile query helpers and SqliteApprovalRouter.

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use rusqlite::{Connection, OptionalExtension};

use crate::errors::{from_rusqlite, Result};

/// A raw row from the `approval_requests` table.
#[derive(Debug, Clone)]
pub struct ApprovalRow {
    /// Unique approval token (UUIDv7)
    pub approval_token: String,
    /// Reason code for the approval request
    pub reason_code: String,
    /// JSON-encoded candidate set
    pub candidate_set_json: String,
    /// Deterministic semantic digest over `reason_code` + sorted candidate IDs
    pub semantic_request_digest: String,
    /// Current status (`pending`, `approved`, `rejected`)
    pub status: String,
    /// Creation timestamp, milliseconds since epoch
    pub created_at: i64,
    /// CAS digest for the full request payload blob (added in migration 007)
    pub request_digest: Option<String>,
}

/// Load a profile's payload JSON from the profiles table.
///
/// Returns `None` if no row with the given ref exists.
pub fn load_profile_payload(
    conn: &Connection,
    profile_ref: &str,
) -> Result<Option<serde_json::Value>> {
    let payload: Option<String> = conn
        .query_row(
            "SELECT payload_json FROM profiles WHERE profile_ref = ?1",
            [profile_ref],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("load_profile_payload")
                .with_message(format!("DB error: {}", e))
        })?;

    match payload {
        None => Ok(None),
        Some(s) => {
            let val: serde_json::Value = serde_json::from_str(&s).map_err(|e| {
                ExError::new(ExErrorKind::Serialization)
                    .with_op("load_profile_payload")
                    .with_message(format!("Invalid profile JSON: {}", e))
            })?;
            Ok(Some(val))
        }
    }
}

/// Approval router backed by SQLite (writes to `approval_requests` table and CAS).
///
/// When `cas` is provided (post-migration-007), the full request payload JSON is
/// written to CAS and the resulting digest is stored in `request_digest`.
pub struct SqliteApprovalRouter<'a> {
    conn: std::cell::UnsafeCell<*mut Connection>,
    cas: Option<&'a crate::cas::FsStore>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

// SAFETY: SqliteApprovalRouter is used only in single-threaded test contexts.
// The connection pointer outlives self, and we never share across threads.
unsafe impl Send for SqliteApprovalRouter<'_> {}
unsafe impl Sync for SqliteApprovalRouter<'_> {}

impl<'a> SqliteApprovalRouter<'a> {
    /// Create a router that persists approval requests via `conn`.
    ///
    /// # Safety
    /// Caller must ensure `conn` outlives the router and is not
    /// accessed concurrently.
    pub fn new(conn: &'a mut Connection) -> Self {
        Self {
            conn: std::cell::UnsafeCell::new(conn as *mut Connection),
            cas: None,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a router with CAS backing for `request_digest` storage (migration 007+).
    ///
    /// # Safety
    /// Caller must ensure `conn` and `cas` outlive the router and are not
    /// accessed concurrently.
    pub fn new_with_cas(conn: &'a mut Connection, cas: &'a crate::cas::FsStore) -> Self {
        Self {
            conn: std::cell::UnsafeCell::new(conn as *mut Connection),
            cas: Some(cas),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl ApprovalRouter for SqliteApprovalRouter<'_> {
    fn route_approval_request(
        &self,
        reason_code: &str,
        candidate_set: Vec<String>,
    ) -> std::result::Result<String, ExError> {
        let conn = unsafe { &mut **self.conn.get() };

        let token = uuid::Uuid::now_v7().to_string();
        let candidate_json = serde_json::to_string(&candidate_set).map_err(|e| {
            ExError::new(ExErrorKind::Serialization)
                .with_message(format!("Failed to serialize candidate_set: {}", e))
        })?;

        // Compute a deterministic digest over reason_code + sorted candidate_ids.
        let mut sorted = candidate_set.clone();
        sorted.sort();
        let digest_input = format!("{}:{}", reason_code, sorted.join(","));
        let semantic_digest = sha2_hex(digest_input.as_bytes());

        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        // Write full payload to CAS if available (migration 007+)
        let request_digest: Option<String> = if let Some(cas) = self.cas {
            let payload = serde_json::json!({
                "approval_token": token,
                "reason_code": reason_code,
                "candidate_set_json": candidate_json,
                "semantic_request_digest": semantic_digest,
                "created_at": now_ms,
            });
            let payload_bytes = serde_json::to_string(&payload).map_err(|e| {
                ExError::new(ExErrorKind::Serialization)
                    .with_message(format!("Failed to serialize approval payload: {}", e))
            })?;
            let digest = cas.write(payload_bytes.as_bytes(), "json").map_err(|e| {
                ExError::new(ExErrorKind::Persistence)
                    .with_op("route_approval_request")
                    .with_message(format!("Failed to write approval payload to CAS: {}", e))
            })?;
            Some(digest)
        } else {
            None
        };

        conn.execute(
            r#"INSERT INTO approval_requests
               (approval_token, reason_code, candidate_set_json, semantic_request_digest,
                status, created_at, request_digest)
               VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6)"#,
            rusqlite::params![
                token,
                reason_code,
                candidate_json,
                semantic_digest,
                now_ms,
                request_digest
            ],
        )
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("route_approval_request")
                .with_message(format!("Failed to insert approval request: {}", e))
        })?;

        Ok(token)
    }
}

/// Get the semantic_request_digest for an approval token.
pub fn get_approval_semantic_digest(
    conn: &Connection,
    approval_token: &str,
) -> Result<Option<String>> {
    let digest: Option<String> = conn
        .query_row(
            "SELECT semantic_request_digest FROM approval_requests WHERE approval_token = ?1",
            [approval_token],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("get_approval_semantic_digest")
                .with_message(format!("DB error: {}", e))
        })?;
    Ok(digest)
}

fn sha2_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Load a profile's full metadata: `(profile_ref, sha256_of_payload, payload_json)`.
///
/// Returns `None` if no row with the given `profile_ref` exists.
pub fn load_profile_full(
    conn: &Connection,
    profile_ref: &str,
) -> Result<Option<(String, String, serde_json::Value)>> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT profile_ref, payload_json FROM profiles WHERE profile_ref = ?1",
            [profile_ref],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("load_profile_full")
                .with_message(format!("DB error: {}", e))
        })?;

    match row {
        None => Ok(None),
        Some((pref, payload_str)) => {
            let val: serde_json::Value = serde_json::from_str(&payload_str).map_err(|e| {
                ExError::new(ExErrorKind::Serialization)
                    .with_op("load_profile_full")
                    .with_message(format!("Invalid profile JSON: {}", e))
            })?;
            let digest = sha2_hex(payload_str.as_bytes());
            Ok(Some((pref, digest, val)))
        }
    }
}

/// Load the default profile: `(profile_ref, sha256_of_payload, payload_json)`.
///
/// Returns `None` if no profile is marked `is_default = 1`.
pub fn load_default_profile(
    conn: &Connection,
) -> Result<Option<(String, String, serde_json::Value)>> {
    let row: Option<(String, String)> = conn
        .query_row(
            "SELECT profile_ref, payload_json FROM profiles WHERE is_default = 1 LIMIT 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|e| {
            ExError::new(ExErrorKind::Persistence)
                .with_op("load_default_profile")
                .with_message(format!("DB error: {}", e))
        })?;

    match row {
        None => Ok(None),
        Some((pref, payload_str)) => {
            let val: serde_json::Value = serde_json::from_str(&payload_str).map_err(|e| {
                ExError::new(ExErrorKind::Serialization)
                    .with_op("load_default_profile")
                    .with_message(format!("Invalid profile JSON: {}", e))
            })?;
            let digest = sha2_hex(payload_str.as_bytes());
            Ok(Some((pref, digest, val)))
        }
    }
}

/// List profiles with cursor-based pagination, ordered by `profile_ref`.
///
/// Returns up to `limit` profiles whose `profile_ref` is lexicographically
/// greater than `after_ref` (exclusive).
pub fn list_profiles_paginated(
    conn: &Connection,
    after_ref: Option<&str>,
    limit: usize,
) -> Result<Vec<(String, String, serde_json::Value)>> {
    let raw: Vec<(String, String)> = if let Some(after) = after_ref {
        let sql = format!(
            "SELECT profile_ref, payload_json FROM profiles
             WHERE profile_ref > ?1
             ORDER BY profile_ref LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<(String, String)>, _> = stmt
            .query_map([after], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)?
    } else {
        let sql = format!(
            "SELECT profile_ref, payload_json FROM profiles ORDER BY profile_ref LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<(String, String)>, _> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)?
    };

    raw.into_iter()
        .map(|(pref, payload_str)| {
            let val: serde_json::Value =
                serde_json::from_str(&payload_str).unwrap_or(serde_json::json!({}));
            let digest = sha2_hex(payload_str.as_bytes());
            Ok((pref, digest, val))
        })
        .collect()
}

/// Fetch an approval request row by token.
///
/// Returns `None` if no row with the given `approval_token` exists.
pub fn fetch_approval_row(conn: &Connection, approval_token: &str) -> Result<Option<ApprovalRow>> {
    // Try with request_digest column (post-migration-007); fall back gracefully
    let row = conn
        .query_row(
            "SELECT approval_token, reason_code, candidate_set_json,
                    semantic_request_digest, status, created_at, request_digest
             FROM approval_requests
             WHERE approval_token = ?1",
            [approval_token],
            |row| {
                Ok(ApprovalRow {
                    approval_token: row.get(0)?,
                    reason_code: row.get(1)?,
                    candidate_set_json: row.get(2)?,
                    semantic_request_digest: row.get(3)?,
                    status: row.get(4)?,
                    created_at: row.get(5)?,
                    request_digest: row.get(6)?,
                })
            },
        )
        .optional();

    match row {
        Ok(opt) => Ok(opt),
        Err(rusqlite::Error::InvalidColumnName(_)) => {
            // Migration 007 not yet applied — query without request_digest
            conn.query_row(
                "SELECT approval_token, reason_code, candidate_set_json,
                        semantic_request_digest, status, created_at
                 FROM approval_requests
                 WHERE approval_token = ?1",
                [approval_token],
                |row| {
                    Ok(ApprovalRow {
                        approval_token: row.get(0)?,
                        reason_code: row.get(1)?,
                        candidate_set_json: row.get(2)?,
                        semantic_request_digest: row.get(3)?,
                        status: row.get(4)?,
                        created_at: row.get(5)?,
                        request_digest: None,
                    })
                },
            )
            .optional()
            .map_err(from_rusqlite)
        }
        Err(e) => Err(from_rusqlite(e)),
    }
}

/// List approval rows with cursor-based pagination, ordered by `(created_at, approval_token)`.
///
/// `after_key` is `(created_at_ms, approval_token)` exclusive lower bound.
/// Gracefully falls back to the pre-migration-007 schema if `request_digest` column is absent.
pub fn list_approval_rows_paginated(
    conn: &Connection,
    after_key: Option<(i64, &str)>,
    limit: usize,
) -> Result<Vec<ApprovalRow>> {
    // Try with request_digest column first (post-migration-007).
    // On InvalidColumnName fall back to without.
    let with_digest = query_approval_rows_with_digest(conn, after_key, limit);
    match with_digest {
        Ok(rows) => return Ok(rows),
        Err(ref e) if e.kind() == ExErrorKind::InvalidInput => {
            // InvalidColumnName from rusqlite gets mapped here — fall through
        }
        Err(e) => return Err(e),
    }
    query_approval_rows_no_digest(conn, after_key, limit)
}

fn query_approval_rows_with_digest(
    conn: &Connection,
    after_key: Option<(i64, &str)>,
    limit: usize,
) -> Result<Vec<ApprovalRow>> {
    if let Some((ts, tok)) = after_key {
        let sql = format!(
            "SELECT approval_token, reason_code, candidate_set_json,
                    semantic_request_digest, status, created_at, request_digest
             FROM approval_requests
             WHERE (created_at > ?1) OR (created_at = ?1 AND approval_token > ?2)
             ORDER BY created_at, approval_token LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<ApprovalRow>, _> = stmt
            .query_map(rusqlite::params![ts, tok], approval_row_with_digest)
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)
    } else {
        let sql = format!(
            "SELECT approval_token, reason_code, candidate_set_json,
                    semantic_request_digest, status, created_at, request_digest
             FROM approval_requests
             ORDER BY created_at, approval_token LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<ApprovalRow>, _> = stmt
            .query_map([], approval_row_with_digest)
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)
    }
}

fn query_approval_rows_no_digest(
    conn: &Connection,
    after_key: Option<(i64, &str)>,
    limit: usize,
) -> Result<Vec<ApprovalRow>> {
    if let Some((ts, tok)) = after_key {
        let sql = format!(
            "SELECT approval_token, reason_code, candidate_set_json,
                    semantic_request_digest, status, created_at
             FROM approval_requests
             WHERE (created_at > ?1) OR (created_at = ?1 AND approval_token > ?2)
             ORDER BY created_at, approval_token LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<ApprovalRow>, _> = stmt
            .query_map(rusqlite::params![ts, tok], approval_row_no_digest)
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)
    } else {
        let sql = format!(
            "SELECT approval_token, reason_code, candidate_set_json,
                    semantic_request_digest, status, created_at
             FROM approval_requests
             ORDER BY created_at, approval_token LIMIT {}",
            limit
        );
        let mut stmt = conn.prepare(&sql).map_err(from_rusqlite)?;
        let collected: std::result::Result<Vec<ApprovalRow>, _> = stmt
            .query_map([], approval_row_no_digest)
            .map_err(from_rusqlite)?
            .collect();
        collected.map_err(from_rusqlite)
    }
}

fn approval_row_with_digest(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRow> {
    Ok(ApprovalRow {
        approval_token: row.get(0)?,
        reason_code: row.get(1)?,
        candidate_set_json: row.get(2)?,
        semantic_request_digest: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        request_digest: row.get(6)?,
    })
}

fn approval_row_no_digest(row: &rusqlite::Row<'_>) -> rusqlite::Result<ApprovalRow> {
    Ok(ApprovalRow {
        approval_token: row.get(0)?,
        reason_code: row.get(1)?,
        candidate_set_json: row.get(2)?,
        semantic_request_digest: row.get(3)?,
        status: row.get(4)?,
        created_at: row.get(5)?,
        request_digest: None,
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

    fn insert_profile(conn: &Connection, profile_ref: &str, is_default: bool, payload: &str) {
        conn.execute(
            "INSERT INTO profiles (profile_ref, payload_json, is_default, created_at)
             VALUES (?1, ?2, ?3, 0)",
            rusqlite::params![profile_ref, payload, if is_default { 1 } else { 0 }],
        )
        .unwrap();
    }

    fn insert_approval(conn: &Connection, token: &str, reason: &str) {
        conn.execute(
            "INSERT INTO approval_requests
             (approval_token, reason_code, candidate_set_json, semantic_request_digest,
              status, created_at)
             VALUES (?1, ?2, '[]', 'semdig', 'pending', 0)",
            rusqlite::params![token, reason],
        )
        .unwrap();
    }

    // ── load_profile_full ──────────────────────────────────────────────────

    #[test]
    fn test_load_profile_full_found() {
        let conn = setup();
        insert_profile(&conn, "prof/a@1", false, r#"{"x": 1}"#);
        let result = load_profile_full(&conn, "prof/a@1").unwrap();
        assert!(result.is_some());
        let (pref, digest, val) = result.unwrap();
        assert_eq!(pref, "prof/a@1");
        assert!(!digest.is_empty());
        assert_eq!(val["x"], 1);
    }

    #[test]
    fn test_load_profile_full_not_found() {
        let conn = setup();
        let result = load_profile_full(&conn, "missing@0").unwrap();
        assert!(result.is_none());
    }

    // ── load_default_profile ──────────────────────────────────────────────

    #[test]
    fn test_load_default_profile_found() {
        let conn = setup();
        insert_profile(&conn, "prof/default@0", true, r#"{"default": true}"#);
        let result = load_default_profile(&conn).unwrap();
        assert!(result.is_some());
        let (pref, _, val) = result.unwrap();
        assert_eq!(pref, "prof/default@0");
        assert_eq!(val["default"], true);
    }

    #[test]
    fn test_load_default_profile_none() {
        let conn = setup();
        insert_profile(&conn, "prof/non-default@0", false, r#"{"x": 0}"#);
        let result = load_default_profile(&conn).unwrap();
        assert!(result.is_none());
    }

    // ── list_profiles_paginated ───────────────────────────────────────────

    #[test]
    fn test_list_profiles_paginated_basic() {
        let conn = setup();
        insert_profile(&conn, "prof/a@0", false, "{}");
        insert_profile(&conn, "prof/b@0", false, "{}");
        insert_profile(&conn, "prof/c@0", false, "{}");
        let rows = list_profiles_paginated(&conn, None, 10).unwrap();
        assert_eq!(rows.len(), 3);
        let refs: Vec<_> = rows.iter().map(|(r, _, _)| r.clone()).collect();
        assert_eq!(refs, vec!["prof/a@0", "prof/b@0", "prof/c@0"]);
    }

    #[test]
    fn test_list_profiles_paginated_with_cursor() {
        let conn = setup();
        for i in 0..5 {
            insert_profile(&conn, &format!("prof/p{:02}@0", i), false, "{}");
        }
        let page1 = list_profiles_paginated(&conn, None, 2).unwrap();
        assert_eq!(page1.len(), 2);
        let after = page1.last().map(|(r, _, _)| r.clone()).unwrap();
        let page2 = list_profiles_paginated(&conn, Some(&after), 2).unwrap();
        assert_eq!(page2.len(), 2);
        // Pages must be disjoint
        for (r, _, _) in &page1 {
            assert!(!page2.iter().any(|(r2, _, _)| r2 == r));
        }
    }

    // ── fetch_approval_row ────────────────────────────────────────────────

    #[test]
    fn test_fetch_approval_row_found() {
        let conn = setup();
        insert_approval(&conn, "token-abc", "reason_x");
        let row = fetch_approval_row(&conn, "token-abc").unwrap();
        assert!(row.is_some());
        let r = row.unwrap();
        assert_eq!(r.approval_token, "token-abc");
        assert_eq!(r.reason_code, "reason_x");
        assert_eq!(r.status, "pending");
        assert!(r.request_digest.is_none());
    }

    #[test]
    fn test_fetch_approval_row_not_found() {
        let conn = setup();
        let row = fetch_approval_row(&conn, "nonexistent").unwrap();
        assert!(row.is_none());
    }

    // ── list_approval_rows_paginated ──────────────────────────────────────

    #[test]
    fn test_list_approval_rows_paginated_basic() {
        let conn = setup();
        insert_approval(&conn, "t1", "r1");
        insert_approval(&conn, "t2", "r2");
        let rows = list_approval_rows_paginated(&conn, None, 10).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_list_approval_rows_paginated_cursor() {
        let conn = setup();
        for i in 0..4 {
            conn.execute(
                "INSERT INTO approval_requests
                 (approval_token, reason_code, candidate_set_json, semantic_request_digest,
                  status, created_at)
                 VALUES (?1, 'r', '[]', 'sd', 'pending', ?2)",
                rusqlite::params![format!("tok-{}", i), i as i64],
            )
            .unwrap();
        }
        let page1 = list_approval_rows_paginated(&conn, None, 2).unwrap();
        assert_eq!(page1.len(), 2);
        let after_ts = page1.last().unwrap().created_at;
        let after_tok = page1.last().unwrap().approval_token.clone();
        let page2 = list_approval_rows_paginated(&conn, Some((after_ts, &after_tok)), 2).unwrap();
        assert_eq!(page2.len(), 2);
        // Pages must be disjoint
        let toks1: Vec<_> = page1.iter().map(|r| r.approval_token.clone()).collect();
        let toks2: Vec<_> = page2.iter().map(|r| r.approval_token.clone()).collect();
        for t in &toks1 {
            assert!(!toks2.contains(t));
        }
    }

    // ── query_approval_rows_no_digest (fallback path) ─────────────────────

    #[test]
    fn test_query_approval_rows_no_digest_fallback() {
        let conn = setup();
        insert_approval(&conn, "tok-fallback", "reason");
        // Call the no-digest function directly to exercise the fallback path
        let rows = query_approval_rows_no_digest(&conn, None, 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].approval_token, "tok-fallback");
        assert!(rows[0].request_digest.is_none());
    }

    #[test]
    fn test_query_approval_rows_no_digest_with_cursor() {
        let conn = setup();
        for i in 0..4 {
            conn.execute(
                "INSERT INTO approval_requests
                 (approval_token, reason_code, candidate_set_json, semantic_request_digest,
                  status, created_at)
                 VALUES (?1, 'r', '[]', 'sd', 'pending', ?2)",
                rusqlite::params![format!("nod-tok-{}", i), i as i64],
            )
            .unwrap();
        }
        let page1 = query_approval_rows_no_digest(&conn, None, 2).unwrap();
        assert_eq!(page1.len(), 2);
        let after_ts = page1.last().unwrap().created_at;
        let after_tok = page1.last().unwrap().approval_token.clone();
        let page2 = query_approval_rows_no_digest(&conn, Some((after_ts, &after_tok)), 2).unwrap();
        assert_eq!(page2.len(), 2);
    }
}
