//! Command types representing all Phase 0.5 operations
//!
//! This module defines a command inventory that serves as the entry point for
//! functional-boundary operations via the `apply()` function.

use crate::model::Metadata;

/// Command enum representing all Phase 0.5 operations
///
/// Commands are processed by the `apply()` function, which takes ownership of
/// the current state, executes the command, and returns a new valid state.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Create a new Ettle with metadata and optional EP0 content
    EttleCreate {
        title: String,
        metadata: Option<Metadata>,
        why: Option<String>,
        what: Option<String>,
        how: Option<String>,
    },

    /// Update an Ettle's title and/or metadata
    EttleUpdate {
        ettle_id: String,
        title: Option<String>,
        metadata: Option<Metadata>,
    },

    /// Delete an Ettle (tombstone only in Phase 0.5)
    EttleDelete { ettle_id: String },

    /// Create a new EP with specified ordinal and content
    EpCreate {
        ettle_id: String,
        ordinal: u32,
        normative: bool,
        why: String,
        what: String,
        how: String,
    },

    /// Update EP content fields and/or normative flag
    EpUpdate {
        ep_id: String,
        why: Option<String>,
        what: Option<String>,
        how: Option<String>,
        normative: Option<bool>,
    },

    /// Delete an EP (policy-gated: hard delete or tombstone)
    EpDelete { ep_id: String },

    /// Link a child Ettle to a parent EP
    RefineLinkChild {
        parent_ep_id: String,
        child_ettle_id: String,
    },

    /// Unlink a child Ettle from a parent EP
    RefineUnlinkChild { parent_ep_id: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_ettle_create() {
        let cmd = Command::EttleCreate {
            title: "Test".to_string(),
            metadata: None,
            why: None,
            what: None,
            how: None,
        };

        match cmd {
            Command::EttleCreate { title, .. } => {
                assert_eq!(title, "Test");
            }
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_command_ep_delete() {
        let cmd = Command::EpDelete {
            ep_id: "ep-123".to_string(),
        };

        match cmd {
            Command::EpDelete { ep_id } => {
                assert_eq!(ep_id, "ep-123");
            }
            _ => panic!("Wrong command variant"),
        }
    }

    #[test]
    fn test_command_clone() {
        let cmd1 = Command::EttleUpdate {
            ettle_id: "e1".to_string(),
            title: Some("Updated".to_string()),
            metadata: None,
        };

        let cmd2 = cmd1.clone();
        assert_eq!(cmd1, cmd2);
    }
}
