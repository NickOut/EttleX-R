//! Snapshot commit domain logic.
//!
//! This module provides manifest generation and digest computation for
//! creating immutable semantic anchors in the EttleX ledger.
//!
//! ## Responsibilities
//!
//! - Generate canonical snapshot manifests from EPT state
//! - Compute deterministic digests (EPT, manifest, semantic)
//! - Define snapshot manifest schema
//!
//! ## Non-Responsibilities
//!
//! - Persistence (handled by `ettlex-store`)
//! - Orchestration (handled by `ettlex-engine`)

pub mod digest;
pub mod manifest;

// Re-export primary types
pub use digest::{compute_ept_digest, compute_manifest_digest, compute_semantic_digest};
pub use manifest::{generate_manifest, EpEntry, SnapshotManifest};
