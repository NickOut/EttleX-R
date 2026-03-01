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
pub mod query;

// Re-export primary types
pub use persist::{
    commit_snapshot, persist_manifest_to_cas, SnapshotCommitResult, SnapshotOptions,
};
pub use query::{
    fetch_head_snapshot, fetch_manifest_bytes_by_digest, fetch_snapshot_digests,
    fetch_snapshot_manifest_digest, fetch_snapshot_row, list_snapshot_rows, SnapshotRow,
};
