//! Command types representing all EttleX operations
//!
//! This module defines a command inventory that serves as the entry point for
//! functional-boundary operations via the `apply()` function.
//!
//! EP commands have been retired in Slice 03.

use serde_json::Value as JsonValue;

/// Command enum representing all EttleX operations
///
/// Commands are processed by the `apply()` function, which takes ownership of
/// the current state, executes the command, and returns a new valid state.
///
/// EP-related commands (EpCreate, EpUpdate, EpDelete, RefineLinkChild,
/// RefineUnlinkChild, ConstraintAttachToEp, ConstraintDetachFromEp) have
/// been retired in Slice 03 along with the EP construct.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Create a new Ettle
    EttleCreate { title: String },

    /// Delete an Ettle
    EttleDelete { ettle_id: String },

    /// Create a new constraint
    ConstraintCreate {
        constraint_id: String,
        family: String,
        kind: String,
        scope: String,
        payload_json: JsonValue,
    },

    /// Update a constraint's payload
    ConstraintUpdate {
        constraint_id: String,
        payload_json: JsonValue,
    },

    /// Tombstone a constraint (soft delete)
    ConstraintTombstone { constraint_id: String },

    /// Create a new decision with evidence
    DecisionCreate {
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
    },

    /// Update a decision's fields
    DecisionUpdate {
        decision_id: String,
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
    },

    /// Tombstone a decision (soft delete)
    DecisionTombstone { decision_id: String },

    /// Link a decision to a target (Ettle/Constraint/Decision)
    DecisionLink {
        decision_id: String,
        target_kind: String,
        target_id: String,
        relation_kind: String,
        ordinal: i32,
    },

    /// Unlink a decision from a target
    DecisionUnlink {
        decision_id: String,
        target_kind: String,
        target_id: String,
        relation_kind: String,
    },

    /// Mark one decision as superseding another
    DecisionSupersede {
        old_decision_id: String,
        new_decision_id: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_ettle_create() {
        let cmd = Command::EttleCreate {
            title: "Test".to_string(),
        };

        match cmd {
            Command::EttleCreate { title } => {
                assert_eq!(title, "Test");
            }
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_command_clone() {
        let cmd1 = Command::EttleDelete {
            ettle_id: "e1".to_string(),
        };

        let cmd2 = cmd1.clone();
        assert_eq!(cmd1, cmd2);
    }
}
