//! EttleX Memory — context assembly and command delegation layer.
//!
//! This crate sits between the MCP transport layer and the engine, providing:
//! - Re-exports of the engine's public command API (for upstream crates to avoid
//!   depending on `ettlex-engine` directly).
//! - `MemoryManager` — assembles rich ettle context from store relations and groups.

pub mod memory_manager;

// Re-export the engine's commands module tree so downstream crates can
// use `ettlex_memory::commands::*` without a direct ettlex-engine dep.
pub use ettlex_engine::commands;

// Top-level re-exports for convenience
pub use ettlex_engine::commands::command::{apply_command, Command, CommandResult};
pub use ettlex_engine::commands::engine_query::{
    apply_engine_query, EngineQuery, EngineQueryResult,
};
