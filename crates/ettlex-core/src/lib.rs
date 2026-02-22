//! EttleX Core - Canonical in-memory semantic kernel
//!
//! This crate provides the foundational data structures and operations for EttleX,
//! including:
//! - Ettle and EP (Ettle Partition) models with full CRUD semantics
//! - Deterministic traversal algorithms (RT/EPT)
//! - Tree validation and invariant enforcement
//! - Rendering capabilities for Markdown export
//!
//! Phase 0.5 implementation - pure library with no persistence, CLI, or constraints.

pub mod apply;
pub mod commands;
pub mod errors;
pub mod logging_facility;
pub mod model;
pub mod ops;
pub mod policy;
pub mod render;
pub mod rules;
pub mod snapshot;
pub mod traversal;

// Re-export commonly used types
pub use apply::apply;
pub use commands::Command;
pub use errors::{EttleXError, ExError, ExErrorKind, Result};
pub use model::{Ep, Ettle, Metadata};
pub use ops::Store;
pub use policy::AnchorPolicy;
