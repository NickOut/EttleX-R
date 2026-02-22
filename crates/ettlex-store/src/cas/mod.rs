//! Content-Addressable Storage (CAS)
//!
//! Provides:
//! - Filesystem-based CAS with atomic writes
//! - Collision detection
//! - Sharding by first 2 hex chars of digest

mod atomic;
mod fs_store;
mod sharding;

pub use fs_store::FsStore;
