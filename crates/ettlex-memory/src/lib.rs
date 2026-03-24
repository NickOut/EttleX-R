//! EttleX Memory — context assembly and command delegation layer.
//!
//! This crate sits between the MCP transport layer and the engine, providing:
//! - Re-exports of the engine's public command API (for upstream crates to avoid
//!   depending on `ettlex-engine` directly).
//! - `MemoryManager` — assembles rich ettle context from store relations and groups.

pub mod macros;
pub mod memory_manager;

// Re-export the engine's commands module tree so downstream crates can
// use `ettlex_memory::commands::*` without a direct ettlex-engine dep.
pub use ettlex_engine::commands;

// Top-level re-exports for convenience
pub use ettlex_engine::commands::command::{apply_command, Command, CommandResult};
pub use ettlex_engine::commands::engine_query::{
    apply_engine_query, EngineQuery, EngineQueryResult,
};

// Re-exports required by ettlex-agent-api (SC-49: only workspace dep is ettlex-memory).
// All types used in agent-api public signatures must flow through these re-exports.
pub use ettlex_core::approval_router::{ApprovalRouter, NoopApprovalRouter};
pub use ettlex_core::errors::{ExError, ExErrorKind};
pub use ettlex_core::policy_provider::{NoopPolicyProvider, PolicyProvider};
pub use ettlex_logging::{init_test_capture, CapturedEvent, TestCapture};
// Note: log_op_start!, log_op_end!, log_op_error! are provided as #[macro_export] macros
// defined in src/macros.rs. They inline tracing calls and require only tracing as a dep.
pub use ettlex_store::cas::FsStore;
pub use ettlex_store::migrations;
pub use ettlex_store::model::{
    EttleCursor, EttleListItem, EttleListOpts, EttleListPage, EttleRecord, GroupMemberRecord,
    GroupRecord, RelationListOpts, RelationRecord,
};
pub use ettlex_store::repo::SqliteRepo;
pub use memory_manager::EttleContext;
pub use rusqlite::Connection;
