//! EttleX Agent API — public API surface for agent consumers.
//!
//! This crate provides the public API for agent consumers of EttleX.
//! It depends only on `ettlex-memory` and does not have direct dependencies
//! on `ettlex-engine` or `ettlex-store`.
//!
//! ## Modules
//!
//! - `operations::ettle` — Ettle read/write operations
//! - `operations::relation` — Relation read/write operations
//! - `operations::group` — Group and group-member read/write operations
//! - `boundary::mapping` — Single designated boundary for error display mapping

pub mod boundary;
pub mod operations;

pub use operations::ettle::{
    agent_ettle_context, agent_ettle_create, agent_ettle_get, agent_ettle_list,
    agent_ettle_tombstone, agent_ettle_update, AgentEttleCreate, AgentEttleCreateResult,
    AgentEttleListOpts, AgentEttleTombstoneResult, AgentEttleUpdate, AgentEttleUpdateResult,
};
pub use operations::group::{
    agent_group_create, agent_group_get, agent_group_list, agent_group_member_add,
    agent_group_member_list, agent_group_member_remove, AgentGroupCreateResult,
    AgentGroupMemberAddResult, AgentGroupMemberListOpts, AgentGroupMemberRemoveResult,
};
pub use operations::relation::{
    agent_relation_create, agent_relation_get, agent_relation_list, agent_relation_tombstone,
    AgentRelationCreate, AgentRelationCreateResult, AgentRelationListOpts,
    AgentRelationTombstoneResult,
};

/// Returns a shared MemoryManager instance.
///
/// Used internally by operation modules that need to delegate to MemoryManager.
pub(crate) fn memory_manager_instance() -> ettlex_memory::memory_manager::MemoryManager {
    ettlex_memory::memory_manager::MemoryManager::new()
}
