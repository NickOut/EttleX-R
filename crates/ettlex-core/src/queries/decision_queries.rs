//! Decision query operations
//!
//! This module provides read-only query operations for decisions with
//! deterministic ordering, pagination, and filtering.

use crate::errors::{EttleXError, Result};
use crate::model::{Decision, DecisionLink};
use crate::ops::Store;
use crate::traversal::ept::compute_ept;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

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
/// Returns `DecisionNotFound` if the decision doesn't exist,
/// or `DecisionDeleted` if it was tombstoned.
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

/// List decisions linked to a specific EP
///
/// Returns decisions linked to the EP, ordered by (ordinal ASC, relation_kind ASC, decision_id ASC).
/// Supports filtering by status and relation kind.
///
/// # Errors
///
/// Returns `EpNotFound` if the EP doesn't exist.
pub fn ep_list_decisions(
    store: &Store,
    ep_id: &str,
    filters: &DecisionFilters,
) -> Result<Vec<Decision>> {
    // Verify EP exists
    store.get_ep(ep_id)?;

    // Get links for this EP
    let mut links: Vec<&DecisionLink> = store
        .decision_links
        .values()
        .filter(|link| link.target_kind == "ep" && link.target_id == ep_id && !link.is_tombstoned())
        .collect();

    // Sort by (ordinal ASC, relation_kind ASC, decision_id ASC)
    links.sort_by(|a, b| {
        a.ordinal
            .cmp(&b.ordinal)
            .then_with(|| a.relation_kind.cmp(&b.relation_kind))
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });

    // Filter by relation_kind if specified
    if let Some(ref relation) = filters.relation_filter {
        links.retain(|link| &link.relation_kind == relation);
    }

    // Get decisions
    let mut decisions = Vec::new();
    for link in links {
        if let Some(decision) = store.decisions.get(&link.decision_id) {
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

            decisions.push(decision.clone());
        }
    }

    Ok(decisions)
}

/// Decision context for EPT projection
///
/// Contains decisions organized by EP with ancestor inheritance information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionContext {
    /// Decisions directly attached to each EP in the EPT
    pub direct_by_ep: BTreeMap<String, Vec<Decision>>,

    /// All decisions inherited by the leaf EP (aggregated from ancestors)
    pub inherited_for_leaf: Vec<Decision>,
}

/// List decisions for EP with optional ancestor inclusion
///
/// When include_ancestors is true, traverses the refinement graph upward
/// to collect decisions from all ancestor EPs in the chain.
///
/// # Errors
///
/// Returns `EpNotFound` if the EP doesn't exist.
/// Returns `EptDuplicateMapping` if the refinement graph is ambiguous (multiple parents).
pub fn ep_list_decisions_with_ancestors(
    store: &Store,
    ep_id: &str,
    include_ancestors: bool,
    filters: &DecisionFilters,
) -> Result<Vec<Decision>> {
    if !include_ancestors {
        // Simple case: just return decisions for this EP
        return ep_list_decisions(store, ep_id, filters);
    }

    // Get the EP and its ettle
    let ep = store.get_ep(ep_id)?;
    let ettle_id = &ep.ettle_id;

    // Compute EPT to get all EPs in the refinement chain
    let ept = compute_ept(store, ettle_id, Some(ep.ordinal))?;

    // Collect decisions from all EPs in the EPT
    let mut all_decisions = Vec::new();
    let mut seen_ids = HashSet::new();

    for ep_id_in_ept in &ept {
        let ep_decisions = ep_list_decisions(store, ep_id_in_ept, filters)?;
        for decision in ep_decisions {
            if seen_ids.insert(decision.decision_id.clone()) {
                all_decisions.push(decision);
            }
        }
    }

    // Sort by created_at for deterministic ordering
    all_decisions.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });

    Ok(all_decisions)
}

/// Compute decision context for an EPT projection
///
/// Returns decisions organized by EP, with all decisions in the EPT included.
/// The direct_by_ep map uses BTreeMap for deterministic ordering.
///
/// # Errors
///
/// Returns errors if EPT computation fails (ambiguous graph, missing mappings, etc.).
pub fn ept_compute_decision_context(
    store: &Store,
    leaf_ettle_id: &str,
    leaf_ep_ordinal: Option<u32>,
    filters: &DecisionFilters,
) -> Result<DecisionContext> {
    // Compute EPT
    let ept = compute_ept(store, leaf_ettle_id, leaf_ep_ordinal)?;

    // Collect decisions for each EP in the EPT
    let mut direct_by_ep: BTreeMap<String, Vec<Decision>> = BTreeMap::new();
    let mut all_decisions_set = HashSet::new();

    for ep_id in &ept {
        let ep_decisions = ep_list_decisions(store, ep_id, filters)?;

        if !ep_decisions.is_empty() {
            // Track for deduplication
            for decision in &ep_decisions {
                all_decisions_set.insert(decision.decision_id.clone());
            }

            direct_by_ep.insert(ep_id.clone(), ep_decisions);
        }
    }

    // Collect all unique decisions for inherited_for_leaf
    let mut inherited_for_leaf = Vec::new();
    for decisions in direct_by_ep.values() {
        for decision in decisions {
            inherited_for_leaf.push(decision.clone());
        }
    }

    // Sort inherited decisions deterministically
    inherited_for_leaf.sort_by(|a, b| {
        a.created_at
            .cmp(&b.created_at)
            .then_with(|| a.decision_id.cmp(&b.decision_id))
    });

    // Deduplicate
    inherited_for_leaf.dedup_by(|a, b| a.decision_id == b.decision_id);

    Ok(DecisionContext {
        direct_by_ep,
        inherited_for_leaf,
    })
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
        .map_err(|_| EttleXError::Internal {
            message: "Invalid cursor: base64 decode failed".to_string(),
        })?;

    let cursor_str = String::from_utf8(decoded).map_err(|_| EttleXError::Internal {
        message: "Invalid cursor: UTF-8 decode failed".to_string(),
    })?;

    let parts: Vec<&str> = cursor_str.split('|').collect();
    if parts.len() != 2 {
        return Err(EttleXError::Internal {
            message: "Invalid cursor: wrong format".to_string(),
        });
    }

    let created_at_ms = parts[0].parse::<i64>().map_err(|_| EttleXError::Internal {
        message: "Invalid cursor: timestamp parse failed".to_string(),
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
