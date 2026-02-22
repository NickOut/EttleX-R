//! Snapshot persistence layer.
//!
//! This module handles the persistence of snapshot manifests to both
//! content-addressable storage (CAS) and the SQLite ledger.
//!
//! ## Responsibilities
//!
//! - Persist manifests to CAS with deterministic digests
//! - Create ledger entries in snapshots table
//! - Atomic commit of both CAS + ledger
//! - Idempotency checks (same semantic state → same snapshot ID)
//! - Optimistic concurrency via expected_head validation
//!
//! ## Non-Responsibilities
//!
//! - Manifest generation (handled by `ettlex-core`)
//! - Orchestration (handled by `ettlex-engine`)

pub mod persist;

// Re-export primary types
pub use persist::{
    commit_snapshot, persist_manifest_to_cas, SnapshotCommitResult, SnapshotOptions,
};
