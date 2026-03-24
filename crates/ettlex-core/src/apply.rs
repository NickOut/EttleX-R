//! Functional-boundary apply function
//!
//! This module provides the `apply()` function, which is the canonical entry
//! point for atomic state mutations in the functional-boundary style.
//!
//! EP commands have been retired in Slice 03.

use crate::commands::Command;
use crate::errors::Result;
use crate::ops::{constraint_ops, decision_ops, ettle_ops, Store};
use crate::policy::AnchorPolicy;

/// Apply a command to a store, returning a new store state
///
/// This is the functional-boundary entry point for all EttleX operations.
/// It takes ownership of the current state, executes the command atomically,
/// and returns either a new valid state or an error.
///
/// # Errors
///
/// Returns an error if the command cannot be applied due to validation failures,
/// constraint violations, or other domain-specific errors.
pub fn apply(mut state: Store, cmd: Command, _policy: &dyn AnchorPolicy) -> Result<Store> {
    match cmd {
        Command::EttleCreate { title } => {
            ettle_ops::create_ettle(&mut state, title)?;
            Ok(state)
        }

        Command::EttleDelete { ettle_id } => {
            ettle_ops::delete_ettle(&mut state, &ettle_id)?;
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::policy::NeverAnchoredPolicy;

    #[test]
    fn test_apply_ettle_create() {
        let state = Store::new();
        let cmd = Command::EttleCreate {
            title: "Test Ettle".to_string(),
        };

        let policy = NeverAnchoredPolicy;
        let new_state = apply(state, cmd, &policy).unwrap();

        assert_eq!(new_state.list_ettles().len(), 1);
        let ettle = &new_state.list_ettles()[0];
        assert_eq!(ettle.title, "Test Ettle");
    }

    #[test]
    fn test_apply_atomic_on_error() {
        let state = Store::new();
        let cmd = Command::EttleCreate {
            title: "".to_string(), // Invalid title
        };

        let policy = NeverAnchoredPolicy;
        let result = apply(state.clone(), cmd, &policy);

        // Should fail
        assert!(result.is_err());

        // Original state should still be valid and unchanged
        assert_eq!(state.list_ettles().len(), 0);
    }

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
}
