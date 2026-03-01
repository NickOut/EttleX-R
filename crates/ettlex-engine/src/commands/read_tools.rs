//! Result types and pagination helpers for the read-only query surface.
//!
//! This module defines the data structures returned by `apply_engine_query` for all
//! entity read, list, and compute queries. All types are plain data containers with
//! no I/O or mutation.

use ettlex_core::model::{Decision, Ep, Ettle};
use std::collections::BTreeMap;

/// Default maximum items per paginated list query.
pub const DEFAULT_LIST_LIMIT: usize = 100;

// ---------------------------------------------------------------------------
// Pagination
// ---------------------------------------------------------------------------

/// Options controlling a paginated list query.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Maximum number of items to return (defaults to `DEFAULT_LIST_LIMIT`).
    pub limit: Option<usize>,
    /// Opaque cursor from a previous response (base64-encoded sort key).
    pub cursor: Option<String>,
    /// If set, only return items whose ID starts with this prefix.
    pub prefix_filter: Option<String>,
    /// If set, only return items whose title contains this substring.
    pub title_contains: Option<String>,
}

impl ListOptions {
    /// Effective limit — `limit.unwrap_or(DEFAULT_LIST_LIMIT)`.
    pub fn effective_limit(&self) -> usize {
        self.limit.unwrap_or(DEFAULT_LIST_LIMIT)
    }

    /// Decode the cursor to an after-key string.
    pub fn decode_cursor(&self) -> Option<String> {
        self.cursor.as_deref().and_then(|c| base64_decode(c).ok())
    }
}

/// A paginated page of results.
#[derive(Debug, Clone)]
pub struct Page<T> {
    /// Items in this page.
    pub items: Vec<T>,
    /// Opaque cursor for the next page; `None` when this is the last page.
    pub cursor: Option<String>,
    /// Whether more items may exist after this page.
    pub has_more: bool,
}

impl<T> Page<T> {
    /// Build a page from a raw over-fetched slice.
    ///
    /// `raw` should contain `limit + 1` items at most. If `raw.len() > limit`,
    /// the extra item is dropped and `has_more` is set to `true`.
    pub fn from_overshot(mut raw: Vec<T>, limit: usize, cursor_fn: impl Fn(&T) -> String) -> Self {
        let has_more = raw.len() > limit;
        if has_more {
            raw.truncate(limit);
        }
        let cursor = if has_more {
            raw.last().map(|item| base64_encode(&cursor_fn(item)))
        } else {
            None
        };
        Page {
            items: raw,
            cursor,
            has_more,
        }
    }
}

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

/// Result of a `StateGetVersion` query.
#[derive(Debug, Clone)]
pub struct StateVersionResult {
    /// Current schema migration version number (row count in `schema_version`).
    pub state_version: u64,
    /// Manifest digest of the most recent committed snapshot, if any.
    pub semantic_head_digest: Option<String>,
}

// ---------------------------------------------------------------------------
// Ettle / EP
// ---------------------------------------------------------------------------

/// Result of an `EttleGet` query.
#[derive(Debug, Clone)]
pub struct EttleGetResult {
    /// The ettle entity.
    pub ettle: Ettle,
    /// IDs of EPs belonging to this ettle, ordered by ordinal.
    pub ep_ids: Vec<String>,
}

// ---------------------------------------------------------------------------
// Snapshot / Manifest
// ---------------------------------------------------------------------------

/// Result of a `ManifestGet*` query.
#[derive(Debug, Clone)]
pub struct ManifestGetResult {
    /// Snapshot identifier.
    pub snapshot_id: String,
    /// Full manifest digest (CAS key).
    pub manifest_digest: String,
    /// Semantic manifest digest (excludes `created_at`).
    pub semantic_manifest_digest: String,
    /// Raw manifest bytes from CAS.
    pub manifest_bytes: Vec<u8>,
}

// ---------------------------------------------------------------------------
// EPT
// ---------------------------------------------------------------------------

/// Result of an `EptCompute` query.
#[derive(Debug, Clone)]
pub struct EptComputeResult {
    /// The leaf EP ID used to anchor the EPT.
    pub leaf_ep_id: String,
    /// Ordered list of EP IDs in the EPT (root → leaf).
    pub ept_ep_ids: Vec<String>,
    /// SHA-256 digest of EP IDs joined with `\n`.
    pub ept_digest: String,
}

// ---------------------------------------------------------------------------
// Profile
// ---------------------------------------------------------------------------

/// Result of a `ProfileGet` query.
#[derive(Debug, Clone)]
pub struct ProfileGetResult {
    /// Profile reference string.
    pub profile_ref: String,
    /// SHA-256 digest of the raw `payload_json` bytes.
    pub profile_digest: String,
    /// Parsed profile payload.
    pub payload_json: serde_json::Value,
}

/// Result of a `ProfileResolve` query.
#[derive(Debug, Clone)]
pub struct ProfileResolveResult {
    /// Resolved profile reference string.
    pub profile_ref: String,
    /// SHA-256 digest of the raw `payload_json` bytes.
    pub profile_digest: String,
    /// Parsed profile payload.
    pub parsed_profile: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Approval
// ---------------------------------------------------------------------------

/// Result of an `ApprovalGet` query.
#[derive(Debug, Clone)]
pub struct ApprovalGetResult {
    /// Approval token (UUIDv7).
    pub approval_token: String,
    /// CAS digest of the full request payload blob.
    pub request_digest: String,
    /// Deterministic semantic digest over `reason_code` + sorted candidates.
    pub semantic_request_digest: String,
    /// Full parsed request payload from CAS.
    pub payload_json: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Decision context
// ---------------------------------------------------------------------------

/// Result of an `EptComputeDecisionContext` query.
#[derive(Debug, Clone)]
pub struct DecisionContextResult {
    /// Decisions grouped by EP ID.
    pub by_ep: BTreeMap<String, Vec<Decision>>,
    /// All decisions for the leaf EP (union across all EPs in the EPT).
    pub all_for_leaf: Vec<Decision>,
}

// ---------------------------------------------------------------------------
// Constraint predicate preview
// ---------------------------------------------------------------------------

/// Status of a constraint predicate preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewStatus {
    /// A single candidate was selected.
    Selected,
    /// No candidates matched.
    NoMatch,
    /// Multiple candidates and policy would route for approval.
    Ambiguous,
    /// Approval routing is configured (would be routed to approval).
    RoutedForApproval,
}

/// Result of a `ConstraintPredicatesPreview` query.
#[derive(Debug, Clone)]
pub struct PredicatePreviewResult {
    /// Resolution status.
    pub status: PreviewStatus,
    /// Selected candidate ID (only set when `status == Selected`).
    pub selected: Option<String>,
    /// All candidate IDs.
    pub candidates: Vec<String>,
}

// ---------------------------------------------------------------------------
// EP list helper types
// ---------------------------------------------------------------------------

/// A page of `Ep` items.
pub type EpPage = Page<Ep>;

/// A page of `Ettle` items.
pub type EttlePage = Page<Ettle>;

/// A page of `Decision` items.
pub type DecisionPage = Page<Decision>;

/// A page of `ProfileGetResult` items.
pub type ProfilePage = Page<ProfileGetResult>;

/// A page of `ApprovalGetResult` items — used for list queries.
#[derive(Debug, Clone)]
pub struct ApprovalListItem {
    /// Approval token.
    pub approval_token: String,
    /// Reason code.
    pub reason_code: String,
    /// Semantic digest.
    pub semantic_request_digest: String,
    /// Status.
    pub status: String,
    /// Creation timestamp (ms).
    pub created_at: i64,
    /// CAS digest (may be None for pre-007 rows).
    pub request_digest: Option<String>,
}

pub type ApprovalPage = Page<ApprovalListItem>;

// ---------------------------------------------------------------------------
// Snapshot row result
// ---------------------------------------------------------------------------

/// A snapshot ledger row returned from list/get queries.
#[derive(Debug, Clone)]
pub struct SnapshotGetResult {
    pub snapshot_id: String,
    pub root_ettle_id: String,
    pub manifest_digest: String,
    pub semantic_manifest_digest: String,
    pub created_at: i64,
    pub parent_snapshot_id: Option<String>,
    pub policy_ref: String,
    pub profile_ref: String,
    pub status: String,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Base64-encode a string using the standard alphabet with padding.
pub fn base64_encode(s: &str) -> String {
    let bytes = s.as_bytes();
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = Vec::with_capacity((bytes.len() + 2) / 3 * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = if chunk.len() > 1 {
            chunk[1] as usize
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            chunk[2] as usize
        } else {
            0
        };
        out.push(TABLE[(b0 >> 2) & 0x3f]);
        out.push(TABLE[((b0 << 4) | (b1 >> 4)) & 0x3f]);
        if chunk.len() > 1 {
            out.push(TABLE[((b1 << 2) | (b2 >> 6)) & 0x3f]);
        } else {
            out.push(b'=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[b2 & 0x3f]);
        } else {
            out.push(b'=');
        }
    }
    String::from_utf8(out).unwrap_or_default()
}

/// Base64-decode a string (standard or URL-safe, with or without padding).
pub fn base64_decode(s: &str) -> Result<String, String> {
    const TABLE: [u8; 128] = {
        let mut t = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < chars.len() {
            t[chars[i] as usize] = i as u8;
            i += 1;
        }
        // URL-safe variants
        t[b'-' as usize] = 62;
        t[b'_' as usize] = 63;
        t
    };

    let mut padded: Vec<u8> = s.as_bytes().to_vec();
    while padded.len() % 4 != 0 {
        padded.push(b'=');
    }

    let mut out = Vec::new();
    for chunk in padded.chunks(4) {
        if chunk.len() < 4 {
            break;
        }
        let get = |b: u8| -> u8 {
            if (b as usize) < 128 {
                TABLE[b as usize]
            } else {
                0xff
            }
        };
        let c0 = get(chunk[0]);
        let c1 = get(chunk[1]);
        let c2 = get(chunk[2]);
        let c3 = get(chunk[3]);
        if c0 == 0xff || c1 == 0xff {
            break;
        }
        out.push((c0 << 2) | (c1 >> 4));
        if chunk[2] != b'=' && c2 != 0xff {
            out.push((c1 << 4) | (c2 >> 2));
        }
        if chunk[3] != b'=' && c3 != 0xff {
            out.push((c2 << 6) | c3);
        }
    }
    String::from_utf8(out).map_err(|e| e.to_string())
}
