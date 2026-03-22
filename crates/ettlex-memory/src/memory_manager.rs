//! MemoryManager — command delegation and ettle context assembly.

#![allow(clippy::result_large_err)]

use ettlex_core::approval_router::ApprovalRouter;
use ettlex_core::errors::ExError;
use ettlex_core::policy_provider::PolicyProvider;
use ettlex_store::cas::FsStore;
use ettlex_store::model::{GroupRecord, RelationListOpts, RelationRecord};
use ettlex_store::repo::SqliteRepo;
use rusqlite::Connection;

use crate::{apply_command, Command, CommandResult};

/// Rich context for an Ettle, assembled from relations and group memberships.
#[derive(Debug, Clone)]
pub struct EttleContext {
    /// The Ettle ID.
    pub ettle_id: String,
    /// WHY field from the Ettle record.
    pub why: Option<String>,
    /// WHAT field from the Ettle record.
    pub what: Option<String>,
    /// HOW field from the Ettle record.
    pub how: Option<String>,
    /// Active outgoing relations from this Ettle.
    pub relations: Vec<RelationRecord>,
    /// Active groups the Ettle is a member of.
    pub groups: Vec<GroupRecord>,
}

/// MemoryManager delegates commands to the engine and assembles ettle context.
#[derive(Debug, Default)]
pub struct MemoryManager;

impl MemoryManager {
    /// Create a new MemoryManager.
    pub fn new() -> Self {
        MemoryManager
    }

    /// Apply a command by delegating to the engine's `apply_command`.
    ///
    /// This method is a thin wrapper; all invariant enforcement lives in the engine.
    pub fn apply_command(
        &self,
        cmd: Command,
        expected_state_version: Option<u64>,
        conn: &mut Connection,
        cas: &FsStore,
        policy_provider: &dyn PolicyProvider,
        approval_router: &dyn ApprovalRouter,
    ) -> Result<(CommandResult, u64), ExError> {
        apply_command(
            cmd,
            expected_state_version,
            conn,
            cas,
            policy_provider,
            approval_router,
        )
    }

    /// Assemble a rich EttleContext for the given ettle_id.
    ///
    /// Fetches:
    /// - WHY / WHAT / HOW from the Ettle record.
    /// - All active outgoing relations where source_ettle_id = ettle_id.
    /// - All active groups the ettle is a member of.
    pub fn assemble_ettle_context(
        &self,
        ettle_id: &str,
        conn: &Connection,
    ) -> Result<EttleContext, ExError> {
        use ettlex_core::errors::ExErrorKind;

        // Get ettle record
        let record = SqliteRepo::get_ettle_record(conn, ettle_id)?.ok_or_else(|| {
            ExError::new(ExErrorKind::NotFound)
                .with_op("assemble_ettle_context")
                .with_entity_id(ettle_id)
                .with_message(format!("Ettle not found: {}", ettle_id))
        })?;

        // Get active outgoing relations
        let opts = RelationListOpts {
            source_ettle_id: Some(ettle_id.to_string()),
            target_ettle_id: None,
            relation_type: None,
            include_tombstoned: false,
        };
        let relations = SqliteRepo::list_relations(conn, &opts)?;

        // Get active groups for this ettle
        let groups = SqliteRepo::get_active_groups_for_ettle(conn, ettle_id)?;

        Ok(EttleContext {
            ettle_id: ettle_id.to_string(),
            why: if record.why.is_empty() {
                None
            } else {
                Some(record.why)
            },
            what: if record.what.is_empty() {
                None
            } else {
                Some(record.what)
            },
            how: if record.how.is_empty() {
                None
            } else {
                Some(record.how)
            },
            relations,
            groups,
        })
    }
}
