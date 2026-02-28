//! Profile query helpers and SqliteApprovalRouter.

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::{ExError, ExErrorKind};
use rusqlite::{Connection, OptionalExtension};

use crate::errors::Result;

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

/// Approval router backed by SQLite (writes to approval_requests table).
pub struct SqliteApprovalRouter<'a> {
    conn: std::cell::UnsafeCell<*mut Connection>,
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

        conn.execute(
            r#"INSERT INTO approval_requests
               (approval_token, reason_code, candidate_set_json, semantic_request_digest, status, created_at)
               VALUES (?1, ?2, ?3, ?4, 'pending', ?5)"#,
            rusqlite::params![token, reason_code, candidate_json, semantic_digest, now_ms],
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
