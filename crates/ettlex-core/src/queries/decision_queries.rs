//! Decision query operations
//!
//! This module provides read-only query operations for decisions with
//! deterministic ordering, pagination, and filtering.
//!
//! EP-related query operations have been retired in Slice 03.

use crate::errors::{ExError, ExErrorKind, Result};
use crate::model::{Decision, DecisionLink};
use crate::ops::Store;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Decision detail with evidence summary and outgoing links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionDetail {
    /// The decision
    pub decision: Decision,

    /// Evidence summary
    pub evidence_summary: EvidenceSummary,

    /// Outgoing links from this decision
    pub outgoing_links: Vec<DecisionLink>,
}

/// Evidence summary for a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSummary {
    pub kind: String,
    pub excerpt: Option<String>,
    pub hash: String,
}

/// Filters for decision queries
#[derive(Debug, Clone, Default)]
pub struct DecisionFilters {
    /// Filter by status (e.g., "proposed", "accepted")
    pub status_filter: Option<String>,

    /// Filter by relation kind (e.g., "grounds", "constrains")
    pub relation_filter: Option<String>,

    /// Include tombstoned decisions
    pub include_tombstoned: bool,
}

/// Pagination parameters for cursor-based pagination
#[derive(Debug, Clone)]
pub struct PaginationParams {
    /// Cursor for pagination (base64 encoded)
    pub cursor: Option<String>,

    /// Maximum number of items to return
    pub limit: usize,
}

/// Paginated decision results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedDecisions {
    /// Decision items
    pub items: Vec<Decision>,

    /// Cursor for next page (if has_more is true)
    pub cursor: Option<String>,

    /// Whether there are more items
    pub has_more: bool,
}

/// Get a decision by ID with details
///
/// Returns the decision with evidence summary and outgoing links.
///
/// # Errors
///
/// Returns `NotFound` if the decision doesn't exist,
/// or `Deleted` if it was tombstoned.
pub fn decision_get(store: &Store, decision_id: &str) -> Result<DecisionDetail> {
    let decision = store.get_decision(decision_id)?;

    let evidence_summary = EvidenceSummary {
        kind: decision.evidence_kind.clone(),
        excerpt: decision.evidence_excerpt.clone(),
        hash: decision.evidence_hash.clone(),
    };

    // Get outgoing links
    let outgoing_links: Vec<DecisionLink> = store
        .decision_links
        .values()
        .filter(|link| link.decision_id == decision_id && !link.is_tombstoned())
        .cloned()
        .collect();

    Ok(DecisionDetail {
        decision: decision.clone(),
        evidence_summary,
        outgoing_links,
    })
}

/// List decisions with filters and pagination
///
/// Returns decisions ordered deterministically by (created_at ASC, decision_id ASC).
/// Supports cursor-based pagination for large result sets.
///
/// # Errors
///
/// Returns error if cursor is invalid.
pub fn decision_list(
    store: &Store,
    filters: &DecisionFilters,
    pagination: &PaginationParams,
) -> Result<PaginatedDecisions> {
    // Collect decisions into BTreeMap for deterministic ordering
    let mut decisions: BTreeMap<(i64, String), Decision> = BTreeMap::new();

    for decision in store.decisions.values() {
        // Filter tombstoned
        if !filters.include_tombstoned && decision.is_tombstoned() {
            continue;
        }

        // Filter by status
        if let Some(ref status) = filters.status_filter {
            if &decision.status != status {
                continue;
            }
        }

        // Key: (created_at_millis, decision_id) for deterministic sorting
        let key = (
            decision.created_at.timestamp_millis(),
            decision.decision_id.clone(),
        );
        decisions.insert(key, decision.clone());
    }

    // Handle cursor if provided
    let start_key = if let Some(ref cursor_str) = pagination.cursor {
        decode_cursor(cursor_str)?
    } else {
        None
    };

    // Collect items after cursor position
    let mut items = Vec::new();
    let mut found_start = start_key.is_none();

    for (key, decision) in decisions.iter() {
        if !found_start {
            if Some(key) == start_key.as_ref() {
                found_start = true;
            }
            continue;
        }

        if items.len() >= pagination.limit {
            break;
        }

        items.push(decision.clone());
    }

    // Check if there are more items
    let has_more = if items.len() == pagination.limit {
        // Check if there's at least one more item after our limit
        let last_key = items
            .last()
            .map(|d| (d.created_at.timestamp_millis(), d.decision_id.clone()));

        decisions
            .range((
                std::ops::Bound::Excluded(last_key.as_ref().unwrap()),
                std::ops::Bound::Unbounded,
            ))
            .next()
            .is_some()
    } else {
        false
    };

    // Generate cursor for next page if there are more items
    let cursor = if has_more {
        items
            .last()
            .map(|d| encode_cursor(d.created_at.timestamp_millis(), &d.decision_id))
    } else {
        None
    };

    Ok(PaginatedDecisions {
        items,
        cursor,
        has_more,
    })
}

/// Decision context — EP-era construct, retired in Slice 03.
///
/// Contains decisions organized by EP with ancestor inheritance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Decisions directly attached to each EP in the EPT
    pub direct_by_ep: BTreeMap<String, Vec<Decision>>,

    /// All decisions inherited by the leaf EP (aggregated from ancestors)
    pub inherited_for_leaf: Vec<Decision>,
}

/// Compute decision context for an EPT projection — RETIRED in Slice 03.
///
/// Returns `NotImplemented` as EPT has been retired along with the EP construct.
///
/// # Errors
/// Always returns `NotImplemented` — EPT is retired in Slice 03.
#[allow(unused_variables)]
pub fn ept_compute_decision_context(
    store: &Store,
    leaf_ettle_id: &str,
    leaf_ep_ordinal: Option<u32>,
    filters: &DecisionFilters,
) -> Result<DecisionContext> {
    Err(ExError::new(ExErrorKind::NotImplemented)
        .with_message("EPT decision context retired in Slice 03 — EP construct removed"))
}

/// Encode cursor for pagination
///
/// Cursor format: base64(created_at_ms|decision_id)
fn encode_cursor(created_at_ms: i64, decision_id: &str) -> String {
    let cursor_data = format!("{}|{}", created_at_ms, decision_id);
    base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        cursor_data.as_bytes(),
    )
}

/// Decode cursor for pagination
///
/// Returns (created_at_ms, decision_id) tuple
fn decode_cursor(cursor: &str) -> Result<Option<(i64, String)>> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor)
        .map_err(|_| {
            ExError::new(ExErrorKind::Internal).with_message("Invalid cursor: base64 decode failed")
        })?;

    let cursor_str = String::from_utf8(decoded).map_err(|_| {
        ExError::new(ExErrorKind::Internal).with_message("Invalid cursor: UTF-8 decode failed")
    })?;

    let parts: Vec<&str> = cursor_str.split('|').collect();
    if parts.len() != 2 {
        return Err(
            ExError::new(ExErrorKind::Internal).with_message("Invalid cursor: wrong format")
        );
    }

    let created_at_ms = parts[0].parse::<i64>().map_err(|_| {
        ExError::new(ExErrorKind::Internal).with_message("Invalid cursor: timestamp parse failed")
    })?;

    let decision_id = parts[1].to_string();

    Ok(Some((created_at_ms, decision_id)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_encoding() {
        let created_at_ms = 1234567890000;
        let decision_id = "d:123";

        let cursor = encode_cursor(created_at_ms, decision_id);
        let decoded = decode_cursor(&cursor).unwrap();

        assert_eq!(decoded, Some((created_at_ms, decision_id.to_string())));
    }

    #[test]
    fn test_cursor_invalid() {
        let result = decode_cursor("invalid-cursor");
        assert!(result.is_err());
    }
}
