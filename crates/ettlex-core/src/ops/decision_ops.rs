//! Decision operation handlers
//!
//! This module implements CRUD operations for decisions, evidence items, and decision links.
//! All operations follow the TES (Temporal Event Sourcing) pattern and maintain
//! deterministic ordering for query operations.

use crate::errors::{EttleXError, Result};
use crate::model::{Decision, DecisionEvidenceItem, DecisionLink};
use crate::ops::store::Store;

/// Create a new decision
///
/// Creates a decision with the specified attributes. Validates required fields
/// and evidence rules. Generates UUIDv7 if decision_id is None.
///
/// # Arguments
///
/// * `store` - The store to add the decision to
/// * `decision_id` - Optional explicit ID (or None to generate)
/// * `title` - Decision title (required, non-empty)
/// * `status` - Decision status (defaults to "proposed" if None)
/// * `decision_text` - The actual decision (required, non-empty)
/// * `rationale` - Rationale for the decision (required, non-empty)
/// * `alternatives_text` - Alternatives considered (optional)
/// * `consequences_text` - Consequences of the decision (optional)
/// * `evidence_kind` - Evidence kind (none, excerpt, capture, file)
/// * `evidence_excerpt` - Portable short excerpt (optional)
/// * `evidence_capture_content` - Full capture content (optional)
/// * `evidence_file_path` - Repo-relative file path (optional)
///
/// # Errors
///
/// Returns `InvalidDecision` if title, decision_text, or rationale are empty.
/// Returns `InvalidEvidence` if evidence validation fails.
/// Returns `InvalidEvidencePath` if file path is absolute.
/// Returns `AlreadyExists` if decision_id already exists.
#[allow(clippy::too_many_arguments)]
pub fn create_decision(
    store: &mut Store,
    decision_id: Option<String>,
    title: String,
    status: Option<String>,
    decision_text: String,
    rationale: String,
    alternatives_text: Option<String>,
    consequences_text: Option<String>,
    evidence_kind: String,
    evidence_excerpt: Option<String>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<String>,
) -> Result<String> {
    // Validate required fields
    if title.trim().is_empty() {
        return Err(EttleXError::InvalidDecision {
            reason: "Title cannot be empty".to_string(),
        });
    }
    if decision_text.trim().is_empty() {
        return Err(EttleXError::InvalidDecision {
            reason: "Decision text cannot be empty".to_string(),
        });
    }
    if rationale.trim().is_empty() {
        return Err(EttleXError::InvalidDecision {
            reason: "Rationale cannot be empty".to_string(),
        });
    }

    // Validate evidence rules
    validate_evidence(
        &evidence_kind,
        &evidence_excerpt,
        &evidence_capture_content,
        &evidence_file_path,
    )?;

    // Generate or validate decision_id
    let final_decision_id = decision_id.unwrap_or_else(|| {
        // Generate UUIDv7
        uuid::Uuid::now_v7().to_string()
    });

    // Check for duplicate
    if store.decisions.contains_key(&final_decision_id) {
        return Err(EttleXError::AlreadyExists {
            entity_id: final_decision_id,
        });
    }

    // Handle evidence capture if provided
    let evidence_capture_id = if let Some(content) = evidence_capture_content {
        let capture_id = uuid::Uuid::now_v7().to_string();
        let evidence_item =
            DecisionEvidenceItem::new(capture_id.clone(), "mcp_chat_capture".to_string(), content);
        store.insert_evidence_item(evidence_item);
        Some(capture_id)
    } else {
        None
    };

    // Create decision
    let status = status.unwrap_or_else(|| "proposed".to_string());
    let decision = Decision::new(
        final_decision_id.clone(),
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

    store.insert_decision(decision);
    Ok(final_decision_id)
}

/// Validate evidence fields based on evidence_kind
fn validate_evidence(
    evidence_kind: &str,
    evidence_excerpt: &Option<String>,
    evidence_capture_content: &Option<String>,
    evidence_file_path: &Option<String>,
) -> Result<()> {
    match evidence_kind {
        "none" => {
            // No evidence fields allowed
            if evidence_excerpt.is_some()
                || evidence_capture_content.is_some()
                || evidence_file_path.is_some()
            {
                return Err(EttleXError::InvalidEvidence {
                    reason: "evidence_kind 'none' does not allow evidence fields".to_string(),
                });
            }
        }
        "excerpt" => {
            // Requires evidence_excerpt
            if evidence_excerpt.is_none() {
                return Err(EttleXError::InvalidEvidence {
                    reason: "evidence_kind 'excerpt' requires evidence_excerpt".to_string(),
                });
            }
        }
        "capture" => {
            // Requires evidence_capture_content OR evidence_excerpt
            if evidence_capture_content.is_none() && evidence_excerpt.is_none() {
                return Err(EttleXError::InvalidEvidence {
                    reason: "evidence_kind 'capture' requires evidence_capture_content or evidence_excerpt".to_string(),
                });
            }
        }
        "file" => {
            // Requires evidence_file_path (must be relative)
            if let Some(path) = evidence_file_path {
                if path.starts_with('/') || path.starts_with("\\") {
                    return Err(EttleXError::InvalidEvidencePath {
                        reason: "File path must be relative, not absolute".to_string(),
                    });
                }
            } else {
                return Err(EttleXError::InvalidEvidence {
                    reason: "evidence_kind 'file' requires evidence_file_path".to_string(),
                });
            }
        }
        _ => {
            // Allow arbitrary evidence_kind values (open set)
        }
    }
    Ok(())
}

/// Update a decision
///
/// Updates the specified fields of an existing decision. Preserves created_at,
/// updates updated_at. Nil values mean "don't change".
///
/// # Errors
///
/// Returns `DecisionNotFound` if the decision doesn't exist,
/// or `DecisionDeleted` if it was tombstoned.
#[allow(clippy::too_many_arguments)]
pub fn update_decision(
    store: &mut Store,
    decision_id: &str,
    title: Option<String>,
    status: Option<String>,
    decision_text: Option<String>,
    rationale: Option<String>,
    alternatives_text: Option<Option<String>>,
    consequences_text: Option<Option<String>>,
    evidence_kind: Option<String>,
    evidence_excerpt: Option<Option<String>>,
    evidence_capture_content: Option<String>,
    evidence_file_path: Option<Option<String>>,
) -> Result<()> {
    // Verify decision exists first
    {
        let _decision = store.get_decision(decision_id)?;
    }

    // Validate evidence if being updated
    if let Some(ref ek) = evidence_kind {
        validate_evidence(
            ek,
            &evidence_excerpt.clone().flatten(),
            &evidence_capture_content,
            &evidence_file_path.clone().flatten(),
        )?;
    }

    // Handle evidence capture if provided
    let new_capture_id = if let Some(content) = evidence_capture_content {
        let capture_id = uuid::Uuid::now_v7().to_string();
        let evidence_item =
            DecisionEvidenceItem::new(capture_id.clone(), "mcp_chat_capture".to_string(), content);
        store.insert_evidence_item(evidence_item);
        Some(capture_id)
    } else {
        None
    };

    // Now get mutable reference and update
    let decision = store.get_decision_mut(decision_id)?;

    if let Some(capture_id) = new_capture_id {
        decision.evidence_capture_id = Some(capture_id);
    }

    // Update fields
    decision.update(
        title,
        status,
        decision_text,
        rationale,
        alternatives_text,
        consequences_text,
        evidence_kind,
        evidence_excerpt,
        evidence_file_path,
    );

    Ok(())
}

/// Tombstone a decision (soft delete)
///
/// Marks a decision as deleted by setting its tombstoned_at timestamp.
/// The decision remains in storage for historical references.
///
/// # Errors
///
/// Returns `DecisionNotFound` if the decision doesn't exist,
/// or `DecisionDeleted` if it was already tombstoned.
pub fn tombstone_decision(store: &mut Store, decision_id: &str) -> Result<()> {
    let decision = store.get_decision_mut(decision_id)?;
    decision.tombstone();
    Ok(())
}

/// Attach a decision to a target (EP, Ettle, Constraint, or Decision)
///
/// Creates a link record with the specified relation kind and ordinal.
/// The ordinal determines position in deterministically ordered queries.
///
/// # Errors
///
/// Returns `DecisionNotFound` if the decision doesn't exist,
/// `DecisionTombstoned` if the decision was tombstoned,
/// `NotFound` if the target doesn't exist,
/// `InvalidTargetKind` if the target_kind is not allowed,
/// or `DuplicateDecisionLink` if the link already exists.
pub fn attach_decision_to_target(
    store: &mut Store,
    decision_id: &str,
    target_kind: String,
    target_id: String,
    relation_kind: String,
    ordinal: i32,
) -> Result<()> {
    // Verify decision exists (raw lookup to check tombstoned separately)
    let decision =
        store
            .decisions
            .get(decision_id)
            .ok_or_else(|| EttleXError::DecisionNotFound {
                decision_id: decision_id.to_string(),
            })?;

    // Check if tombstoned - return specific error for linking context
    if decision.is_tombstoned() {
        return Err(EttleXError::DecisionTombstoned {
            decision_id: decision_id.to_string(),
        });
    }

    // Validate target_kind
    match target_kind.as_str() {
        "ep" | "ettle" | "constraint" | "decision" => {}
        _ => {
            return Err(EttleXError::InvalidTargetKind {
                target_kind: target_kind.clone(),
            })
        }
    }

    // Verify target exists
    match target_kind.as_str() {
        "ep" => {
            store.get_ep(&target_id)?;
        }
        "ettle" => {
            store.get_ettle(&target_id)?;
        }
        "constraint" => {
            store.get_constraint(&target_id)?;
        }
        "decision" => {
            store.get_decision(&target_id)?;
        }
        _ => unreachable!(),
    }

    // Check if link already exists
    if store.is_decision_linked(decision_id, &target_kind, &target_id, &relation_kind) {
        return Err(EttleXError::DuplicateDecisionLink {
            decision_id: decision_id.to_string(),
            target_kind,
            target_id,
            relation_kind,
        });
    }

    // Create link
    let link = DecisionLink::new(
        decision_id.to_string(),
        target_kind,
        target_id,
        relation_kind,
        ordinal,
    );
    store.insert_decision_link(link);

    Ok(())
}

/// Detach a decision from a target
///
/// Removes the link record. The decision itself is preserved.
///
/// # Errors
///
/// Returns `DecisionLinkNotFound` if the link doesn't exist.
pub fn detach_decision_from_target(
    store: &mut Store,
    decision_id: &str,
    target_kind: &str,
    target_id: &str,
    relation_kind: &str,
) -> Result<()> {
    if !store.is_decision_linked(decision_id, target_kind, target_id, relation_kind) {
        return Err(EttleXError::DecisionLinkNotFound {
            decision_id: decision_id.to_string(),
            target_kind: target_kind.to_string(),
            target_id: target_id.to_string(),
            relation_kind: relation_kind.to_string(),
        });
    }

    store.remove_decision_link(decision_id, target_kind, target_id, relation_kind);
    Ok(())
}

/// Supersede a decision with a new decision
///
/// Creates a decision→decision link with relation_kind="supersedes".
/// Does NOT tombstone the old decision.
///
/// # Errors
///
/// Returns `DecisionNotFound` if either decision doesn't exist.
pub fn supersede_decision(
    store: &mut Store,
    old_decision_id: &str,
    new_decision_id: &str,
) -> Result<()> {
    // Verify both decisions exist
    store.get_decision(old_decision_id)?;
    store.get_decision(new_decision_id)?;

    // Create supersedes link (old → new)
    attach_decision_to_target(
        store,
        old_decision_id,
        "decision".to_string(),
        new_decision_id.to_string(),
        "supersedes".to_string(),
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::{ep_ops, ettle_ops};

    fn setup_store_with_ep() -> (Store, String, String) {
        let mut store = Store::new();

        let ettle_id = ettle_ops::create_ettle(
            &mut store,
            "Test Ettle".to_string(),
            None,
            None,
            Some("what".to_string()),
            Some("how".to_string()),
        )
        .unwrap();

        let ep_id = ep_ops::create_ep(
            &mut store,
            &ettle_id,
            1,
            false,
            "why".to_string(),
            "what".to_string(),
            "how".to_string(),
        )
        .unwrap();

        (store, ettle_id, ep_id)
    }

    #[test]
    fn test_create_decision() {
        let mut store = Store::new();

        let result = create_decision(
            &mut store,
            None,
            "Test Decision".to_string(),
            None,
            "We will do X".to_string(),
            "Because Y".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        );

        assert!(result.is_ok());
        let decision_id = result.unwrap();
        assert!(store.get_decision(&decision_id).is_ok());
    }

    #[test]
    fn test_create_decision_validates_title() {
        let mut store = Store::new();

        let result = create_decision(
            &mut store,
            None,
            "".to_string(),
            None,
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        );

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::InvalidDecision { .. })));
    }

    #[test]
    fn test_attach_decision_to_ep() {
        let (mut store, _ettle_id, ep_id) = setup_store_with_ep();

        let decision_id = create_decision(
            &mut store,
            None,
            "Test".to_string(),
            None,
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        let result = attach_decision_to_target(
            &mut store,
            &decision_id,
            "ep".to_string(),
            ep_id,
            "grounds".to_string(),
            0,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_supersede_decision() {
        let mut store = Store::new();

        let old_id = create_decision(
            &mut store,
            Some("d1".to_string()),
            "Old".to_string(),
            None,
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        let new_id = create_decision(
            &mut store,
            Some("d2".to_string()),
            "New".to_string(),
            None,
            "text".to_string(),
            "rationale".to_string(),
            None,
            None,
            "none".to_string(),
            None,
            None,
            None,
        )
        .unwrap();

        let result = supersede_decision(&mut store, &old_id, &new_id);
        assert!(result.is_ok());

        // Old decision should still exist (not tombstoned)
        assert!(store.get_decision(&old_id).is_ok());
    }
}
