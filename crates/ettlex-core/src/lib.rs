//! EttleX Core - Canonical in-memory semantic kernel
//!
//! This crate provides the foundational data structures and operations for EttleX,
//! including:
//! - Ettle and EP (Ettle Partition) models with full CRUD semantics
//! - Constraint models with family-agnostic design
//! - Deterministic traversal algorithms (RT/EPT)
//! - Tree validation and invariant enforcement
//! - Snapshot manifest generation with constraints envelope
//! - Rendering capabilities for Markdown export
//!
//! Phase 1 implementation - includes core domain models, constraints, and snapshot manifests.

pub mod apply;
pub mod approval_router;
pub mod candidate_resolver;
pub mod commands;
pub mod constraint_engine;
pub mod diff;
pub mod errors;
pub mod logging_facility;
pub mod model;
pub mod ops;
pub mod policy;
pub mod queries;
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
pub use policy::{AnchorPolicy, CommitPolicyHook, DenyAllCommitPolicyHook, NoopCommitPolicyHook};
