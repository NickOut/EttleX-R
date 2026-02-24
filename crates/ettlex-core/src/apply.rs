//! Functional-boundary apply function
//!
//! This module provides the `apply()` function, which is the canonical entry
//! point for atomic state mutations in the functional-boundary style.
//!
//! ## Atomicity Contract
//!
//! The `apply()` function guarantees:
//! - **All-or-nothing**: Either the entire command succeeds and returns a valid
//!   new state, or it fails and the old state remains valid
//! - **No panics**: Invalid input returns typed errors
//! - **Deterministic validation**: Structural commands are validated before commit
//!
//! ## Example
//!
//! ```
//! use ettlex_core::{Store, Command, policy::NeverAnchoredPolicy, apply::apply};
//!
//! let state = Store::new();
//! let cmd = Command::EttleCreate {
//!     title: "My Ettle".to_string(),
//!     metadata: None,
//!     why: None,
//!     what: None,
//!     how: None,
//! };
//!
//! let policy = NeverAnchoredPolicy;
//! let new_state = apply(state, cmd, &policy).unwrap();
//! ```

use crate::commands::Command;
use crate::errors::{EttleXError, Result};
use crate::ops::{constraint_ops, decision_ops, ep_ops, ettle_ops, refinement_ops, Store};
use crate::policy::AnchorPolicy;
use crate::rules::validation;

/// Apply a command to a store, returning a new store state
///
/// This is the functional-boundary entry point for all EttleX operations (Phase 1).
/// It takes ownership of the current state, executes the command atomically,
/// and returns either a new valid state or an error.
///
/// # Atomicity Guarantee
///
/// If this function returns `Ok(new_state)`, the new state is guaranteed to
/// be structurally valid. If it returns `Err`, the old state (which the caller
/// still owns) remains valid and unchanged.
///
/// # Arguments
///
/// * `state` - Current store state (ownership transferred)
/// * `cmd` - Command to execute
/// * `policy` - Anchor policy for deletion behavior
///
/// # Returns
///
/// * `Ok(Store)` - New valid state after successful command execution
/// * `Err(EttleXError)` - Typed error, old state remains valid
///
/// # Errors
///
/// Returns an error if the command cannot be applied due to validation failures,
/// constraint violations, or other domain-specific errors. See `EttleXError` for
/// the full taxonomy of possible errors.
///
/// # Example
///
/// ```
/// use ettlex_core::{Store, Command, policy::NeverAnchoredPolicy, apply::apply};
///
/// let state = Store::new();
/// let cmd = Command::EttleCreate {
///     title: "Test".to_string(),
///     metadata: None,
///     why: None,
///     what: None,
///     how: None,
/// };
///
/// let new_state = apply(state, cmd, &NeverAnchoredPolicy).unwrap();
/// assert_eq!(new_state.list_ettles().len(), 1);
/// ```
pub fn apply(mut state: Store, cmd: Command, policy: &dyn AnchorPolicy) -> Result<Store> {
    match cmd {
        Command::EttleCreate {
            title,
            metadata,
            why,
            what,
            how,
        } => {
            ettle_ops::create_ettle(&mut state, title, metadata, why, what, how)?;
            Ok(state)
        }

        Command::EttleUpdate {
            ettle_id,
            title,
            metadata,
        } => {
            ettle_ops::update_ettle(&mut state, &ettle_id, title, metadata)?;
            Ok(state)
        }

        Command::EttleDelete { ettle_id } => {
            ettle_ops::delete_ettle(&mut state, &ettle_id)?;
            Ok(state)
        }

        Command::EpCreate {
            ettle_id,
            ordinal,
            normative,
            why,
            what,
            how,
        } => {
            ep_ops::create_ep(&mut state, &ettle_id, ordinal, normative, why, what, how)?;
            Ok(state)
        }

        Command::EpUpdate {
            ep_id,
            why,
            what,
            how,
            normative,
        } => {
            ep_ops::update_ep(&mut state, &ep_id, why, what, how, normative)?;
            Ok(state)
        }

        Command::EpDelete { ep_id } => {
            // Check anchoring policy to determine deletion strategy
            if policy.is_anchored_ep(&ep_id) {
                // Anchored: use tombstone deletion (existing behavior)
                ep_ops::delete_ep(&mut state, &ep_id)?;
            } else {
                // Not anchored: use hard deletion (NEW behavior)
                hard_delete_ep(&mut state, &ep_id)?;
            }
            Ok(state)
        }

        Command::RefineLinkChild {
            parent_ep_id,
            child_ettle_id,
        } => {
            refinement_ops::link_child(&mut state, &parent_ep_id, &child_ettle_id)?;
            // Validate tree structure after linking
            validation::validate_tree(&state)?;
            Ok(state)
        }

        Command::RefineUnlinkChild { parent_ep_id } => {
            refinement_ops::unlink_child(&mut state, &parent_ep_id)?;
            // Validate tree structure after unlinking
            validation::validate_tree(&state)?;
            Ok(state)
        }

        Command::ConstraintCreate {
            constraint_id,
            family,
            kind,
            scope,
            payload_json,
        } => {
            constraint_ops::create_constraint(
                &mut state,
                constraint_id,
                family,
                kind,
                scope,
                payload_json,
            )?;
            Ok(state)
        }

        Command::ConstraintUpdate {
            constraint_id,
            payload_json,
        } => {
            constraint_ops::update_constraint(&mut state, &constraint_id, payload_json)?;
            Ok(state)
        }

        Command::ConstraintTombstone { constraint_id } => {
            constraint_ops::tombstone_constraint(&mut state, &constraint_id)?;
            Ok(state)
        }

        Command::ConstraintAttachToEp {
            ep_id,
            constraint_id,
            ordinal,
        } => {
            constraint_ops::attach_constraint_to_ep(&mut state, ep_id, constraint_id, ordinal)?;
            Ok(state)
        }

        Command::ConstraintDetachFromEp {
            ep_id,
            constraint_id,
        } => {
            constraint_ops::detach_constraint_from_ep(&mut state, &ep_id, &constraint_id)?;
            Ok(state)
        }

        Command::DecisionCreate {
            decision_id,
            title,
            status,
            decision_text,
            rationale,
            alternatives_text,
            consequences_text,
            evidence_kind,
            evidence_excerpt,
            evidence_capture_content,
            evidence_file_path,
        } => {
            decision_ops::create_decision(
                &mut state,
                decision_id,
                title,
                status,
                decision_text,
                rationale,
                alternatives_text,
                consequences_text,
                evidence_kind,
                evidence_excerpt,
                evidence_capture_content,
                evidence_file_path,
            )?;
            Ok(state)
        }

        Command::DecisionUpdate {
            decision_id,
            title,
            status,
            decision_text,
            rationale,
            alternatives_text,
            consequences_text,
            evidence_kind,
            evidence_excerpt,
            evidence_capture_content,
            evidence_file_path,
        } => {
            decision_ops::update_decision(
                &mut state,
                &decision_id,
                title,
                status,
                decision_text,
                rationale,
                alternatives_text,
                consequences_text,
                evidence_kind,
                evidence_excerpt,
                evidence_capture_content,
                evidence_file_path,
            )?;
            Ok(state)
        }

        Command::DecisionTombstone { decision_id } => {
            decision_ops::tombstone_decision(&mut state, &decision_id)?;
            Ok(state)
        }

        Command::DecisionLink {
            decision_id,
            target_kind,
            target_id,
            relation_kind,
            ordinal,
        } => {
            decision_ops::attach_decision_to_target(
                &mut state,
                &decision_id,
                target_kind,
                target_id,
                relation_kind,
                ordinal,
            )?;
            Ok(state)
        }

        Command::DecisionUnlink {
            decision_id,
            target_kind,
            target_id,
            relation_kind,
        } => {
            decision_ops::detach_decision_from_target(
                &mut state,
                &decision_id,
                &target_kind,
                &target_id,
                &relation_kind,
            )?;
            Ok(state)
        }

        Command::DecisionSupersede {
            old_decision_id,
            new_decision_id,
        } => {
            decision_ops::supersede_decision(&mut state, &old_decision_id, &new_decision_id)?;
            Ok(state)
        }
    }
}

/// Hard delete an EP (remove from storage completely)
///
/// This is an alternative to tombstone deletion for non-anchored EPs.
/// The EP is completely removed from the store and from the owning Ettle's
/// ep_ids list.
///
/// # Safety Checks
///
/// Same as tombstone delete:
/// - Cannot delete EP0 (ordinal 0)
/// - Cannot delete EP if it's the only active mapping to a child
///
/// # Arguments
///
/// * `store` - Mutable reference to the Store
/// * `ep_id` - ID of the EP to hard delete
///
/// # Errors
///
/// * `EpNotFound` - If EP doesn't exist
/// * `EpDeleted` - If EP was already deleted
/// * `CannotDeleteEp0` - If attempting to delete EP with ordinal 0
/// * `TombstoneStrandsChild` - If EP is the only active mapping to its child
/// * `DeleteReferencesMissingEpInOwningEttle` - If EP not in owning Ettle's ep_ids
fn hard_delete_ep(store: &mut Store, ep_id: &str) -> Result<()> {
    use crate::ops::active_eps;
    use chrono::Utc;

    // Get EP first (validates exists and not already deleted)
    let ep = store.get_ep(ep_id)?;

    // Safety check: Cannot delete EP0
    if ep.ordinal == 0 {
        return Err(EttleXError::CannotDeleteEp0 {
            ettle_id: ep.ettle_id.clone(),
        });
    }

    // Safety check: If EP maps to child, ensure it's not the only mapping
    if let Some(ref child_id) = ep.child_ettle_id {
        // Get parent Ettle to check other active EPs
        let parent = store.get_ettle(&ep.ettle_id)?;
        let active = active_eps(store, parent)?;

        // Count how many active EPs map to this child
        let mapping_count = active
            .iter()
            .filter(|e| e.child_ettle_id.as_deref() == Some(child_id))
            .count();

        // If this is the only mapping, deletion would strand the child
        if mapping_count == 1 {
            return Err(EttleXError::TombstoneStrandsChild {
                ep_id: ep_id.to_string(),
                child_id: child_id.clone(),
            });
        }
    }

    // Store ettle_id before removing EP
    let ettle_id = ep.ettle_id.clone();

    // Remove from EP store
    store
        .eps
        .remove(ep_id)
        .ok_or_else(|| EttleXError::EpNotFound {
            ep_id: ep_id.to_string(),
        })?;

    // Remove from owning Ettle's ep_ids
    let ettle = store.get_ettle_mut(&ettle_id)?;
    let original_len = ettle.ep_ids.len();
    ettle.ep_ids.retain(|id| id != ep_id);

    // Verify EP was actually in the ep_ids list
    if ettle.ep_ids.len() == original_len {
        return Err(EttleXError::DeleteReferencesMissingEpInOwningEttle {
            ep_id: ep_id.to_string(),
            ettle_id,
        });
    }

    ettle.updated_at = Utc::now();

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::policy::{NeverAnchoredPolicy, SelectedAnchoredPolicy};
    use std::collections::HashSet;

    #[test]
    fn test_apply_ettle_create() {
        let state = Store::new();
        let cmd = Command::EttleCreate {
            title: "Test Ettle".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        assert_eq!(new_state.list_ettles().len(), 1);
        let ettle = &new_state.list_ettles()[0];
        assert_eq!(ettle.title, "Test Ettle");
    }

    #[test]
    fn test_apply_ettle_update() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Original".to_string(), None, None, None, None)
                .unwrap();

        let cmd = Command::EttleUpdate {
            ettle_id: ettle_id.clone(),
            title: Some("Updated".to_string()),
            metadata: None,
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        let ettle = new_state.get_ettle(&ettle_id).unwrap();
        assert_eq!(ettle.title, "Updated");
    }

    #[test]
    fn test_apply_ep_create() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();

        let cmd = Command::EpCreate {
            ettle_id: ettle_id.clone(),
            ordinal: 1,
            normative: true,
            why: "Why".to_string(),
            what: "What".to_string(),
            how: "How".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        let ettle = new_state.get_ettle(&ettle_id).unwrap();
        assert_eq!(ettle.ep_ids.len(), 2); // EP0 + EP1
    }

    #[test]
    fn test_apply_ep_delete_tombstone_for_anchored() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ep_id = ep_ops::create_ep(
            &mut state,
            &ettle_id,
            1,
            true,
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();

        // Mark EP as anchored
        let mut anchored_eps = HashSet::new();
        anchored_eps.insert(ep_id.clone());
        let policy = SelectedAnchoredPolicy::with_eps(anchored_eps);

        let cmd = Command::EpDelete {
            ep_id: ep_id.clone(),
        };

        let new_state = apply(state, cmd, &policy).unwrap();

        // EP should still exist but be tombstoned
        assert!(new_state.eps.contains_key(&ep_id));
        let ep = new_state.eps.get(&ep_id).unwrap();
        assert!(ep.deleted);

        // EP should still be in ettle's ep_ids
        let ettle = new_state.get_ettle(&ettle_id).unwrap();
        assert!(ettle.ep_ids.contains(&ep_id));
    }

    #[test]
    fn test_apply_ep_delete_hard_for_non_anchored() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ep_id = ep_ops::create_ep(
            &mut state,
            &ettle_id,
            1,
            true,
            String::new(),
            String::new(),
            String::new(),
        )
        .unwrap();

        // Use policy that doesn't anchor this EP
        let policy = NeverAnchoredPolicy;

        let cmd = Command::EpDelete {
            ep_id: ep_id.clone(),
        };

        let new_state = apply(state, cmd, &policy).unwrap();

        // EP should be completely removed from store
        assert!(!new_state.eps.contains_key(&ep_id));

        // EP should be removed from ettle's ep_ids
        let ettle = new_state.get_ettle(&ettle_id).unwrap();
        assert!(!ettle.ep_ids.contains(&ep_id));
        assert_eq!(ettle.ep_ids.len(), 1); // Only EP0 remains
    }

    #[test]
    fn test_hard_delete_cannot_delete_ep0() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();

        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep0_id = ettle.ep_ids[0].clone();

        let policy = NeverAnchoredPolicy;
        let cmd = Command::EpDelete { ep_id: ep0_id };

        let result = apply(state, cmd, &policy);
        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::CannotDeleteEp0 { .. })));
    }

    #[test]
    fn test_apply_refine_link_child() {
        let mut state = Store::new();
        let parent_id =
            ettle_ops::create_ettle(&mut state, "Parent".to_string(), None, None, None, None)
                .unwrap();
        let child_id =
            ettle_ops::create_ettle(&mut state, "Child".to_string(), None, None, None, None)
                .unwrap();

        let parent = state.get_ettle(&parent_id).unwrap();
        let ep0_id = parent.ep_ids[0].clone();

        let cmd = Command::RefineLinkChild {
            parent_ep_id: ep0_id.clone(),
            child_ettle_id: child_id.clone(),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        let ep = new_state.get_ep(&ep0_id).unwrap();
        assert_eq!(ep.child_ettle_id, Some(child_id.clone()));

        let child = new_state.get_ettle(&child_id).unwrap();
        assert_eq!(child.parent_id, Some(parent_id));
    }

    #[test]
    fn test_apply_atomic_on_error() {
        let state = Store::new();
        let cmd = Command::EttleCreate {
            title: "".to_string(), // Invalid title
            metadata: None,
            why: None,
            what: None,
            how: None,
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state.clone(), cmd, &policy);

        // Should fail
        assert!(result.is_err());

        // Original state should still be valid and unchanged
        assert_eq!(state.list_ettles().len(), 0);
    }

    // ===== Constraint Command Tests =====

    #[test]
    fn test_apply_constraint_create() {
        use serde_json::json;

        let state = Store::new();
        let cmd = Command::ConstraintCreate {
            constraint_id: "c1".to_string(),
            family: "ABB".to_string(),
            kind: "Rule".to_string(),
            scope: "EP".to_string(),
            payload_json: json!({"rule": "test"}),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        assert!(new_state.constraints.contains_key("c1"));
        let constraint = new_state.get_constraint("c1").unwrap();
        assert_eq!(constraint.family, "ABB");
    }

    #[test]
    fn test_apply_constraint_update_success() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({"rule": "old"}),
        )
        .unwrap();

        let cmd = Command::ConstraintUpdate {
            constraint_id: "c1".to_string(),
            payload_json: json!({"rule": "new"}),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        let constraint = new_state.get_constraint("c1").unwrap();
        assert_eq!(constraint.payload_json["rule"], "new");
    }

    #[test]
    fn test_apply_constraint_update_nonexistent() {
        use serde_json::json;

        let state = Store::new();
        let cmd = Command::ConstraintUpdate {
            constraint_id: "nonexistent".to_string(),
            payload_json: json!({}),
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotFound { .. })
        ));
    }

    #[test]
    fn test_apply_constraint_update_deleted() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();
        constraint_ops::tombstone_constraint(&mut state, "c1").unwrap();

        let cmd = Command::ConstraintUpdate {
            constraint_id: "c1".to_string(),
            payload_json: json!({"new": "data"}),
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::ConstraintDeleted { .. })));
    }

    #[test]
    fn test_apply_constraint_tombstone_success() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();

        let cmd = Command::ConstraintTombstone {
            constraint_id: "c1".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        let constraint = new_state.constraints.get("c1").unwrap();
        assert!(constraint.is_deleted());
    }

    #[test]
    fn test_apply_constraint_tombstone_nonexistent() {
        let state = Store::new();
        let cmd = Command::ConstraintTombstone {
            constraint_id: "nonexistent".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotFound { .. })
        ));
    }

    #[test]
    fn test_apply_constraint_tombstone_already_deleted() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();
        constraint_ops::tombstone_constraint(&mut state, "c1").unwrap();

        let cmd = Command::ConstraintTombstone {
            constraint_id: "c1".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::ConstraintDeleted { .. })));
    }

    #[test]
    fn test_apply_constraint_attach_success() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();

        let cmd = Command::ConstraintAttachToEp {
            ep_id: ep_id.clone(),
            constraint_id: "c1".to_string(),
            ordinal: 0,
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        assert!(new_state.is_constraint_attached_to_ep(&ep_id, "c1"));
    }

    #[test]
    fn test_apply_constraint_attach_nonexistent_constraint() {
        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        let cmd = Command::ConstraintAttachToEp {
            ep_id,
            constraint_id: "nonexistent".to_string(),
            ordinal: 0,
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotFound { .. })
        ));
    }

    #[test]
    fn test_apply_constraint_attach_deleted_constraint() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();
        constraint_ops::tombstone_constraint(&mut state, "c1").unwrap();

        let cmd = Command::ConstraintAttachToEp {
            ep_id,
            constraint_id: "c1".to_string(),
            ordinal: 0,
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::ConstraintDeleted { .. })));
    }

    #[test]
    fn test_apply_constraint_attach_nonexistent_ep() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();

        let cmd = Command::ConstraintAttachToEp {
            ep_id: "nonexistent".to_string(),
            constraint_id: "c1".to_string(),
            ordinal: 0,
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(result, Err(EttleXError::EpNotFound { .. })));
    }

    #[test]
    fn test_apply_constraint_attach_already_attached() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();
        constraint_ops::attach_constraint_to_ep(&mut state, ep_id.clone(), "c1".to_string(), 0)
            .unwrap();

        let cmd = Command::ConstraintAttachToEp {
            ep_id,
            constraint_id: "c1".to_string(),
            ordinal: 0,
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintAlreadyAttached { .. })
        ));
    }

    #[test]
    fn test_apply_constraint_detach_success() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();
        constraint_ops::attach_constraint_to_ep(&mut state, ep_id.clone(), "c1".to_string(), 0)
            .unwrap();

        let cmd = Command::ConstraintDetachFromEp {
            ep_id: ep_id.clone(),
            constraint_id: "c1".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        assert!(!new_state.is_constraint_attached_to_ep(&ep_id, "c1"));
    }

    #[test]
    fn test_apply_constraint_detach_not_attached() {
        use crate::ops::constraint_ops;
        use serde_json::json;

        let mut state = Store::new();
        let ettle_id =
            ettle_ops::create_ettle(&mut state, "Test".to_string(), None, None, None, None)
                .unwrap();
        let ettle = state.get_ettle(&ettle_id).unwrap();
        let ep_id = ettle.ep_ids[0].clone();

        constraint_ops::create_constraint(
            &mut state,
            "c1".to_string(),
            "ABB".to_string(),
            "Rule".to_string(),
            "EP".to_string(),
            json!({}),
        )
        .unwrap();

        let cmd = Command::ConstraintDetachFromEp {
            ep_id,
            constraint_id: "c1".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state, cmd, &policy);

        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EttleXError::ConstraintNotAttached { .. })
        ));
    }
}
